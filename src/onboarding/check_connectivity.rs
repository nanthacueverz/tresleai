/*
 * Created Date:   Feb 23, 2024
 * -----
 * Copyright (c) 2024 Tresle.ai or its affiliates. All Rights Reserved.
 */

//! This module checks the connectivity to the different data sources. It calls the appropriate
//! connectivity check logic based on the data source being checked.
//! The module is used by the onboarding service to check the connectivity to the different data sources.
//! The module returns an error if the connectivity check fails, else returns a success message.
//! The module returns a 400 status code if an error occurs while checking the connectivity.
//! The module returns a 500 status code if an error occurs while checking the connectivity.
//! The module returns a JSON response with the status and message.
//!

use crate::onboarding::datasource_connectivity::checker::{
    CheckerTrait, DatastoreChecker, FilestoreChecker,
};
use crate::onboarding::schema::app_onboarding_request::AppDataSource;
use crate::onboarding::schema::response::ErrorResponse;
use crate::service::state::AppState;
use axum::{http::StatusCode, Json};
use serde_json::json;
use std::sync::Arc;
use tracing::{debug, error, info, instrument};

// Main caller to the connectivity checks for different data sources
#[instrument(skip_all)]
pub async fn check_datasource_connectivity(
    app_state: &Arc<AppState>,
    app_datasource: &AppDataSource,
    app_name: &String,
) -> Result<(), (StatusCode, Json<serde_json::Value>)> {
    debug!("Starting connectivity check for the data sources.");
    let supported_data_sources: Vec<&String> = app_state
        .app_settings
        .supported_data_source_types
        .data_store
        .iter()
        .chain(
            app_state
                .app_settings
                .supported_data_source_types
                .file_store
                .iter(),
        )
        .collect();

    let filestore_data_sources: Vec<_> = app_datasource.filestore.keys().cloned().collect();
    let datastore_data_sources: Vec<_> = app_datasource.datastore.keys().cloned().collect();
    let mut unsupported_data_sources = Vec::new();

    // Check all filestore and datastore data sources are supported by the application
    for data_source in filestore_data_sources
        .iter()
        .chain(datastore_data_sources.iter())
    {
        if !supported_data_sources.contains(&data_source) {
            unsupported_data_sources.push(data_source.to_string());
        }
    }

    // Return error if any unsupported data sources found
    if !unsupported_data_sources.is_empty() {
        let error_message = format!(
            "Unsupported data sources found: {:?}",
            unsupported_data_sources
        );
        error!(ext_message = error_message, message = error_message);

        let error_response = ErrorResponse {
            status: "error".to_string(),
            message: "Unsupported data sources. Please check and try again.".to_string(),
            errors: unsupported_data_sources,
        };

        match serde_json::to_value(error_response) {
            Ok(json_response) => {
                return Err((StatusCode::BAD_REQUEST, Json(json_response)));
            }
            Err(e) => {
                let error_message = format!("Failed to serialize error response: {}", e);
                error!(ext_message = error_message, message = error_message);
                return Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({ "error": "Internal server error" })),
                ));
            }
        }
    }

    let mut connectivity_errors = Vec::new();

    // Check the connectivity for 'filestore' and 'datastore' data sources by calling their respective checkers
    for data_source in &filestore_data_sources {
        let mut errors = FilestoreChecker
            .connectivity(data_source.as_str(), app_state, app_datasource)
            .await?;
        connectivity_errors.append(&mut errors)
    }
    for data_source in &datastore_data_sources {
        let mut errors = DatastoreChecker
            .connectivity(data_source.as_str(), app_state, app_datasource)
            .await?;
        connectivity_errors.append(&mut errors);
    }

    // Return error if any connectivity errors found
    if !connectivity_errors.is_empty() {
        let error_message = format!(
            "Connectivity check failed. Errors: {:?}",
            connectivity_errors
        );
        error!(ext_message = error_message, message = error_message);

        let error_response = ErrorResponse {
            status: "error".to_string(),
            message: "Connectivity check failed. Please check and try again.".to_string(),
            errors: connectivity_errors,
        };

        match serde_json::to_value(error_response) {
            Ok(json_response) => {
                return Err((StatusCode::BAD_REQUEST, Json(json_response)));
            }
            Err(e) => {
                let error_message = format!("Failed to serialize error response: {}", e);
                error!(ext_message = error_message, message = error_message);
                return Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({ "error": "Internal server error" })),
                ));
            }
        }
    }
    info!(app_name = app_name, "Connectivity check successful.");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Read;
    use tokio::runtime::Runtime;

    #[test]
    fn test_success_check_connectivity() {
        let rt = Runtime::new().unwrap();

        rt.block_on(async {
            // Create a dev AppState
            let app_state = crate::tests::test_get_appstate().await.unwrap();

            let mut file = File::open("src/test/app_data_source.json").unwrap();
            let mut buff = String::new();
            file.read_to_string(&mut buff).unwrap();

            let app_name = String::from("app1");
            let app_data_source: AppDataSource = serde_json::from_str(&buff).unwrap();

            // Call the function
            let result =
                check_datasource_connectivity(&app_state, &app_data_source, &app_name).await;

            // Check that the result is as expected
            assert!(result.is_ok());
        });
    }
}
