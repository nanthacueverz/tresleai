/*
*  Created Date:  Mar 17, 2024
 * -----
 * Copyright (c) 2024 Tresle.ai or its affiliates. All Rights Reserved.
 */
//! This module contains the function to fetch api_key and app_id from DocumentDB corresponding to an input app_name
//! during the app update process.
//! The function is used by the onboarding service to fetch api_key and app_id from DocumentDB.
//! The function returns the api_key, api_key_id and app_id if the api_key and app_id are fetched successfully.
//! The function returns a 404 status code if the app document is not found.
//! The function returns a 500 status code if an error occurs while fetching the api_key and app_id.
//! The function returns a JSON response with the status and message.
//!

use crate::service::state::AppState;
use api_utils::errors::error_interceptor::ErrorInterceptor;
use axum::{http::StatusCode, Json};
use mongodb::bson::doc;
use std::sync::Arc;
use tracing::{error, info, instrument};

/// Asynchronous function to fetch api_key and app_id from DocumentDB.
#[instrument(skip_all)]
pub async fn fetch_api_key(
    app_state: &Arc<AppState>,
    app_name: &String,
) -> Result<(String, String, String), (StatusCode, Json<serde_json::Value>)> {
    let filter = doc! {"app_name": app_name};
    let collection_name = &app_state.app_settings.mongo_db.mongo_db_app_collection;

    match app_state
        .db
        .get_document(collection_name, filter)
        .await
        .map_err(ErrorInterceptor::from)
    {
        Ok(Some(response)) => {
            if let (Some(api_key), Some(api_key_id), Some(app_id)) = (
                response.get("api_key").and_then(|api_key| api_key.as_str()),
                response
                    .get("api_key_id")
                    .and_then(|api_key_id| api_key_id.as_str()),
                response.get("app_id").and_then(|app_id| app_id.as_str()),
            ) {
                let success_message =
                    "Api_key, api_key_id and app_id fetched successfully for given app_name."
                        .to_string();
                info!(app_name = app_name, message = success_message);
                Ok((
                    api_key.to_string(),
                    api_key_id.to_string(),
                    app_id.to_string(),
                ))
            } else {
                let error_message = "Failed to fetch api_key and/or api_key_id and/or app_id. No such key(s) found in document.".to_string();
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
                "Failed to fetch api_key, api_key_id and app_id from DocumentDB. Error: {}",
                e
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

    // #[test]
    // fn test_success_fetch_api_key() {
    //     let rt = Runtime::new().unwrap();

    //     rt.block_on(async {
    //         // Create a dev AppState
    //         let app_state = crate::tests::test_get_appstate().await.unwrap();

    //         // Call the function
    //         let result = fetch_api_key(&app_state, &"app100".to_string()).await;

    //         // If the function returns Ok, check the api_key
    //         assert!(result.is_ok());
    //         let (api_key, _api_key_id, _app_id) = result.unwrap();
    //         assert_eq!(api_key, "uXsdqeiMNIDNBrRU71kI520ovleoOC3CXBTw30d0");
    //         // assert_eq!(app_id, "test");
    //     });
    // }

    #[test]
    fn test_failure_fetch_api_key_no_document_found() {
        let rt = Runtime::new().unwrap();

        rt.block_on(async {
            // Create a dev AppState
            let app_state = crate::tests::test_get_appstate().await.unwrap();

            // Call the function
            let result = fetch_api_key(&app_state, &"non_existent_app_name".to_string()).await;

            // No document found for the given app_name. unwrap_err() returns the contained error.
            assert_eq!(result.unwrap_err().0, StatusCode::NOT_FOUND);
        });
    }
}
