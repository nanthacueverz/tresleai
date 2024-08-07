/*
 * Created Date:   Feb 23, 2024
 * -----
 * Copyright (c) 2024 Tresle.ai or its affiliates. All Rights Reserved.
 */
//! This module contains the PATCH handler for updating the search_enabled flag of an app in DocumentDB.
//! The handler is mounted at `/api/v1.1/admin/search/apps/{app_name}`.
//! The handler is called by the admin UI to update the search_enabled flag of an app by its name.
//! The handler returns a 200 status code if the search_enabled flag is updated successfully.
//! The handler returns a 400 status code if an error occurs while updating the search_enabled flag.
//! The handler returns a 500 status code if an error occurs while updating the search_enabled flag.
//! The handler returns a JSON response with the status and message.
//!

use crate::admin_ui_api::schema::{QueryParams, UpdateResponse};
use crate::service::state::AppState;
use api_utils::errors::error_interceptor::ErrorInterceptor;
use axum::{
    extract::{Path, Query, State},
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

/// PATCH handler to update the search_enabled flag of an app.
#[utoipa::path(
    patch,
    path = "/api/v1.1/admin/search/apps/{app_name}",
    params(
        (
            "search_enabled" = inline(Option<bool>), 
            Query,
            description = "search enabled flag.",
        )
    ),
    responses(
        (status = 200, description = "Search_enabled flag updated successfully."),
        (status = StatusCode::BAD_REQUEST, description = "Invalid Request", body = [ErrorResponse]),
        (status = StatusCode::INTERNAL_SERVER_ERROR, description = "Tresle error occurred. Please save reference id: {} and contact support.")
    )
)]
#[instrument(skip_all)]
pub async fn update_search_enabled_handler(
    Query(params): Query<QueryParams>,
    Path(app_name): Path<String>,
    State(app_state): State<Arc<AppState>>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    // Create a reference ID ,task ID and initialize the documentdb variables
    let ref_id = create_ref_id();
    let service_type = "UpdateSearch".to_string();
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

    let filter = doc! {"app_name": &app_name};
    let collection_name = &app_state.app_settings.mongo_db.mongo_db_app_collection;

    // Extract the search_enabled flag from the query params
    let search_enabled = params.search_enabled.unwrap_or(false);

    // Update the search_enabled flag in the app document
    let updated_document = doc! {"search_enabled": search_enabled};

    match app_state
        .db
        .update_document(collection_name, filter, updated_document)
        .await
        .map_err(ErrorInterceptor::from)
    {
        Ok(json_result) => {
            let result: UpdateResponse = match serde_json::from_value(json_result) {
                Ok(result) => result,
                Err(e) => {
                    let error_message =
                        format!("Failed to deserialize update response. Error: {:?}", e);
                    let ext_message = format!(
                        "{} Use reference ID: {}",
                        app_state.app_settings.general_message, ref_id
                    );
                    let _ = create_task_ref_collection(
                        mongo_url,
                        mongo_db_name,
                        id_collection,
                        app_name.clone(),
                        task_id.clone(),
                        ref_id,
                    )
                    .await;
                    error!(
                        app_name = app_name,
                        task_id = task_id,
                        ext_message = ext_message,
                        message = error_message
                    );
                    return Err((
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(json!({"status": "error", "message": error_message})),
                    ));
                }
            };
            // Check if the app was found
            if result.matchedCount == 0 {
                let error_message = format!("No app found with name '{}'.", app_name);
                debug!(message = error_message);
                Err((
                    StatusCode::NOT_FOUND,
                    Json(json!({"status": "error", "message": error_message})),
                ))
            } else {
                let success_message = format!(
                    "Search_enabled flag updated to '{}' successfully.",
                    search_enabled
                );
                info!(app_name = app_name, message = success_message);
                Ok(Json(
                    json!({"status": "success", "message": success_message, "app_name": app_name}),
                ))
            }
        }
        Err(e) => {
            let error_message = format!("Failed to update app '{}'. Error: {}", app_name, e);
            let ext_message = format!(
                "{} Use reference ID: {}",
                app_state.app_settings.general_message, ref_id
            );
            let _ = create_task_ref_collection(
                mongo_url,
                mongo_db_name,
                id_collection,
                app_name.clone(),
                task_id.clone(),
                ref_id,
            )
            .await;
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
    fn test_success_update_search_enabled_handler_set_to_true() {
        let rt = Runtime::new().unwrap();

        rt.block_on(async {
            // Create a dev AppState
            let app_state = crate::tests::test_get_appstate().await.unwrap();
            let app_name = "app100".to_string();

            // Call the function
            let result = update_search_enabled_handler(
                Query(QueryParams {
                    page: Some(1),
                    limit: Some(10),
                    app_name: None,
                    is_update: None,
                    search_enabled: Some(true),
                    reference_id: None,
                    knowledge_node_type: None,
                    start_timestamp: None,
                    end_timestamp: None,
                    utc_start_timestamp: None,
                    utc_end_timestamp: None,
                }),
                Path(app_name),
                State(app_state),
            )
            .await;

            // Check if the function returns Ok
            assert!(result.is_ok());
        });
    }

    #[test]
    fn test_success_update_search_enabled_handler_set_to_false() {
        let rt = Runtime::new().unwrap();

        rt.block_on(async {
            // Create a dev AppState
            let app_state = crate::tests::test_get_appstate().await.unwrap();
            let app_name = "app100".to_string();

            // Call the function
            let result = update_search_enabled_handler(
                Query(QueryParams {
                    page: Some(1),
                    limit: Some(10),
                    app_name: None,
                    is_update: None,
                    search_enabled: Some(false),
                    reference_id: None,
                    knowledge_node_type: None,
                    start_timestamp: None,
                    end_timestamp: None,
                    utc_start_timestamp: None,
                    utc_end_timestamp: None,
                }),
                Path(app_name),
                State(app_state),
            )
            .await;

            // Check if the function returns Ok
            assert!(result.is_ok());
        });
    }

    #[test]
    fn test_failure_update_search_enabled_handler_no_app_found() {
        let rt = Runtime::new().unwrap();

        rt.block_on(async {
            // Create a dev AppState
            let app_state = crate::tests::test_get_appstate().await.unwrap();
            let app_name = "non-existing-app".to_string();

            // Call the function
            let result = update_search_enabled_handler(
                Query(QueryParams {
                    page: Some(1),
                    limit: Some(10),
                    app_name: None,
                    is_update: None,
                    search_enabled: Some(true),
                    reference_id: None,
                    knowledge_node_type: None,
                    start_timestamp: None,
                    end_timestamp: None,
                    utc_start_timestamp: None,
                    utc_end_timestamp: None,
                }),
                Path(app_name),
                State(app_state),
            )
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
    fn test_success_update_search_enabled_handler_flag_not_provided() {
        let rt = Runtime::new().unwrap();

        rt.block_on(async {
            // Create a dev AppState
            let app_state = crate::tests::test_get_appstate().await.unwrap();
            let app_name = "app100".to_string();

            // Call the function
            let result = update_search_enabled_handler(
                Query(QueryParams {
                    page: Some(1),
                    limit: Some(10),
                    app_name: None,
                    is_update: None,
                    search_enabled: None,
                    reference_id: None,
                    knowledge_node_type: None,
                    start_timestamp: None,
                    end_timestamp: None,
                    utc_start_timestamp: None,
                    utc_end_timestamp: None,
                }),
                Path(app_name),
                State(app_state),
            )
            .await;

            // Check if the function returns Ok. If the search_enabled flag is not provided, it should default to false.
            assert!(result.is_ok());
        });
    }
}
