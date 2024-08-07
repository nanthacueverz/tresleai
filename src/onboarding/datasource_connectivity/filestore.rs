/*
 * Created Date:  Mar 8, 2024
 * -----
 * Copyright (c) 2024 Tresle.ai or its affiliates. All Rights Reserved.
 */

//! This module contains the functions to check the connectivity to the filestore URLs concurrently. For s3, it checks the
//! bucket and object connectivity (wildcard and non-wildcard) and generates the data for sending to Kafka.
//! It returns the connectivity check failures, if any.
//!

use crate::onboarding::schema::app_onboarding_request::{AppDataSource, FileStore};
use crate::service::state::AppState;
use aws_config::meta::region::RegionProviderChain;
use aws_config::{BehaviorVersion, Region};
use axum::{http::StatusCode, Json};
use futures::stream::StreamExt;
use percent_encoding::percent_decode_str;
use std::sync::Arc;
use tracing::{debug, error, info, instrument};
use url::Url;

#[instrument(skip_all)]
/// Function to check the connectivity to the filestore URLs concurrently. Returns a vector of strings representing
/// the connectivity check failures, if any.
pub async fn filestore_check_connectivity(
    data_source: &str,
    app_state: &Arc<AppState>,
    app_datasource: &AppDataSource,
) -> Result<Vec<String>, (StatusCode, Json<serde_json::Value>)> {
    // Get the URLs for a particular 'filestore' data source.
    let data = filestore_get_data(data_source, app_datasource);
    let mut s3_urls = Vec::new();
    for s3 in data {
        s3_urls.push(s3.url.clone());
    }
    info!("Checking connectivity for: {:?}", s3_urls);

    // Instantiating S3 client. If more data sources are added to 'filestore' in future, may need to create new client for each.
    let s3_client = create_s3_client(None).await;

    // Process the URLs concurrently using a buffer_unordered stream.
    // Process the URLs concurrently using a buffer_unordered stream.
    let connectivity_errors = futures::stream::iter(
        s3_urls
            .into_iter()
            .map(|s3_url| process_url(s3_client.clone(), app_state, s3_url)),
    )
    .buffer_unordered(app_state.app_settings.aws_s3.max_concurrent_requests)
    .filter_map(|result| async move {
        match &result {
            Some(e) => error!("{}", e),
            None => debug!("No connectivity errors found"),
        }
        result
    })
    .collect::<Vec<_>>()
    .await;
    Ok(connectivity_errors)
}

/// Create an S3 client with the specified region. If region is not provided, it uses the default region.
async fn create_s3_client(region_str: Option<String>) -> Arc<aws_sdk_s3::Client> {
    let region_provider = match region_str {
        Some(region) => RegionProviderChain::first_try(Region::new(region)),
        None => RegionProviderChain::default_provider(),
    };

    let s3_config = aws_config::defaults(BehaviorVersion::latest())
        .region(region_provider)
        .load()
        .await;
    Arc::new(aws_sdk_s3::Client::new(&s3_config))
}

/// Function to process each S3 URL. Returns connectivity check failure as a string, if any.
async fn process_url(
    s3_client: Arc<aws_sdk_s3::Client>,
    app_state: &Arc<AppState>,
    s3_url: String,
) -> Option<String> {
    // URL encode the s3_url string
    let encoded_url = s3_url.replace(' ', "%20");
    info!("Processing S3 URL: '{}'", encoded_url);
    let parsed_url = match Url::parse(&encoded_url) {
        Ok(url) => url,
        Err(e) => {
            let url_result = format!("Error: Failed to parse S3 URL '{}': {}\n", encoded_url, e);
            debug!("{}", url_result);
            return Some(url_result);
        }
    };

    let bucket = if let Some(host) = parsed_url.host_str() {
        host
    } else {
        let bucket_parse_result = format!("Failed to get bucket name from S3 URL '{}'", s3_url);
        debug!("{}", bucket_parse_result);
        return Some(bucket_parse_result);
    };

    // URL decode the object key
    let object = percent_decode_str(parsed_url.path().trim_start_matches('/'))
        .decode_utf8_lossy()
        .into_owned();

    info!(
        "Checking connectivity for bucket: '{}' and object: '{}'",
        bucket, object
    );

    // Check the connectivity by fetching region of S3 bucket
    match s3_client.get_bucket_location().bucket(bucket).send().await {
        Ok(response) => {
            let bucket_result = format!("Successfully connected to S3 bucket: {}", bucket);
            debug!("{}", bucket_result);

            // Extract the region from the response
            let region = match &response.location_constraint {
                Some(region) if !region.to_string().is_empty() => region.to_string(),
                _ => "us-east-1".to_string(),
            };

            // Create a new S3 client if the region of the bucket is different from the region of the first S3 client
            let s3_client = if s3_client
                .config()
                .region()
                .unwrap_or(&Region::new("us-east-1"))
                != &Region::new(region.clone())
            {
                create_s3_client(Some(region)).await
            } else {
                s3_client
            };

            if object.contains('*') {
                handle_wildcard_object(s3_client, app_state, s3_url, bucket, &object).await
            } else {
                handle_non_wildcard_object(s3_client, app_state, s3_url, bucket, &object).await
            }
        }
        Err(e) => {
            let bucket_result = format!(
                "Error: Failed to connect to S3 bucket '{}' in URL '{}': {}\n",
                bucket, s3_url, e
            );
            debug!("{}", bucket_result);
            Some(bucket_result)
        }
    }
}

async fn handle_wildcard_object(
    s3_client: Arc<aws_sdk_s3::Client>,
    app_state: &Arc<AppState>,
    s3_url: String,
    bucket: &str,
    object: &str,
) -> Option<String> {
    let parts: Vec<&str> = object.split('*').collect();
    let folder = parts.first().unwrap_or(&"");
    let extension = parts.get(1).unwrap_or(&"").trim_start_matches('.');

    // If folder is not empty, check if the path up to the wildcard exists in the bucket
    if !folder.is_empty() {
        let response = s3_client
            .list_objects_v2()
            .bucket(bucket)
            .prefix(*folder)
            .send()
            .await;
        match response {
            Ok(output) => {
                println!("output: {:?}", output);
                if output.contents.unwrap_or_else(Vec::new).is_empty() {
                    return Some(format!(
                        "Error: Path '{}' does not exist in bucket '{}'",
                        folder, bucket
                    ));
                }
            }
            Err(e) => {
                return Some(format!(
                    "Error: Failed to list objects in bucket '{}': {}",
                    bucket, e
                ));
            }
        }
    }

    let supported_file_types: Vec<&str> = app_state
        .app_settings
        .supported_file_types
        .image
        .iter()
        .chain(app_state.app_settings.supported_file_types.text.iter())
        .map(|file_type| file_type.as_str())
        .collect();

    // Check any unsupported file type in url of the form s3://bucket/*.ext, s3://bucket/folder/*.ext, s3://bucket/folder/subfolder/*.ext, etc.
    // We are not checking any unsupported file types existing under s3://bucket/*, s3://bucket/folder/*, s3://bucket/folder/subfolder/*, etc.
    if !extension.is_empty() && !supported_file_types.contains(&extension) {
        Some(format!(
            "Error: Unsupported file extension(s) found in URL '{}': .{}",
            s3_url, extension
        ))
    } else {
        None
    }
}

/// Function to handle non-wildcard object. Returns connectivity check failure as a string, if any.
async fn handle_non_wildcard_object(
    s3_client: Arc<aws_sdk_s3::Client>,
    app_state: &Arc<AppState>,
    s3_url: String,
    bucket: &str,
    object: &str,
) -> Option<String> {
    let extension = object.split('.').last().unwrap_or("");
    let supported_file_types: Vec<&str> = app_state
        .app_settings
        .supported_file_types
        .image
        .iter()
        .chain(app_state.app_settings.supported_file_types.text.iter())
        .map(|file_type| file_type.as_str())
        .collect();

    // Check if the extension is supported
    if !extension.is_empty() && !supported_file_types.contains(&extension) {
        return Some(format!(
            "Error: Unsupported file extension(s) found in URL '{}': .{}",
            s3_url, extension
        ));
    }

    // Check the connectivity to the S3 object
    match s3_client
        .get_object()
        .bucket(bucket)
        .key(object)
        .send()
        .await
    {
        Ok(_) => {
            let object_result = format!(
                "Successfully accessed '{}' in bucket '{}'\n",
                object, bucket
            );
            debug!("{}", object_result);
            None
        }
        Err(e) => {
            let object_result = format!(
                "Error: Failed to access '{}' in bucket '{}' in URL '{}': {}\n",
                object, bucket, s3_url, e
            );
            debug!("{}", object_result);
            Some(object_result)
        }
    }
}

/// Function to collect URLS for a particular 'filestore' data source. These details will be sent to kafka.
pub fn filestore_get_data(data_source: &str, app_datasource: &AppDataSource) -> Vec<FileStore> {
    let mut result = Vec::new();
    let filestore = &app_datasource.filestore;
    if let Some(filestore_data) = filestore.get(data_source) {
        for data in filestore_data {
            result.push(data.clone());
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::test_get_appstate;
    use std::error::Error;
    use tokio::runtime::Runtime;
    use tracing_test::traced_test;

    async fn test_get_s3_client() -> Result<Arc<aws_sdk_s3::Client>, Box<dyn Error>> {
        let region_provider = RegionProviderChain::default_provider();
        let s3_config = aws_config::defaults(BehaviorVersion::latest())
            .region(region_provider)
            .load()
            .await;

        let s3_client = aws_sdk_s3::Client::new(&s3_config);
        return Ok(Arc::new(s3_client));
    }

    #[test]
    #[traced_test]
    /// Positive test case for filestore_check_connectivity
    fn test_success_filestore_check_connectivity() {
        let rt = Runtime::new().unwrap();

        rt.block_on(async {
            let app_state = test_get_appstate().await.unwrap();
            let app_data_source_json = r#"
            {
                "filestore": {
                "s3": [
                    {
                    "url": "s3://tresleai-dev-unittest/2020-Laboratory-Procedures-508.pdf",
                     "hints": [ ]
                    } ]
                },
                 "datastore": {}
            }"#;
            let app_data_source: AppDataSource =
                serde_json::from_str(app_data_source_json).unwrap();

            let result = filestore_check_connectivity("s3", &app_state, &app_data_source).await;
            for res in result.clone().unwrap() {
                assert!(!res.contains("Error"));
            }
            assert!(result.is_ok())
        });
    }

    #[test]
    #[traced_test]
    /// Positive test case for filestore_check_connectivity
    fn test_success_filestore_with_space_check_connectivity() {
        let rt = Runtime::new().unwrap();

        rt.block_on(async {
            let app_state = test_get_appstate().await.unwrap();
            let app_data_source_json = r#"
            {
                "filestore": {
                "s3": [
                    {
                    "url": "s3://tresleai-dev-unittest/2020-Laboratory-Procedures- space -508.pdf",
                     "hints": [ ]
                    } ]
                },
                 "datastore": {}
            }"#;
            let app_data_source: AppDataSource =
                serde_json::from_str(app_data_source_json).unwrap();

            let result = filestore_check_connectivity("s3", &app_state, &app_data_source).await;
            for res in result.clone().unwrap() {
                info!("res: {}", res);
                assert!(!res.contains("Error"));
            }
            assert!(result.is_ok())
        });
    }

    #[test]
    fn test_success_process_url() {
        let rt = Runtime::new().unwrap();

        rt.block_on(async {
            let app_state = test_get_appstate().await.unwrap();
            let s3_client = test_get_s3_client().await.unwrap();
            let s3_url =
                "s3://tresleai-dev-unittest/2021-Laboratory-Procedures-508.pdf".to_string();
            let result = process_url(s3_client, &app_state, s3_url).await;

            assert!(result.is_none())
        });
    }

    #[test]
    /// Positive test case for handle_wildcard_object
    fn test_success_handle_wildcard_object() {
        let rt = Runtime::new().unwrap();

        rt.block_on(async {
            let s3_client = test_get_s3_client().await.unwrap();
            let app_state = test_get_appstate().await.unwrap();
            let bucket = "tresleai-dev-unittest";
            let s3_url = "s3://tresleai-dev-unittest/*.pdf".to_string();
            let object = "*.pdf";
            let result =
                handle_wildcard_object(s3_client, &app_state, s3_url, bucket, object).await;

            assert!(result.is_none())
        });
    }

    #[test]
    /// failed test case for handle_wildcard_object file tyoe not found
    fn test_failed_handle_wildcard_object_miss_file() {
        let rt = Runtime::new().unwrap();

        rt.block_on(async {
            let s3_client = test_get_s3_client().await.unwrap();
            let app_state = test_get_appstate().await.unwrap();
            let bucket = "tresleai-dev-unittest";
            let s3_url = "s3://tresleai-dev-unittest/*.xxx".to_string();
            let object = "*.xxx";
            let result =
                handle_wildcard_object(s3_client, &app_state, s3_url, bucket, object).await;

            assert!(result.is_some())
        });
    }

    #[test]
    /// Positive test case for handle_non_wildcard_object
    fn test_success_handle_non_wildcard_object() {
        let rt = Runtime::new().unwrap();

        rt.block_on(async {
            let app_state = test_get_appstate().await.unwrap();
            let s3_client = test_get_s3_client().await.unwrap();
            let s3_url =
                "s3://tresleai-dev-unittest/2020-Laboratory-Procedures-508.pdf".to_string();
            let bucket = "tresleai-dev-unittest";
            let object = "2021-Laboratory-Procedures-508.pdf";
            let result =
                handle_non_wildcard_object(s3_client, &app_state, s3_url, bucket, object).await;

            assert!(result.is_none())
        });
    }

    #[test]
    fn test_failed_handle_non_wildcard_object_file_missing() {
        let rt = Runtime::new().unwrap();

        rt.block_on(async {
            let app_state = test_get_appstate().await.unwrap();
            let s3_client = test_get_s3_client().await.unwrap();
            let s3_url = "s3://tresleai-dev-unittest/FileNotFound.pdf".to_string();
            let bucket = "tresleai-dev-unittest";
            let object = "FileNotFound.pdf";
            let result =
                handle_non_wildcard_object(s3_client, &app_state, s3_url, bucket, object).await;

            assert!(result.is_some())
        });
    }

    #[test]
    /// Positive test case for filestore_get_data
    fn test_success_filestore_get_data() {
        let app_data_source_json = "\
        {\
            \"filestore\": {\
            \"s3\": [\
                {\
                \"url\": \"s3://badfoldername/tresleai-dev-unittest/2020-Laboratory-Procedures-508.pdf\",\
                 \"hints\": [ ]\
                } ]\
            },\
             \"datastore\": {}\
        }";
        let app_data_source: AppDataSource = serde_json::from_str(app_data_source_json).unwrap();

        let result = filestore_get_data("s3", &app_data_source);

        assert_eq!(result.len(), 1);
    }
}
