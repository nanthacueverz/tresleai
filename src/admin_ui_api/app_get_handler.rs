/*
 * Created Date:   Feb 23, 2024
 * -----
 * Copyright (c) 2024 Tresle.ai or its affiliates. All Rights Reserved.
 */
//! This module contains the GET handler for fetching an app from DocumentDB.
//! The handler is used by the admin UI to fetch an app by its name.
//! The handler is mounted at `/api/v1.1/admin/apps/{app_name}`.
//! The handler is called by the admin UI to fetch an app by its name.
//! The handler returns the app document if it exists, else returns an error message.
//! The handler returns a 200 status code if the app is fetched successfully.
//! The handler returns a 404 status code if the app is not found.
//! The handler returns a 500 status code if an error occurs while fetching the app.
//! The handler returns a JSON response with the status and message.
//!

use crate::service::state::AppState;
use api_utils::errors::error_interceptor::ErrorInterceptor;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use logging_utils::create_ref_id_helper::create_ref_id;
use logging_utils::create_task_id_helper::create_task_id;
use logging_utils::create_task_ref_id_helper::create_task_ref_collection;
use mongodb::bson::doc;
use serde_json::json;
use std::sync::Arc;
use tracing::{debug, error, info, instrument};

/// GET handler to get an app.
#[utoipa::path(
    get,
    path = "/api/v1.1/admin/apps/{app_name}",
    responses(
        (status = 200, description = "App retrieved succesfully."),
        (status = StatusCode::BAD_REQUEST, description = "Invalid Request", body = [ErrorResponse]),
        (status = StatusCode::INTERNAL_SERVER_ERROR, description = "Tresle error occurred. Please save reference id: {} and contact support.")
    )
)]
#[instrument(skip_all)]
pub async fn get_app(
    Path(app_name): Path<String>,
    State(app_state): State<Arc<AppState>>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    let filter = doc! {"app_name": &app_name};
    let collection_name = &app_state.app_settings.mongo_db.mongo_db_app_collection;

    match app_state
        .db
        .get_document(collection_name, filter)
        .await
        .map_err(ErrorInterceptor::from)
    {
        Ok(Some(app)) => {
            let success_message = format!("{} retrieved successfully.", app_name);
            info!(app_name = app_name, message = success_message);
            Ok(Json(
                json!({"status": "success", "message": success_message, "data": app}),
            ))
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
            let error_message = format!("Failed to retrieve app '{}'. Error: {}", app_name, e);
            let ref_id = create_ref_id();
            let service_type = "GetApp".to_string();
            let task_id = create_task_id(&app_name, service_type);
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

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::runtime::Runtime;

    #[test]
    fn test_success_get_app() {
        let rt = Runtime::new().unwrap();

        rt.block_on(async {
            // Create a dev AppState and app_name
            let app_state = crate::tests::test_get_appstate().await.unwrap();
            let app_name = "app100".to_string();

            // Call the function
            let result = get_app(Path(app_name), State(app_state)).await;

            // Check if the function returns Ok
            assert!(result.is_ok());
        });
    }

    #[test]
    fn test_failure_get_app_no_app_found() {
        let rt = Runtime::new().unwrap();

        rt.block_on(async {
            // Create a dev AppState and app_name
            let app_state = crate::tests::test_get_appstate().await.unwrap();
            let app_name = "non_existent_app".to_string();

            // Call the function
            let result = get_app(Path(app_name), State(app_state.clone())).await;

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
    #[ignore = "until get_document returns an error"]
    fn test_failure_get_app() {
        let rt = Runtime::new().unwrap();

        rt.block_on(async {
            // Create a dev AppState and app_name
            let app_state = crate::tests::test_get_appstate().await.unwrap();
            let app_name = "app100".to_string();

            // Call the function
            let result = get_app(Path(app_name), State(app_state.clone())).await;

            // If the function returns Err, check the status code and message
            let (status_code, Json(message)) = result.err().unwrap();
            assert_eq!(status_code, StatusCode::INTERNAL_SERVER_ERROR);
            assert_eq!(message.get("status").unwrap().as_str().unwrap(), "error");
            assert!(message
                .get("message")
                .unwrap()
                .as_str()
                .unwrap()
                .contains("Failed to retrieve app "));
        });
    }
}
