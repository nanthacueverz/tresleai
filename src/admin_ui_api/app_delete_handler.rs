/*
 * Created Date:   Feb 23, 2024
 * -----
 * Copyright (c) 2024 Tresle.ai or its affiliates. All Rights Reserved.
 */
//! This module contains the DELETE handler for deleting an app from DocumentDB and other associated resources.
//! The handler also deletes the API key for the app and notifies Kafka about the app deletion.
//! The handler also deletes the collections associated with the app.
//! It is instrumented to capture traces using tracing.
//!

use crate::admin_ui_api::schema::DeleteResponse;
use crate::onboarding::schema::app_onboarding_request::FileStore;
use crate::service::publish_to_kafka::app_deletion_notify_kafka;
use crate::service::state::AppState;
use api_utils::errors::error_interceptor::ErrorInterceptor;
use aws_config::meta::region::RegionProviderChain;
use aws_config::{BehaviorVersion, Region};
use axum::{extract::Path, extract::State, http::StatusCode, response::IntoResponse, Json};
use chrono::Utc;
use logging_utils::create_ref_id_helper::create_ref_id;
use logging_utils::create_task_id_helper::create_task_id;
use logging_utils::create_task_ref_id_helper::create_task_ref_collection;
use mongodb::bson::{doc, from_bson};
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, error, info, instrument};

const COLLECTION_SUFFIXES_TO_DELETE: [&str; 8] = [
    "audit-microservices",
    "general",
    "error",
    "history",
    "logs",
    "metric",
    "multimodal",
    "text",
];

/// DELETE handler to delete an app and other associated resources.
#[utoipa::path(
    delete,
    path = "/api/v1.1/admin/apps/{app_name}",
    responses(
        (status = 200, description = "App deleted succesfully."),
        (status = StatusCode::BAD_REQUEST, description = "Invalid Request", body = [ErrorResponse]),
        (status = StatusCode::INTERNAL_SERVER_ERROR, description = "Tresle error occurred. Please save reference id: {} and contact support.")
    )
)]
#[instrument(skip_all)]
pub async fn delete_app(
    Path(app_name): Path<String>,
    State(app_state): State<Arc<AppState>>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    let filter = doc! {"app_name": &app_name};
    let collection_name = &app_state.app_settings.mongo_db.mongo_db_app_collection;

    // Fetch the sqs_key and api_key_id for the app
    let (sqs_key, api_key_id, filestore) =
        fetch_sqs_key_api_key_id_and_filestore(&app_state, &app_name).await?;
    // Generate timestamp and a task_id for the deletion task
    let deletion_timestamp = Utc::now();
    let random_num: u32 = (rand::random::<u32>() % 90000) + 10000;
    let task_id = format!(
        "{}-{}-{}-{}-{}",
        "TSK", random_num, &app_name, "Deletion", deletion_timestamp
    );
    match app_state
        .db
        .delete_document(collection_name, filter)
        .await
        .map_err(ErrorInterceptor::from)
    {
        Ok(json_result) => {
            let result: DeleteResponse = match serde_json::from_value(json_result) {
                Ok(result) => result,
                Err(e) => {
                    let error_message =
                        format!("Failed to deserialize deletion response. Error: {}", e);
                    debug!(message = error_message);
                    return Err((
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(json!({"status": "error", "message": error_message})),
                    ));
                }
            };
            // Check if the app was deleted
            if result.deletedCount == 0 {
                let error_message = format!("No app found with name '{}'.", app_name);
                debug!(message = error_message);
                Err((
                    StatusCode::NOT_FOUND,
                    Json(json!({"status": "error", "message": error_message})),
                ))
            } else {
                for suffix in COLLECTION_SUFFIXES_TO_DELETE {
                    let collection = format!("{}-{}", app_name, suffix);
                    match app_state
                        .db
                        .drop_collection(&collection)
                        .await
                        .map_err(ErrorInterceptor::from)
                    {
                        Ok(_) => {
                            let success_message =
                                format!("Collection '{}' deleted successfully.", collection);
                            debug!(message = success_message);
                        }
                        Err(e) => {
                            let error_message = format!(
                                "Failed to delete collection '{}'. Error: {}",
                                collection, e
                            );
                            debug!(message = error_message);
                        }
                    }
                }

                // Delete API key for the app
                delete_api_key(&app_state, &app_name, &api_key_id).await?;

                // Notify Kafka about app deletion. Pass it the sqs key for the app as well.
                app_deletion_notify_kafka(&app_state, &app_name, &sqs_key, &filestore, task_id)
                    .await?;

                let success_message = format!("App '{}' deleted successfully.", app_name);
                debug!(message = success_message);
                Ok(Json(
                    json!({"status": "success", "message": success_message, "app_name": app_name}),
                ))
            }
        }
        Err(e) => {
            let error_message = format!("Failed to delete app '{}'. Error: {:?}", app_name, e);
            let ref_id = create_ref_id();
            let mongo_url = app_state.app_settings.mongo_db.mongo_db_url.clone();
            let mongo_db_name = app_state
                .app_settings
                .mongo_db
                .mongo_db_database_name
                .clone();
            let id_collection = app_state
                .app_settings
                .mongo_db
                .mongo_db_id_collection
                .clone();
            let _ = create_task_ref_collection(
                mongo_url,
                mongo_db_name,
                id_collection,
                app_name.clone(),
                task_id,
                ref_id.clone(),
            )
            .await;
            let ext_message = format!(
                "{} Use reference ID: {}",
                app_state.app_settings.general_message, ref_id
            );
            error!(
                app_name = app_name,
                ext_message = ext_message,
                message = error_message
            );
            Err(e.intercept_error().await)
        }
    }
}

/// Type alias for complicated return types of 'fetch_sqs_key_api_key_id_and_filestore' function.
pub type FetchResult = (String, String, HashMap<String, Vec<FileStore>>);
pub type FetchError = (StatusCode, Json<serde_json::Value>);

/// Asynchronous function to fetch the sqs key for an app.
#[instrument(skip_all)]
pub async fn fetch_sqs_key_api_key_id_and_filestore(
    app_state: &Arc<AppState>,
    app_name: &String,
) -> Result<FetchResult, FetchError> {
    let filter = doc! {"app_name": app_name};
    let collection_name = &app_state.app_settings.mongo_db.mongo_db_app_collection;

    match app_state
        .db
        .get_document(collection_name, filter)
        .await
        .map_err(ErrorInterceptor::from)
    {
        Ok(Some(response)) => {
            if let (Some(sqs_key), Some(api_key_id), Some(filestore_bson)) = (
                response.get("sqs_key").and_then(|sqs_key| sqs_key.as_str()),
                response
                    .get("api_key_id")
                    .and_then(|api_key_id| api_key_id.as_str()),
                response
                    .get("app_datasource")
                    .and_then(|app_datasource| app_datasource.get("filestore")),
            ) {
                // Convert the Bson object to a HashMap<String, Vec<FileStore>>
                let filestore_bson = bson::to_bson(&filestore_bson).map_err(|_| {
                    let error_message = "Failed to convert filestore to Bson.".to_string();
                    debug!(message = error_message);
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(serde_json::json!({ "status": "error","message": error_message})),
                    )
                })?;

                let filestore_result: Result<HashMap<String, Vec<FileStore>>, _> =
                    from_bson(filestore_bson);

                let filestore = match filestore_result {
                    Ok(filestore) => filestore,
                    Err(_) => {
                        let error_message = "Failed to deserialize filestore.".to_string();
                        debug!(message = error_message);
                        return Err((
                            StatusCode::INTERNAL_SERVER_ERROR,
                            Json(serde_json::json!({ "status": "error","message": error_message})),
                        ));
                    }
                };

                let success_message =
                    "Sqs_key, api_key_id and filestore fetched successfully for given app_name."
                        .to_string();
                info!(app_name = app_name, message = success_message);
                Ok((
                    sqs_key.to_string(),
                    api_key_id.to_string(),
                    filestore.clone(),
                ))
            } else {
                let error_message = format!("Failed to fetch SQS key, API key id and/or filestore. No such key(s) found for app '{}'.", app_name);
                debug!(message = error_message);
                Err((
                    StatusCode::NOT_FOUND,
                    Json(json!({"status": "error", "message": error_message})),
                ))
            }
        }
        Ok(None) => {
            let error_message = format!("No app found with name '{}'.", app_name);
            debug!(message = error_message);
            Err((
                StatusCode::NOT_FOUND,
                Json(json!({"status": "error", "message": error_message})),
            ))
        }
        Err(e) => {
            let error_message = format!(
                "Failed to fetch SQS key, API key id and/or filestore for the app {}. Error: {:?}",
                app_name, e
            );
            let ref_id = create_ref_id();
            let service_type = "FetchSqsKey".to_string();
            let task_id = create_task_id(app_name, service_type);
            let mongo_url = app_state.app_settings.mongo_db.mongo_db_url.clone();
            let mongo_db_name = app_state
                .app_settings
                .mongo_db
                .mongo_db_database_name
                .clone();
            let id_collection = app_state
                .app_settings
                .mongo_db
                .mongo_db_id_collection
                .clone();
            let _ = create_task_ref_collection(
                mongo_url,
                mongo_db_name,
                id_collection,
                app_name.clone(),
                task_id.clone(),
                ref_id.clone(),
            )
            .await;
            let ext_message = format!(
                "{} Use reference ID: {}",
                app_state.app_settings.general_message, ref_id
            );
            error!(
                app_name = app_name,
                task_id = task_id,
                ext_message = ext_message,
                message = error_message
            );
            Err(e.intercept_error().await)
        }
    }
}

/// Asynchronous function to delete an API key for the app. The API key name is same as the app name.
#[instrument(skip_all)]
pub async fn delete_api_key(
    app_state: &Arc<AppState>,
    app_name: &String,
    api_key_id: &String,
) -> Result<String, (StatusCode, Json<serde_json::Value>)> {
    debug!("Deleting api key for the app.");
    let region = app_state.app_settings.aws_api_gateway.region.clone();
    let region_provider = RegionProviderChain::first_try(Region::new(region));

    let config = aws_config::defaults(BehaviorVersion::latest())
        .region(region_provider)
        .load()
        .await;
    let client = aws_sdk_apigateway::Client::new(&config);

    match client.delete_api_key().api_key(api_key_id).send().await {
        Ok(_) => {
            let success_message = format!("API key deleted successfully for the app {}.", app_name);
            debug!(message = success_message);
            Ok(success_message)
        }
        Err(e) => {
            let error_message = format!("API key deletion failed. Error: {}", e);
            let ref_id = create_ref_id();
            let service_type = "DeleteApiKey".to_string();
            let task_id = create_task_id(app_name, service_type);
            let mongo_url = app_state.app_settings.mongo_db.mongo_db_url.clone();
            let mongo_db_name = app_state
                .app_settings
                .mongo_db
                .mongo_db_database_name
                .clone();
            let id_collection = app_state
                .app_settings
                .mongo_db
                .mongo_db_id_collection
                .clone();
            let _ = create_task_ref_collection(
                mongo_url,
                mongo_db_name,
                id_collection,
                app_name.clone(),
                task_id.clone(),
                ref_id.clone(),
            )
            .await;
            let ext_message = format!(
                "{} Use reference ID: {}",
                app_state.app_settings.general_message, ref_id
            );
            error!(
                app_name = app_name,
                task_id = task_id,
                ext_message = ext_message,
                message = error_message
            );
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"status": "error","message": error_message})),
            ))
        }
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use tokio::runtime::Runtime;

    pub async fn test_success_delete_app(app_name: String) {
        // Create a dev AppState and app_name
        let app_state = crate::tests::test_get_appstate().await.unwrap();

        // Call the function
        let _result = delete_app(Path(app_name), State(app_state)).await;
    }

    #[test]
    fn test_failure_delete_app_no_app_found() {
        let rt = Runtime::new().unwrap();

        rt.block_on(async {
            // Create a dev AppState and app_name
            let app_state = crate::tests::test_get_appstate().await.unwrap();
            let app_name = "non_existent_app".to_string();

            // Call the function
            let result = delete_app(Path(app_name), State(app_state.clone())).await;

            // If the function returns Err, check the status code and message
            let (status_code, Json(message)) = result.err().unwrap();
            assert_eq!(status_code, StatusCode::NOT_FOUND);
            assert_eq!(message.get("status").unwrap().as_str().unwrap(), "error");
            assert!(message
                .get("message")
                .unwrap()
                .as_str()
                .unwrap()
                .contains("No app found with name "));
        });
    }

    #[test]
    fn test_success_fetch_sqs_key_api_key_id_and_filestore() {
        let rt = Runtime::new().unwrap();

        rt.block_on(async {
            // Create a dev AppState
            let app_state = crate::tests::test_get_appstate().await.unwrap();

            // Call the function
            let result =
                fetch_sqs_key_api_key_id_and_filestore(&app_state, &"app100".to_string()).await;

            // Check if the function returns Ok
            assert!(result.is_ok());
        });
    }

    #[test]
    fn test_failure_fetch_sqs_key_api_key_id_and_filestore_no_app_found() {
        let rt = Runtime::new().unwrap();

        rt.block_on(async {
            // Create a dev AppState
            let app_state = crate::tests::test_get_appstate().await.unwrap();

            // Call the function
            let result =
                fetch_sqs_key_api_key_id_and_filestore(&app_state, &"non_existent_app".to_string())
                    .await;

            // If the function returns Err, check the status code and message
            let (status_code, Json(message)) = result.err().unwrap();
            assert_eq!(status_code, StatusCode::NOT_FOUND);
            assert_eq!(message.get("status").unwrap().as_str().unwrap(), "error");
            assert!(message
                .get("message")
                .unwrap()
                .as_str()
                .unwrap()
                .contains("No app found with name "));
        });
    }

    #[test]
    fn test_failure_delete_api_key_wrong_app_and_api_key_id() {
        let rt = Runtime::new().unwrap();

        rt.block_on(async {
            // Create a dev AppState
            let app_state = crate::tests::test_get_appstate().await.unwrap();

            // Call the function
            let result = delete_api_key(
                &app_state,
                &"non_existent_app".to_string(),
                &"non_existent_api_key_id".to_string(),
            )
            .await;

            // If the function returns Err, check the status code and message
            let (status_code, Json(message)) = result.err().unwrap();
            assert_eq!(status_code, StatusCode::INTERNAL_SERVER_ERROR);
            assert_eq!(message.get("status").unwrap().as_str().unwrap(), "error");
            assert!(message
                .get("message")
                .unwrap()
                .as_str()
                .unwrap()
                .contains("API key deletion failed. Error: "));
        });
    }
}
