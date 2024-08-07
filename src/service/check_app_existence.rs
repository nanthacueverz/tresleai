/*
*  Created Date:  Mar 17, 2024
 * -----
 * Copyright (c) 2024 Tresle.ai or its affiliates. All Rights Reserved.
 */
//! This module contains the function to check the existence of an app in DocumentDB during the onboarding/ app update
//! process.

use crate::service::state::AppState;
use api_utils::errors::error_interceptor::ErrorInterceptor;
use axum::{http::StatusCode, Json};
use mongodb::bson::doc;
use std::sync::Arc;
use tracing::{debug, error, info, instrument};

/// Asynchronous function to check the existence of an app in DocumentDB.
#[instrument(skip_all)]
pub async fn check_app_existence(
    app_state: &Arc<AppState>,
    app_name: &String,
) -> Result<bool, (StatusCode, Json<serde_json::Value>)> {
    let filter = doc! {"app_name": app_name};
    let collection_name = &app_state.app_settings.mongo_db.mongo_db_app_collection;

    match app_state
        .db
        .get_document_count(collection_name, filter)
        .await
        .map_err(ErrorInterceptor::from)
    {
        Ok(app_count) => {
            if app_count > 0 {
                let message = format!("App {} exists in DocumentDB.", app_name);
                info!(app_name = app_name, message = message);
                Ok(true)
            } else {
                let message = format!("No app found with name '{}'.", app_name);
                debug!(message = message);
                Ok(false)
            }
        }
        Err(e) => {
            let error_message = format!(
                "Failed to check existence of app '{}' in DocumentDB. Error: {}",
                app_name, e
            );
            error!(ext_message = error_message, message = error_message);
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
    use tokio::runtime::Runtime;

    #[test]
    fn test_success_check_app_existence_true() {
        let rt = Runtime::new().unwrap();

        rt.block_on(async {
            // Create a dev AppState and app_name
            let app_state = crate::tests::test_get_appstate().await.unwrap();
            let app_name = "app100".to_string();

            // Call the function
            let result = check_app_existence(&app_state, &app_name).await;

            // Check if the function returns Ok
            assert!(result.is_ok());

            // Check if the result is true for an existing app
            assert_eq!(result.unwrap(), true)
        });
    }

    #[test]
    fn test_success_check_app_existence_false() {
        let rt = Runtime::new().unwrap();

        rt.block_on(async {
            // Create a dev AppState and app_name
            let app_state = crate::tests::test_get_appstate().await.unwrap();
            let app_name = "non-existing-app".to_string();

            // Call the function
            let result = check_app_existence(&app_state, &app_name).await;

            // Check if the function returns Ok
            assert!(result.is_ok());

            // Check if the result is false for a non-existing app
            assert_eq!(result.unwrap(), false)
        });
    }

    #[test]
    fn test_success_check_app_existence_empty_app_name() {
        let rt = Runtime::new().unwrap();

        rt.block_on(async {
            // Create a dev AppState and app_name
            let app_state = crate::tests::test_get_appstate().await.unwrap();
            let app_name = "".to_string();

            // Call the function
            let result = check_app_existence(&app_state, &app_name).await;

            // Check if the function returns Ok
            assert!(result.is_ok());

            // Check if the result is false for an empty app name
            assert_eq!(result.unwrap(), false)
        });
    }

    /* tofix unit test
    #[test]
    #[ignore="until get_document_count returns an error"]
    fn test_failure_check_app_existence() {
        let rt = Runtime::new().unwrap();

        rt.block_on(async {
            // Create a dev AppState and app_name
            let app_state = crate::tests::test_get_appstate().await.unwrap();
            let app_name = "app1".to_string();

            let app_state = Arc::new(app_state);

            // Call the function
            let result = check_app_existence(&app_state, &app_name).await;

            // If the function returns Err, check the status code and message
            let (status_code, Json(message)) = result.err().unwrap();
            assert_eq!(status_code, StatusCode::INTERNAL_SERVER_ERROR);
            assert_eq!(message.get("status").unwrap().as_str().unwrap(), "error");
            assert!(message.get("message").unwrap().as_str().unwrap().contains("Failed to check existence of app "));
        });
    }
    */
}
