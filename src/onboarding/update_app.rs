/*
*  Created Date:  Mar 17, 2024
 * -----
 * Copyright (c) 2024 Tresle.ai or its affiliates. All Rights Reserved.
 */
//! This module contains the function to update an app.
//! The function is used by the onboarding service to update an app.
//! The function returns a 404 status code if the app document is not found.
//! The function returns a 500 status code if an error occurs while updating the app.
//! The function returns a JSON response with the status and message.
//!

use crate::admin_ui_api::schema::UpdateResponse;
use crate::onboarding::schema::app_onboarding_request::OnboardingRequest;
use crate::service::generate_and_insert_document::generate_app_document;
use crate::service::state::AppState;
use api_utils::errors::error_interceptor::ErrorInterceptor;
use axum::{http::StatusCode, Json};
use mongodb::bson::doc;
use mongodb::bson::to_bson;
use serde_json::json;
use std::sync::Arc;
use tracing::{error, info, instrument};

/// Asynchronous function to update an app.
#[instrument(skip_all)]
pub async fn update_app(
    app_state: &Arc<AppState>,
    body: &OnboardingRequest,
    app_id: String,
    api_key: String,
    api_key_id: String,
    has_datasource_changed: bool,
) -> Result<(), (StatusCode, Json<serde_json::Value>)> {
    let app_name = &body.app_name;
    let filter = doc! {"app_name": app_name};
    let collection_name = &app_state.app_settings.mongo_db.mongo_db_app_collection;

    // Create an updated app document for the given app_name
    let updated_document = match generate_app_document(
        app_state,
        body.clone(),
        app_id,
        api_key,
        api_key_id,
        has_datasource_changed,
    )
    .await
    {
        Ok(app_document) => app_document,
        Err(e) => {
            let error_message = format!("Failed to generate updated app document. Error: {}", e);
            error!(
                app_name = app_name,
                ext_message = error_message,
                message = error_message
            );
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"status": "error", "message": error_message})),
            ));
        }
    };

    // TODO: Move to a common function
    // Convert the updated document to BSON (the format required by DocumentDB)
    let app_bson = match to_bson(&updated_document) {
        Ok(bson) => {
            if let Some(document) = bson.as_document() {
                document.clone()
            } else {
                let error_message = "Failed to convert BSON to updated document.";
                error!(
                    app_name = app_name,
                    ext_message = error_message,
                    message = error_message
                );
                return Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({"status": "error","message": error_message})),
                ));
            }
        }
        Err(e) => {
            let error_message =
                format!("Failed to serialize updated document to BSON. Error: {}", e);
            error!(
                app_name = app_name,
                ext_message = error_message,
                message = error_message
            );
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"status": "error","message": error_message})),
            ));
        }
    };

    match app_state
        .db
        .update_document(collection_name, filter, app_bson)
        .await
        .map_err(ErrorInterceptor::from)
    {
        Ok(json_result) => {
            let result: UpdateResponse = match serde_json::from_value(json_result) {
                Ok(result) => result,
                Err(e) => {
                    let error_message =
                        format!("Failed to deserialize update response. Error: {}", e);
                    error!(
                        app_name = app_name,
                        ext_message = error_message,
                        message = error_message
                    );
                    return Err((
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(json!({"status": "error", "message": error_message})),
                    ));
                }
            };
            if result.modifiedCount == 0 {
                let error_message = format!("No app found with name '{}'.", &body.app_name);
                error!(ext_message = error_message, message = error_message);
                return Err((
                    StatusCode::NOT_FOUND,
                    Json(json!({"status": "error", "message": error_message})),
                ));
            } else {
                let success_message = format!("App '{}' updated successfully.", &body.app_name);
                info!(app_name = app_name, message = success_message);
                return Ok(());
            }
        }
        Err(e) => {
            let error_message = format!("Failed to update app '{}'. Error: {}", app_name, e);
            error!(
                app_name = app_name,
                ext_message = error_message,
                message = error_message
            );
            Err(e.intercept_error().await)
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
    fn test_success_update_app() {
        let rt = Runtime::new().unwrap();

        rt.block_on(async {
            // Create a dev AppState
            let app_state = crate::tests::test_get_appstate().await.unwrap();
            let app_id = "facade-update-test-DO_NOT_DELETE".to_string();
            let api_key = "wKgusQLXfH25SNyTTzWDM1Cn8yAiWNuE5mf9Whog".to_string();
            let api_key_id = "wja9ouvh7g".to_string();
            let has_datasource_changed = false;

            // Create a dev app_config
            let mut file = File::open("src/test/app_config2.json").unwrap();
            let mut buff = String::new();
            file.read_to_string(&mut buff).unwrap();
            let body: OnboardingRequest = serde_json::from_str(&buff).unwrap();

            // Call the function
            let result = update_app(
                &app_state,
                &body,
                app_id,
                api_key,
                api_key_id,
                has_datasource_changed,
            )
            .await;

            // Check if the function returns Ok
            assert!(result.is_ok());
        });
    }

    #[test]
    fn test_failure_update_app_bad_app() {
        let rt = Runtime::new().unwrap();

        rt.block_on(async {
            // Create a dev AppState
            let app_state = crate::tests::test_get_appstate().await.unwrap();
            let app_id = "facade-app_NOT_HERE".to_string();
            let api_key = "abcdefghijklmnopr]qrstuvwxz".to_string();
            let api_key_id = "abcdefghijklmnopr]qrstuvwxz".to_string();
            let has_datasource_changed = false;
            // Create a dev app_config
            let mut file = File::open("src/test/update_request.json").unwrap();
            let mut buff = String::new();
            file.read_to_string(&mut buff).unwrap();
            let body: OnboardingRequest = serde_json::from_str(&buff).unwrap();

            // Call the function
            let result = update_app(
                &app_state,
                &body,
                app_id,
                api_key,
                api_key_id,
                has_datasource_changed,
            )
            .await;

            // If the function returns Err, check the status code and message
            let (status_code, Json(message)) = result.unwrap_err();
            // let status_message = message.get("message").unwrap().as_str().unwrap();
            assert_eq!(status_code, StatusCode::NOT_FOUND);
            assert_eq!(message.get("status").unwrap().as_str().unwrap(), "error");
            assert!(message
                .get("message")
                .unwrap()
                .as_str()
                .unwrap()
                .contains("No app found with name"));
        });
    }
}
