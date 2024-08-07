/*
 * Created Date:   Feb 23, 2024
 * -----
 * Copyright (c) 2024 Tresle.ai or its affiliates. All Rights Reserved.
 */
//! This module contains the GET handler for fetching count of knowledge nodes and errors while processing them
//! for an app between two timestamps.
//! The handler is mounted at `/api/v1.1/admin/nodes/count/{app_name}`.
//! The handler returns the count of knowledge nodes and errors if they exist, else returns an error message.
//! The handler returns a 200 status code if the count of knowledge nodes and errors is fetched successfully.
//! The handler returns a 400 status code if an error occurs while fetching the count of knowledge nodes and errors.
//! The handler returns a 500 status code if an error occurs while fetching the count of knowledge nodes and errors.
//! The handler returns a JSON response with the status and message.
//!

use crate::admin_ui_api::schema::{Counts, QueryParams};
use crate::service::check_app_existence::check_app_existence;
use crate::service::state::AppState;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use chrono::DateTime;
use logging_utils::create_ref_id_helper::create_ref_id;
use logging_utils::create_task_id_helper::create_task_id;
use logging_utils::create_task_ref_id_helper::create_task_ref_collection;
use mongodb::bson::doc;
use percent_encoding::percent_decode_str;
use serde_json::json;
use std::sync::Arc;
use tracing::{debug, error, info, instrument};

/// GET handler to fetch count of knowledge nodes and errors while processing them for an app between two timestamps.
#[utoipa::path(
    get,
    path = "/api/v1.1/admin/nodes/count/{app_name}",
    params(
        (
            "start_timestamp" = inline(String), 
            Query,
            description = "start timestamp.",
        ),
        (
            "end_timestamp" = inline(String), 
            Query,
            description = "end timestamp.",
        )
    ),
    responses(
        (status = 200, description = "Count of knowledge nodes and errors for app fetched successfully."),
        (status = StatusCode::BAD_REQUEST, description = "Invalid Request", body = [ErrorResponse]),
        (status = StatusCode::INTERNAL_SERVER_ERROR, description = "Tresle error occurred. Please save reference id: {} and contact support.")
    )
)]
#[instrument(skip_all)]
pub async fn get_knowledge_nodes_and_errors_count(
    Path(app_name): Path<String>,
    Query(params): Query<QueryParams>,
    State(app_state): State<Arc<AppState>>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    // Create a reference ID ,task ID and initialize the documentdb variables
    let ref_id = create_ref_id();
    let service_type = "GetNodeCount".to_string();
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
    //let ext_message = format!("{} Use reference ID: {}", app_state.app_settings.general_message, ref_id);
    // Check if the start timestamp is provided
    let start_timestamp_encoded = params.start_timestamp.ok_or_else(|| {
        let error_message = "start_timestamp is required.".to_string();
        error!(message = error_message);
        (
            StatusCode::BAD_REQUEST,
            Json(json!({"status": "error", "message": error_message})),
        )
    });

    let start_timestamp_encoded = match start_timestamp_encoded {
        Ok(start_timestamp) => start_timestamp,
        Err(err) => {
            let error_message = "start_timestamp is required.".to_string();
            let ext_message = "Please provide the start_timestamp.";
            let _ = create_task_ref_collection(
                mongo_url,
                mongo_db_name,
                id_collection,
                app_name.clone(),
                task_id.clone(),
                ref_id.clone(),
            )
            .await;
            error!(
                app_name = app_name,
                task_id = task_id,
                ext_message = ext_message,
                message = error_message
            );

            return Err(err);
        }
    };

    // Decode the percent-encoded start timestamp
    let start_timestamp = percent_decode_str(&start_timestamp_encoded)
        .decode_utf8_lossy()
        .to_string();

    // Check if the decoded start timestamp is valid in RFC3339 format
    match DateTime::parse_from_rfc3339(&start_timestamp) {
        Ok(_) => {}
        Err(_) => {
            let error_message = format!("Invalid start timestamp '{}'.", start_timestamp);
            let ext_message = "Please provide the valid start_timestamp in RFC3339 format.";
            let _ = create_task_ref_collection(
                mongo_url,
                mongo_db_name,
                id_collection,
                app_name.clone(),
                task_id.clone(),
                ref_id.clone(),
            )
            .await;
            error!(
                app_name = app_name,
                task_id = task_id,
                ext_message = ext_message,
                message = error_message
            );
            return Err((
                StatusCode::BAD_REQUEST,
                Json(json!({"status": "error", "message": error_message})),
            ));
        }
    };

    let end_timestamp_encoded = params.end_timestamp.ok_or_else(|| {
        let error_message = "end_timestamp is required.".to_string();
        error!(message = error_message);
        (
            StatusCode::BAD_REQUEST,
            Json(json!({"status": "error", "message": error_message})),
        )
    });
    let end_timestamp_encoded = match end_timestamp_encoded {
        Ok(end_timestamp) => end_timestamp,
        Err(err) => {
            let error_message = "end_timestamp is required.".to_string();
            let ext_message = "Please provide the end timestamp.";
            let _ = create_task_ref_collection(
                mongo_url,
                mongo_db_name,
                id_collection,
                app_name.clone(),
                task_id.clone(),
                ref_id.clone(),
            )
            .await;
            error!(
                app_name = app_name,
                task_id = task_id,
                ext_message = ext_message,
                message = error_message
            );
            return Err(err);
        }
    };

    let end_timestamp = percent_decode_str(&end_timestamp_encoded)
        .decode_utf8_lossy()
        .to_string();

    match DateTime::parse_from_rfc3339(&end_timestamp) {
        Ok(_) => {}
        Err(_) => {
            let error_message = "Error parsing rfc3339 end_timestamp.".to_string();
            error!(app_name = app_name, message = error_message);
            return Err((
                StatusCode::BAD_REQUEST,
                Json(json!({"status": "error", "message": error_message})),
            ));
        }
    };

    // Check if the app exists
    let app_exists = check_app_existence(&app_state, &app_name).await?;
    if !app_exists {
        let error_message = format!("No app found with name '{}'.", app_name);
        let ext_message = "Please provide a valid app name.";
        debug!(message = error_message);
        let _ = create_task_ref_collection(
            mongo_url,
            mongo_db_name,
            id_collection,
            app_name.clone(),
            task_id.clone(),
            ref_id.clone(),
        )
        .await;
        error!(
            app_name = app_name,
            task_id = task_id,
            ext_message = ext_message,
            message = error_message
        );
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({"status": "error", "message": error_message})),
        ));
    }

    let nodes_collection_name = format!("{}-general", app_name);
    let errors_collection_name = format!("{}-error", app_name);

    // Pipeline to get the count of knowledge nodes
    let nodes_count_pipeline = vec![
        doc! {
            "$match": {
                "indexed_at": {
                    "$gte": start_timestamp.clone(),
                    "$lte": end_timestamp.clone(),
                }
            }
        },
        doc! {
            "$group": {
                "_id": "$_node_label",
                "count": {
                    "$sum": 1
                }
            }
        },
    ];

    // Call the aggregation operation to get the count of knowledge nodes
    let nodes_result = app_state
        .db
        .aggregation_ops_on_documents(&nodes_collection_name, nodes_count_pipeline)
        .await
        .map_err(|err| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                axum::Json(json!({"status": "error", "message": err.to_string()})),
            )
        })?;

    let mut knowledge_node_file_store = 0;
    let mut knowledge_node_data_store = 0;

    // Extract the count of knowledge nodes
    for result in nodes_result {
        if let Some(label) = result.get("_id") {
            if let Some(count) = result.get("count").and_then(|c| c.as_u64()) {
                if label == "FileObject" {
                    knowledge_node_file_store = count;
                } else if label == "DatabaseObjectNode" {
                    knowledge_node_data_store = count;
                }
            }
        }
    }

    // Pipeline to get the count of errors while processing/extracting knowledge nodes
    let errors_count_pipeline = vec![
        doc! {
            "$match": {
                "event_time": {
                    "$gte": start_timestamp.clone(),
                    "$lte": end_timestamp.clone(),
                }
            }
        },
        doc! {
            "$count": "count"
        },
    ];

    // Call the aggregation operation to get the count of errors
    let errors_result = app_state
        .db
        .aggregation_ops_on_documents(&errors_collection_name, errors_count_pipeline)
        .await
        .map_err(|err| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                axum::Json(json!({"status": "error", "message": err.to_string()})),
            )
        })?;

    // Extract the count of errors
    let knowledge_node_errors = errors_result
        .first()
        .and_then(|doc| doc.get("count").and_then(serde_json::Value::as_u64))
        .unwrap_or(0);

    // Set all the counts in the response
    let counts = Counts {
        knowledge_node_errors,
        knowledge_node_file_store,
        knowledge_node_data_store,
    };

    let success_message = format!(
        "Count of knowledge nodes and errors fetched successfully for app '{}' between '{}' and '{}'.",
        app_name, start_timestamp, end_timestamp
    );
    info!(app_name = app_name, message = success_message);
    Ok(Json(
        json!({"status": "success", "message": success_message, "counts": counts,
        }),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::runtime::Runtime;

    #[test]
    fn test_success_get_knowledge_nodes_and_errors_count() {
        let rt = Runtime::new().unwrap();

        rt.block_on(async {
            // Create a dev AppState
            let app_state = crate::tests::test_get_appstate().await.unwrap();
            let app_name = "app100".to_string();

            // Call the function
            let result = get_knowledge_nodes_and_errors_count(
                Path(app_name.clone()),
                Query(QueryParams {
                    page: None,
                    limit: None,
                    app_name: None,
                    is_update: None,
                    search_enabled: None,
                    reference_id: None,
                    knowledge_node_type: None,
                    start_timestamp: Some("2024-05-02T00%3A00%3A00Z".to_string()),
                    end_timestamp: Some("2024-05-09T00%3A00%3A00Z".to_string()),
                    utc_start_timestamp: None,
                    utc_end_timestamp: None,
                }),
                State(app_state),
            )
            .await;

            // Check if the function returns Ok
            assert!(result.is_ok());
        });
    }

    #[test]
    fn test_failure_get_knowledge_nodes_and_errors_count_no_app_found() {
        let rt = Runtime::new().unwrap();

        rt.block_on(async {
            // Create a dev AppState and app_name
            let app_state = crate::tests::test_get_appstate().await.unwrap();
            let app_name = "non_existent_app".to_string();

            // Call the function
            let result = get_knowledge_nodes_and_errors_count(
                Path(app_name.clone()),
                Query(QueryParams {
                    page: None,
                    limit: None,
                    app_name: None,
                    is_update: None,
                    search_enabled: None,
                    reference_id: None,
                    knowledge_node_type: None,
                    start_timestamp: Some("2024-05-02T00%3A00%3A00Z".to_string()),
                    end_timestamp: Some("2024-05-09T00%3A00%3A00Z".to_string()),
                    utc_start_timestamp: None,
                    utc_end_timestamp: None,
                }),
                State(app_state),
            )
            .await;

            // If the function returns Err, check the status code and message
            let (status_code, Json(message)) = result.err().unwrap();
            assert_eq!(status_code, StatusCode::BAD_REQUEST);
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
    fn test_failure_get_knowledge_nodes_and_errors_count_start_timestamp_missing() {
        let rt = Runtime::new().unwrap();

        rt.block_on(async {
            // Create a dev AppState and app_name
            let app_state = crate::tests::test_get_appstate().await.unwrap();
            let app_name = "app100".to_string();

            // Call the function
            let result = get_knowledge_nodes_and_errors_count(
                Path(app_name.clone()),
                Query(QueryParams {
                    page: None,
                    limit: None,
                    app_name: None,
                    is_update: None,
                    search_enabled: None,
                    reference_id: None,
                    knowledge_node_type: None,
                    start_timestamp: None,
                    end_timestamp: Some("2024-05-09T00%3A00%3A00Z".to_string()),
                    utc_start_timestamp: None,
                    utc_end_timestamp: None,
                }),
                State(app_state),
            )
            .await;

            // If the function returns Err, check the status code and message
            let (status_code, Json(message)) = result.err().unwrap();
            assert_eq!(status_code, StatusCode::BAD_REQUEST);
            assert_eq!(message.get("status").unwrap().as_str().unwrap(), "error");
            assert!(message
                .get("message")
                .unwrap()
                .as_str()
                .unwrap()
                .contains("start_timestamp is required."));
        });
    }

    #[test]
    fn test_failure_get_knowledge_nodes_and_errors_count_end_timestamp_missing() {
        let rt = Runtime::new().unwrap();

        rt.block_on(async {
            // Create a dev AppState and app_name
            let app_state = crate::tests::test_get_appstate().await.unwrap();
            let app_name = "app100".to_string();

            // Call the function
            let result = get_knowledge_nodes_and_errors_count(
                Path(app_name.clone()),
                Query(QueryParams {
                    page: None,
                    limit: None,
                    app_name: None,
                    is_update: None,
                    search_enabled: None,
                    reference_id: None,
                    knowledge_node_type: None,
                    start_timestamp: Some("2024-05-02T00%3A00%3A00Z".to_string()),
                    end_timestamp: None,
                    utc_start_timestamp: None,
                    utc_end_timestamp: None,
                }),
                State(app_state),
            )
            .await;

            // If the function returns Err, check the status code and message
            let (status_code, Json(message)) = result.err().unwrap();
            assert_eq!(status_code, StatusCode::BAD_REQUEST);
            assert_eq!(message.get("status").unwrap().as_str().unwrap(), "error");
            assert!(message
                .get("message")
                .unwrap()
                .as_str()
                .unwrap()
                .contains("end_timestamp is required."));
        });
    }

    #[test]
    fn test_failure_get_knowledge_nodes_and_errors_count_start_timestamp_invalid() {
        let rt = Runtime::new().unwrap();

        rt.block_on(async {
            // Create a dev AppState and app_name
            let app_state = crate::tests::test_get_appstate().await.unwrap();
            let app_name = "app100".to_string();

            // Call the function
            let result = get_knowledge_nodes_and_errors_count(
                Path(app_name.clone()),
                Query(QueryParams {
                    page: None,
                    limit: None,
                    app_name: None,
                    is_update: None,
                    search_enabled: None,
                    reference_id: None,
                    knowledge_node_type: None,
                    start_timestamp: Some("2024-05-02T00%3A00%3A000Z".to_string()),
                    end_timestamp: Some("2024-05-09T00%3A00%3A00Z".to_string()),
                    utc_start_timestamp: None,
                    utc_end_timestamp: None,
                }),
                State(app_state),
            )
            .await;

            // If the function returns Err, check the status code and message
            let (status_code, Json(message)) = result.err().unwrap();
            assert_eq!(status_code, StatusCode::BAD_REQUEST);
            assert_eq!(message.get("status").unwrap().as_str().unwrap(), "error");
            assert!(message
                .get("message")
                .unwrap()
                .as_str()
                .unwrap()
                .contains("Invalid start timestamp "));
        });
    }

    #[test]
    fn test_failure_get_knowledge_nodes_and_errors_count_end_timestamp_invalid() {
        let rt = Runtime::new().unwrap();

        rt.block_on(async {
            // Create a dev AppState and app_name
            let app_state = crate::tests::test_get_appstate().await.unwrap();
            let app_name = "app100".to_string();

            // Call the function
            let result = get_knowledge_nodes_and_errors_count(
                Path(app_name.clone()),
                Query(QueryParams {
                    page: None,
                    limit: None,
                    app_name: None,
                    is_update: None,
                    search_enabled: None,
                    reference_id: None,
                    knowledge_node_type: None,
                    start_timestamp: Some("2024-05-02T00%3A00%3A00Z".to_string()),
                    end_timestamp: Some("2024-05-09T00%3A00%3A000Z".to_string()),
                    utc_start_timestamp: None,
                    utc_end_timestamp: None,
                }),
                State(app_state),
            )
            .await;

            // If the function returns Err, check the status code and message
            let (status_code, Json(message)) = result.err().unwrap();
            assert_eq!(status_code, StatusCode::BAD_REQUEST);
            assert_eq!(message.get("status").unwrap().as_str().unwrap(), "error");
            assert!(message
                .get("message")
                .unwrap()
                .as_str()
                .unwrap()
                .contains("Error parsing rfc3339 end_timestamp."));
        });
    }
}
