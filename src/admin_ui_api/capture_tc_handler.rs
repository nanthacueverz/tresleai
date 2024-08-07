/*
 * Created Date:   Feb 23, 2024
 * -----
 * Copyright (c) 2024 Tresle.ai or its affiliates. All Rights Reserved.
 */
//! This module contains the handler for the admin UI to capture the T & C and other user information.
//! The handler is mounted at `/api/v1.1/admin/capture_tc`.
//! The handler captures the T & C and other user information from the admin UI.
//! The handler returns a 200 status code if the user information is captured successfully.
//! The handler returns a 400 status code if the request is invalid.
//! The handler returns a 500 status code if an error occurs while fetching the app.
//! The handler returns a JSON response with the status and message.
//!

use crate::admin_ui_api::schema::{CaptureTcSchema, CaptureUserSchema};
use crate::service::state::AppState;
use axum::extract::Query;
use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use logging_utils::create_ref_id_helper::create_ref_id;
use logging_utils::create_task_id_helper::create_task_id;
use logging_utils::create_task_ref_id_helper::create_task_ref_collection;
use mongodb::bson::doc;
use serde_json::json;
use std::sync::Arc;
use tracing::{info, instrument};

/// post handler to capture the T & C and other user information from admin ui & reference ui.
#[utoipa::path(
    post,
    path = "/api/v1.1/admin/capture_tc",
    request_body = CaptureUserSchema,
    params(
        (
            "is_tc" = inline(Option<bool>),
            Query,
            description = "Check box for Accepting T & C.",
        )
    ),
    responses(
        (status = 200, description = "Captured User and T&C information successfully"),
        (status = StatusCode::BAD_REQUEST, description = "Invalid Request", body = [ErrorResponse]),
        (status = StatusCode::INTERNAL_SERVER_ERROR, description = "Tresle error occurred. Please save reference id: {} and contact support.")
    )
)]
// This method captures the T & C and other user information from the admin UI , and stores it in the database through audit microservice.
#[instrument(skip_all)]
pub async fn post_capture_tc_handler(
    Query(params): Query<CaptureTcSchema>,
    State(app_state): State<Arc<AppState>>,
    Json(body): Json<CaptureUserSchema>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    let ref_id = create_ref_id();
    let app_name = app_state.app_settings.tracing_layer_system_app_name.clone();
    let service_type = "CaptureT&C".to_string();
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
    let user_name = body.user_name;
    let ui_type = body.ui_type;
    let is_tc = params.is_tc;
    // Check if the user has accepted the T & C
    match is_tc {
        true => {
            // user has accepted the T & C
            let msg = format!("User :{} accepted the T&C from '{}' UI", user_name, ui_type);
            info!(app_name = app_name, task_id = task_id, message = msg);
            info!(
                app_name = &app_name,
                service = "audit_microservice",
                task_id = task_id,
                user_id = "tresleai",
                action = "Capture T&C information",
                details = "User accepted the T&C",
                message = msg,
            );
            let _ = create_task_ref_collection(
                mongo_url,
                mongo_db_name,
                id_collection,
                app_name.clone(),
                task_id.clone(),
                ref_id.clone(),
            )
            .await;
            Ok(Json(json!({"status": "success", "message":msg})))
        }
        false => {
            //user has not accepted the T & C
            let msg = format!(
                "User :{} did not accept the T&C from '{}' UI",
                user_name, ui_type
            );
            info!(app_name = app_name, task_id = task_id, message = msg);
            info!(
                app_name = &app_name,
                service = "audit_microservice",
                task_id = task_id,
                user_id = "tresleai",
                action = "Capture T&C information",
                details = "User accepted the T&C",
                message = msg,
            );
            let _ = create_task_ref_collection(
                mongo_url,
                mongo_db_name,
                id_collection,
                app_name.clone(),
                task_id.clone(),
                ref_id.clone(),
            )
            .await;
            Ok(Json(json!({"status": "success", "message":msg})))
        }
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::tests::*;
    use std::fs::File;
    use std::io::Read;
    use tokio::runtime::Runtime;

    #[test]
    pub fn test_success_post_capture_tc_handler_is_tc_true() {
        let rt = Runtime::new().unwrap();
        rt.block_on(async {
            // Create a dev AppState
            let app_state = crate::tests::test_get_appstate().await.unwrap(); // Note global.yaml need to point to localhost:8003

            let path = app_state.app_settings.knowledge_engine.endpoint.to_string();

            let mut mock_server = MOCK_SERVER.lock().unwrap();
            mock_server
                .mock("POST", path.as_str())
                .with_status(200)
                .with_header("content-type", "application/json")
                .with_body(
                    "{\"status\": \"ok\",
                            \"message\": \"User :Test User did not accept the T&C\"
                            }",
                )
                .create();

            // Create a mock RetrievalRequest
            let mut file = File::open("src/test/capture_request.json").unwrap();
            let mut buff = String::new();
            file.read_to_string(&mut buff).unwrap();

            let app_config: CaptureUserSchema = serde_json::from_str(&buff).unwrap();

            let mut query_params = CaptureTcSchema::default();
            query_params.is_tc = true;

            // Call the function
            let result = post_capture_tc_handler(
                Query(query_params),
                State(app_state),
                axum::Json(app_config),
            )
            .await;
            let res = result.into_response();
            // Check that the result is as expected
            assert_eq!(res.status(), StatusCode::OK);

            //assert_eq!(body_str, "Expected message");
        });
    }
    #[test]
    pub fn test_success_post_capture_tc_handler_is_tc_false() {
        let rt = Runtime::new().unwrap();
        rt.block_on(async {
            // Create a dev AppState
            let app_state = crate::tests::test_get_appstate().await.unwrap(); // Note global.yaml need to point to localhost:8003

            let path = app_state.app_settings.knowledge_engine.endpoint.to_string();

            let mut mock_server = MOCK_SERVER.lock().unwrap();
            mock_server
                .mock("POST", path.as_str())
                .with_status(200)
                .with_header("content-type", "application/json")
                .with_body(
                    "{\"status\": \"ok\",
                            \"message\": \"User :Test User did not accept the T&C\"
                            }",
                )
                .create();

            // Create a mock RetrievalRequest
            let mut file = File::open("src/test/capture_request.json").unwrap();
            let mut buff = String::new();
            file.read_to_string(&mut buff).unwrap();

            let app_config: CaptureUserSchema = serde_json::from_str(&buff).unwrap();

            let mut query_params = CaptureTcSchema::default();
            query_params.is_tc = false;

            // Call the function
            let result = post_capture_tc_handler(
                Query(query_params),
                State(app_state),
                axum::Json(app_config),
            )
            .await;
            let res = result.into_response();
            // Check that the result is as expected
            assert_eq!(res.status(), StatusCode::OK);

            //assert_eq!(body_str, "Expected message");
        });
    }
}
