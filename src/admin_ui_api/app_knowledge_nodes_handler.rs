/*
 * Created Date:   Feb 23, 2024
 * -----
 * Copyright (c) 2024 Tresle.ai or its affiliates. All Rights Reserved.
 */
//! This module contains the GET handler for fetching knowledge nodes for an app between two timestamps.
//! The handler is mounted at `/api/v1.1/admin/nodes/{app_name}`.
//! The handler returns the knowledge nodes if they exist, else returns an error message.
//! The handler returns a 200 status code if the knowledge nodes are fetched successfully.
//! The handler returns a 400 status code if an error occurs while fetching the knowledge nodes.
//! The handler returns a 500 status code if an error occurs while fetching the knowledge nodes.
//! The handler returns a JSON response with the status and message.
//!

use crate::admin_ui_api::schema::QueryParams;
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
use serde_json::{json, Value};
use std::sync::Arc;
use tracing::{debug, error, info, instrument};

/// GET handler to fetch knowledge nodes for an app between two timestamps.
#[utoipa::path(
    get,
    path = "/api/v1.1/admin/nodes/{app_name}",
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
        ),
        (
            "knowledge_node_type" = inline(String), 
            Query,
            description = "knowledge node type.",
        ),
        (
            "page" = inline(Option<usize>), 
            Query,
            description = "page number.",
        ),
        (
            "limit" = inline(Option<usize>), 
            Query,
            description = "page limit.",
        )
    ),
    responses(
        (status = 200, description = "Knowledge nodes for app fetched successfully."),
        (status = StatusCode::BAD_REQUEST, description = "Invalid Request", body = [ErrorResponse]),
        (status = StatusCode::INTERNAL_SERVER_ERROR, description = "Tresle error occurred. Please save reference id: {} and contact support.")
    )
)]
#[instrument(skip_all)]
pub async fn get_knowledge_nodes_handler(
    Path(app_name): Path<String>,
    Query(params): Query<QueryParams>,
    State(app_state): State<Arc<AppState>>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    // Create a reference ID ,task ID and initialize the documentdb variables
    let ref_id = create_ref_id();
    let service_type = "GetKNodeHandler".to_string();
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

    let start_timestamp_encoded = params.start_timestamp.ok_or_else(|| {
        let error_message = "start_timestamp is required.".to_string();
        error!(message = error_message);
        (
            StatusCode::BAD_REQUEST,
            Json(json!({"status": "error", "message": error_message})),
        )
    })?;

    // Decode the percent-encoded start timestamp
    let start_timestamp = percent_decode_str(&start_timestamp_encoded)
        .decode_utf8_lossy()
        .to_string();

    // Check if the decoded start timestamp is valid in RFC3339 format
    match DateTime::parse_from_rfc3339(&start_timestamp) {
        Ok(_) => {}
        Err(_) => {
            let error_message = format!("Invalid start timestamp '{}'.", start_timestamp);
            let ext_message = "Please provide a valid start timestamp".to_string();
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
    })?;

    let end_timestamp = percent_decode_str(&end_timestamp_encoded)
        .decode_utf8_lossy()
        .to_string();

    match DateTime::parse_from_rfc3339(&end_timestamp) {
        Ok(_) => {}
        Err(_) => {
            let error_message = format!("Invalid end timestamp '{}'.", end_timestamp);
            let ext_message = "Please provide a valid end timestamp".to_string();
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
                StatusCode::BAD_REQUEST,
                Json(json!({"status": "error", "message": error_message})),
            ));
        }
    };

    // Check if the app exists
    let app_exists = check_app_existence(&app_state, &app_name).await?;
    if !app_exists {
        let error_message = format!("No app found with name '{}'.", app_name);
        let ext_message = "Please provide a valid app name".to_string();
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
        debug!(message = error_message);
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({"status": "error", "message": error_message})),
        ));
    }

    let knowledge_node_type = params.knowledge_node_type.ok_or_else(|| {
        let error_message = "knowledge_node_type is required.".to_string();
        error!(message = error_message);
        (
            StatusCode::BAD_REQUEST,
            Json(json!({"status": "error", "message": error_message})),
        )
    })?;

    let node_label = match knowledge_node_type.as_str() {
        "knowledge_node_file_store" => "FileObject",
        "knowledge_node_data_store" => "DatabaseObjectNode",
        _ => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(json!({"status": "error", "message": "Invalid knowledge_node_type."})),
            ))
        }
    };

    let limit = params.limit.unwrap_or(10) as i64;
    let mut page = params.page.unwrap_or(1) as i64;

    let collection_name = format!("{}-general", app_name);

    // First query to get the count of documents
    let count_pipeline = vec![
        doc! {
            "$match": {
                "indexed_at": {
                    "$gte": start_timestamp.clone(),
                    "$lte": end_timestamp.clone(),
                },
                "_node_label": node_label,
            }
        },
        doc! {
            "$count": "count"
        },
    ];

    let count_result = app_state
        .db
        .aggregation_ops_on_documents(&collection_name, count_pipeline)
        .await
        .map_err(|err| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                axum::Json(json!({"status": "error", "message": err.to_string()})),
            )
        })?;

    let total_count = count_result.first().map_or(0, |doc| {
        doc.get("count")
            .and_then(serde_json::Value::as_i64)
            .unwrap_or(0)
    });

    // Pagination calculation - Determine total pages, page(if needed) and skip value
    let total_pages = (total_count as f64 / limit as f64).ceil() as i64;

    // If page is negative or total_pages is 0, set page to 1. If page is > total_pages, set page to total_pages
    if page < 1 || total_pages == 0 {
        page = 1;
    } else if page > total_pages {
        page = total_pages;
    }
    let skip = (page - 1) * limit;

    let count_str = total_count.to_string();
    let json_count = serde_json::from_str(&count_str).unwrap_or(Value::Number(0.into()));
    json_count.as_i64().unwrap_or(0);

    // Second query to get the nodes subject to $skip and $limit
    let nodes_pipeline = vec![
        doc! {
            "$match": {
                "indexed_at": {
                    "$gte": start_timestamp.clone(),
                    "$lte": end_timestamp.clone(),
                },
                "_node_label": node_label,
            }
        },
        doc! {
            "$project": {
                "_id": 0,
                "indexed_at": 1,
                "source": 1,
                "total_page_num": {
                    "$cond": {
                        "if": { "$eq": [ "$_node_label", "FileObject" ] },
                        "then": "$total_page_num",
                        "else": null
                    }
                },
            }
        },
        doc! { "$skip": skip },
        doc! { "$limit": limit },
    ];

    let nodes_result = app_state
        .db
        .aggregation_ops_on_documents(&collection_name, nodes_pipeline)
        .await
        .map_err(|err| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                axum::Json(json!({"status": "error", "message": err.to_string()})),
            )
        })?;

    let success_message = format!(
        "Knowledge nodes fetched successfully for app '{}' between '{}' and '{}'.",
        app_name, start_timestamp, end_timestamp
    );
    info!(app_name = app_name, message = success_message);
    Ok(Json(
        json!({"status": "success", "message": success_message, "nodes": nodes_result, 
        "total_pages": total_pages, "total_results": total_count}),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::runtime::Runtime;

    #[test]
    fn test_success_get_knowledge_nodes_handler() {
        let rt = Runtime::new().unwrap();

        rt.block_on(async {
            // Create a dev AppState
            let app_state = crate::tests::test_get_appstate().await.unwrap();
            let app_name = "app100".to_string();

            // Call the function
            let result = get_knowledge_nodes_handler(
                Path(app_name.clone()),
                Query(QueryParams {
                    page: None,
                    limit: None,
                    app_name: None,
                    is_update: None,
                    search_enabled: None,
                    reference_id: None,
                    knowledge_node_type: Some("knowledge_node_file_store".to_string()),
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
    fn test_failure_get_knowledge_nodes_handler_no_app_found() {
        let rt = Runtime::new().unwrap();

        rt.block_on(async {
            // Create a dev AppState and app_name
            let app_state = crate::tests::test_get_appstate().await.unwrap();
            let app_name = "non_existent_app".to_string();

            // Call the function
            let result = get_knowledge_nodes_handler(
                Path(app_name.clone()),
                Query(QueryParams {
                    page: None,
                    limit: None,
                    app_name: None,
                    is_update: None,
                    search_enabled: None,
                    reference_id: None,
                    knowledge_node_type: Some("knowledge_node_file_store".to_string()),
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
    fn test_failure_get_knowledge_nodes_handler_start_timestamp_missing() {
        let rt = Runtime::new().unwrap();

        rt.block_on(async {
            // Create a dev AppState and app_name
            let app_state = crate::tests::test_get_appstate().await.unwrap();
            let app_name = "app100".to_string();

            // Call the function
            let result = get_knowledge_nodes_handler(
                Path(app_name.clone()),
                Query(QueryParams {
                    page: None,
                    limit: None,
                    app_name: None,
                    is_update: None,
                    search_enabled: None,
                    reference_id: None,
                    knowledge_node_type: Some("knowledge_node_file_store".to_string()),
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
    fn test_failure_get_knowledge_nodes_handler_end_timestamp_missing() {
        let rt = Runtime::new().unwrap();

        rt.block_on(async {
            // Create a dev AppState and app_name
            let app_state = crate::tests::test_get_appstate().await.unwrap();
            let app_name = "app100".to_string();

            // Call the function
            let result = get_knowledge_nodes_handler(
                Path(app_name.clone()),
                Query(QueryParams {
                    page: None,
                    limit: None,
                    app_name: None,
                    is_update: None,
                    search_enabled: None,
                    reference_id: None,
                    knowledge_node_type: Some("knowledge_node_file_store".to_string()),
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
    fn test_failure_get_knowledge_nodes_handler_knowledge_node_type_missing() {
        let rt = Runtime::new().unwrap();

        rt.block_on(async {
            // Create a dev AppState and app_name
            let app_state = crate::tests::test_get_appstate().await.unwrap();
            let app_name = "app100".to_string();

            // Call the function
            let result = get_knowledge_nodes_handler(
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
                .contains("knowledge_node_type is required."));
        });
    }

    #[test]
    fn test_failure_get_knowledge_nodes_handler_start_timestamp_invalid() {
        let rt = Runtime::new().unwrap();

        rt.block_on(async {
            // Create a dev AppState and app_name
            let app_state = crate::tests::test_get_appstate().await.unwrap();
            let app_name = "app100".to_string();

            // Call the function
            let result = get_knowledge_nodes_handler(
                Path(app_name.clone()),
                Query(QueryParams {
                    page: None,
                    limit: None,
                    app_name: None,
                    is_update: None,
                    search_enabled: None,
                    reference_id: None,
                    knowledge_node_type: Some("knowledge_node_file_store".to_string()),
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
    fn test_failure_get_knowledge_nodes_handler_end_timestamp_invalid() {
        let rt = Runtime::new().unwrap();

        rt.block_on(async {
            // Create a dev AppState and app_name
            let app_state = crate::tests::test_get_appstate().await.unwrap();
            let app_name = "app100".to_string();

            // Call the function
            let result = get_knowledge_nodes_handler(
                Path(app_name.clone()),
                Query(QueryParams {
                    page: None,
                    limit: None,
                    app_name: None,
                    is_update: None,
                    search_enabled: None,
                    reference_id: None,
                    knowledge_node_type: Some("knowledge_node_file_store".to_string()),
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
                .contains("Invalid end timestamp "));
        });
    }

    #[test]
    fn test_failure_get_knowledge_nodes_handler_knowledge_node_type_invalid() {
        let rt = Runtime::new().unwrap();

        rt.block_on(async {
            // Create a dev AppState and app_name
            let app_state = crate::tests::test_get_appstate().await.unwrap();
            let app_name = "app100".to_string();

            // Call the function
            let result = get_knowledge_nodes_handler(
                Path(app_name.clone()),
                Query(QueryParams {
                    page: None,
                    limit: None,
                    app_name: None,
                    is_update: None,
                    search_enabled: None,
                    reference_id: None,
                    knowledge_node_type: Some("invalid_knowledge_node_type".to_string()),
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
                .contains("Invalid knowledge_node_type."));
        });
    }

    #[test]
    fn test_success_get_knowledge_nodes_handler_negative_page() {
        let rt = Runtime::new().unwrap();

        rt.block_on(async {
            // Create a dev AppState and app_name
            let app_state = crate::tests::test_get_appstate().await.unwrap();
            let app_name = "app100".to_string();
            let page: i32 = -1;

            // Call the function
            let result = get_knowledge_nodes_handler(
                Path(app_name.clone()),
                Query(QueryParams {
                    page: Some(page as usize),
                    limit: None,
                    app_name: None,
                    is_update: None,
                    search_enabled: None,
                    reference_id: None,
                    knowledge_node_type: Some("knowledge_node_file_store".to_string()),
                    start_timestamp: Some("2024-05-02T00%3A00%3A00Z".to_string()),
                    end_timestamp: Some("2024-05-09T00%3A00%3A00Z".to_string()),
                    utc_start_timestamp: None,
                    utc_end_timestamp: None,
                }),
                State(app_state),
            )
            .await;

            // Check if the function returns Ok
            assert!(result.is_ok())
        });
    }
}
