/*
*  Created Date:  Mar 17, 2024
 * -----
 * Copyright (c) 2024 Tresle.ai or its affiliates. All Rights Reserved.
 */
//! This module contains the function to check if an app's existing and new datasources are the same during
//! the app update process.
//! The function is used by the onboarding service to check if an app's existing and new datasources are the same.
//! The function returns a boolean value indicating if the datasources are the same and an optional AppDataSource
//! representing the existing datasource if the datasources are different.
//! The function returns a 404 status code if the app document is not found.
//! The function returns a 500 status code if an error occurs while fetching the existing datasource.
//! The function returns a JSON response with the status and message.
//!

use crate::onboarding::schema::app_onboarding_request::AppDataSource;
use crate::service::state::AppState;
use api_utils::errors::error_interceptor::ErrorInterceptor;
use axum::{http::StatusCode, Json};
use mongodb::bson::doc;
use std::sync::Arc;
use tracing::{error, info, instrument};

/// Asynchronous function to check if an app's existing and new datasources are the same.
#[instrument(skip_all)]
pub async fn check_datasource_change(
    app_state: &Arc<AppState>,
    app_name: &String,
    new_app_datasource: &AppDataSource,
) -> Result<(bool, Option<AppDataSource>), (StatusCode, Json<serde_json::Value>)> {
    let filter = doc! {"app_name": app_name};
    let collection_name = &app_state.app_settings.mongo_db.mongo_db_app_collection;

    match app_state
        .db
        .get_document(collection_name, filter)
        .await
        .map_err(ErrorInterceptor::from)
    {
        Ok(Some(response)) => {
            if let Some(existing_app_datasource_value) = response.get("app_datasource") {
                let existing_app_datasource: AppDataSource = serde_json::from_value(
                    existing_app_datasource_value.clone(),
                )
                .map_err(|_| {
                    let error_message = "Failed to deserialize existing datasource.".to_string();
                    error!(
                        app_name = app_name,
                        ext_message = error_message,
                        message = error_message
                    );
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(serde_json::json!({ "status": "error","message": error_message})),
                    )
                })?;

                // Check if the new datasource and existing datasource are the same
                if existing_app_datasource == *new_app_datasource {
                    let message = format!(
                        "New datasource is same as the existing datasource for app '{}'.",
                        app_name
                    );
                    info!(app_name = app_name, message = message);
                    Ok((false, None))
                } else {
                    let message = format!(
                        "New datasource is different from the existing datasource for app '{}'.",
                        app_name
                    );
                    info!(app_name = app_name, message = message);
                    Ok((true, Some(existing_app_datasource)))
                }
            } else {
                let error_message =
                    "Failed to fetch app_datasource. No such key(s) found in document.".to_string();
                error!(
                    app_name = app_name,
                    ext_message = error_message,
                    message = error_message
                );
                Err((
                    StatusCode::NOT_FOUND,
                    Json(serde_json::json!({ "status": "error","message": error_message})),
                ))
            }
        }
        Ok(None) => {
            let error_message = format!("No document found for app {}", app_name);
            error!(ext_message = error_message, message = error_message);
            Err((
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({ "status": "error","message": error_message})),
            ))
        }
        Err(e) => {
            let error_message = format!(
                "Failed to fetch existing datasource from DocumentDB. Error: {}",
                e
            );
            error!(
                app_name = app_name,
                ext_message = error_message,
                message = error_message
            );
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "status": "error","message": error_message})),
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Read;
    use tokio::runtime::Runtime;

    #[test]
    fn test_success_check_datasource_change() {
        let rt = Runtime::new().unwrap();

        rt.block_on(async {
            // Create a dev AppState
            let app_state = crate::tests::test_get_appstate().await.unwrap();

            // Create a new AppDataSource
            let mut file = File::open("src/test/new_app_data_source.json").unwrap();
            let mut buff = String::new();
            file.read_to_string(&mut buff).unwrap();

            let app_name = String::from("app100");
            let new_app_datasource: AppDataSource = serde_json::from_str(&buff).unwrap();

            // Call the function
            let result = check_datasource_change(&app_state, &app_name, &new_app_datasource).await;

            // Assert that result is Ok
            assert!(result.is_ok());
        });
    }

    #[test]
    fn test_success_check_datasource_change_true() {
        let rt = Runtime::new().unwrap();

        rt.block_on(async {
            // Create a dev AppState
            let app_state = crate::tests::test_get_appstate().await.unwrap();

            // Create a new AppDataSource
            let mut file = File::open("src/test/new_app_data_source.json").unwrap();
            let mut buff = String::new();
            file.read_to_string(&mut buff).unwrap();

            let app_name = String::from("app100");
            let new_app_datasource: AppDataSource = serde_json::from_str(&buff).unwrap();

            // Call the function
            let result = check_datasource_change(&app_state, &app_name, &new_app_datasource).await;

            // Assert that result is Ok and the datasources are different
            assert!(result.is_ok());

            let (is_changed, existing_app_datasource_option) = result.unwrap();

            let existing_app_datasource = existing_app_datasource_option.clone().unwrap();
            assert_eq!(
                (is_changed, existing_app_datasource_option),
                (true, Some(existing_app_datasource))
            );
        });
    }

    #[test]
    fn test_failure_check_datasource_change_no_document_found() {
        let rt = Runtime::new().unwrap();

        rt.block_on(async {
            // Create a dev AppState
            let app_state = crate::tests::test_get_appstate().await.unwrap();

            // Create a new AppDataSource
            let mut file = File::open("src/test/new_app_data_source.json").unwrap();
            let mut buff = String::new();
            file.read_to_string(&mut buff).unwrap();

            let app_name = String::from("non_existent_app");
            let new_app_datasource: AppDataSource = serde_json::from_str(&buff).unwrap();

            // Call the function
            let result = check_datasource_change(&app_state, &app_name, &new_app_datasource).await;

            // No document found for the given app_name. unwrap_err() returns the contained error.
            assert!(result.is_err());
            let err = result.unwrap_err();
            assert_eq!(err.0, StatusCode::NOT_FOUND);
            assert_eq!(err.1.get("status").unwrap(), "error");
            assert!(err
                .1
                .get("message")
                .unwrap()
                .as_str()
                .unwrap()
                .contains("No document found for app "));
        });
    }
}
