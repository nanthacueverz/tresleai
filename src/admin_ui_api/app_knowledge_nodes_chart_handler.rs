/*
 * Created Date:   Feb 23, 2024
 * -----
 * Copyright (c) 2024 Tresle.ai or its affiliates. All Rights Reserved.
 */
//! This module contains the GET handler for fetching the data for knowledge nodes for an app
//! between two timestamps. The data is then displayed on a chart on admin UI.
//! The handler is mounted at `/api/v1.1/admin/nodes/chart/{app_name}`.
//! The handler is called by the admin UI to fetch the data for knowledge nodes for an app
//! between two timestamps.
//! The handler returns the data for knowledge nodes for an app if it exists, else returns an error message.
//! The handler returns a 200 status code if the data is fetched successfully.
//! The handler returns a 400 status code if an error occurs while fetching the data.
//! The handler returns a 500 status code if an error occurs while fetching the data.
//! The handler returns a JSON response with the status and message.
//!

use crate::admin_ui_api::schema::{
    GraphItem, KnowledgeNodeChartCount, NodesChartApiResponse, QueryParams,
};
use crate::service::check_app_existence::check_app_existence;
use crate::service::state::AppState;
use api_utils::errors::error_interceptor::ErrorInterceptor;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use chrono::{DateTime, Utc};
use logging_utils::create_ref_id_helper::create_ref_id;
use logging_utils::create_task_id_helper::create_task_id;
use logging_utils::create_task_ref_id_helper::create_task_ref_collection;
use mongodb::bson::doc;
use mongodb::bson::Document;
use serde_json::json;
use std::fmt::Debug;
use std::sync::Arc;
use tracing::{debug, error, instrument};

/// GET handler to fetch the data for knowledge nodes for an app between two timestamps. The data is then displayed on a chart on admin UI.
#[utoipa::path(
    get,
    path = "/api/v1.1/admin/nodes/chart/{app_name}",
    params(
        (
            "utc_start_timestamp" = inline(Option<DateTime<Utc>>), 
            Query,
            description = "UTC start timestamp.",
        ),
        (
            "utc_end_timestamp" = inline(Option<DateTime<Utc>>), 
            Query,
            description = "UTC end timestamp.",
        )
    ),
    responses(
        (status = 200, description = "Chart data for knowledge nodes for app fetched successfully."),
        (status = StatusCode::BAD_REQUEST, description = "Invalid Request", body = [ErrorResponse]),
        (status = StatusCode::INTERNAL_SERVER_ERROR, description = "Tresle error occurred. Please save reference id: {} and contact support.")
    )
)]
#[instrument(skip_all)]
pub async fn get_knowledge_nodes_chart_handler(
    Path(app_name): Path<String>,
    Query(params): Query<QueryParams>,
    State(app_state): State<Arc<AppState>>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    // Create a reference ID ,task ID and initialize the documentdb variables
    let ref_id = create_ref_id();
    let service_type = "GetNodeChart".to_string();
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

    let base_pipeline_doc = vec![
        doc! {
            "$project": doc! {
                "_id": 0,
                "date": doc! {
                    "$toDate": "$indexed_at"
                }
            }
        },
        doc! {
            "$project": doc! {
                "_id": 0,
                "count": 1,
                "indexed_at": "$_id"
            }
        },
    ];
    let mut pipeline_doc = base_pipeline_doc;

    let (start_timestamp, end_timestamp, timestamp_interval, timestamp_group_doc) =
        process_timestamp_data(params.utc_start_timestamp, params.utc_end_timestamp).await;

    let query_doc = doc! {
        "indexed_at": doc! {
            "$gte": start_timestamp,
            "$lte": end_timestamp
        }
    };

    pipeline_doc.insert(
        0,
        doc! {
            "$match": query_doc.clone()
        },
    );
    pipeline_doc.insert(2, timestamp_group_doc);

    let collection_name = format!("{}-general", app_name);

    let mut resp = NodesChartApiResponse {
        graph_interval: timestamp_interval,
        ..Default::default()
    };
    match app_state
        .db
        .aggregation_ops_on_documents(&collection_name, pipeline_doc.clone())
        .await
        .map_err(ErrorInterceptor::from)
    {
        Ok(res) => {
            let mut knowledge_nodes_data: Vec<GraphItem> = Vec::new();
            for knowledge_node in res {
                let knowledge_node_model = doc_to_type::<KnowledgeNodeChartCount>(knowledge_node)?;
                knowledge_nodes_data.push(knowledge_node_model.into());
            }
            resp.graph_items = knowledge_nodes_data;
        }
        Err(e) => return Err(e.intercept_error().await),
    }

    match app_state
        .db
        .get_document_count(&collection_name, query_doc)
        .await
        .map_err(ErrorInterceptor::from)
    {
        Ok(res) => {
            resp.count = res.to_string();
            Ok(Json(resp))
        }
        Err(e) => Err(e.intercept_error().await),
    }
}

/// (Helper fn) process timestamp related data
/// returning start and end timestamps, interval, and group doc based on the input timestamps
pub async fn process_timestamp_data(
    start_ts: Option<DateTime<Utc>>,
    end_ts: Option<DateTime<Utc>>,
) -> (String, String, String, Document) {
    let end_timestamp = match end_ts {
        Some(ts) => ts,
        None => Utc::now(),
    };
    let start_timestamp = match start_ts {
        Some(ts) => ts,
        None => {
            // approx last 6 months
            end_timestamp - chrono::Duration::days(180)
        }
    };

    // Calculate the difference in days
    let duration = end_timestamp.signed_duration_since(start_timestamp);
    let num_days = duration.num_days();

    // Determine the interval and timestamp grouping document based on the number of days
    let (interval, format) = if num_days < 3 {
        ("hour", "%Y-%m-%dT%H:00:00Z")
    } else if num_days < 60 {
        ("day", "%Y-%m-%dT00:00:00Z")
    } else {
        ("month", "%Y-%m-00T00:00:00Z")
    };

    let group_doc = doc! {
        "$group": doc! {
            "_id": doc! {
                "$dateToString": doc! {
                    "format": format,
                    "date": "$date"
                }
            },
            "count": doc! {
                "$sum": 1
            }
        }
    };

    (
        start_timestamp.to_rfc3339().to_string(),
        end_timestamp.to_rfc3339().to_string(),
        interval.to_string(),
        group_doc,
    )
}

/// Converts a json value to rust type
fn doc_to_type<T>(doc: serde_json::Value) -> Result<T, (StatusCode, Json<serde_json::Value>)>
where
    T: serde::de::DeserializeOwned + Default + Debug,
{
    let model: Result<T, _> = serde_json::from_value(doc);
    match model {
        Ok(model) => Ok(model),
        Err(_) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
            "error": "Deserializing error occurred from DB result(s)" })),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};
    use tokio::runtime::Runtime;

    #[test]
    fn test_success_get_knowledge_nodes_chart_handler() {
        let rt = Runtime::new().unwrap();

        rt.block_on(async {
            // Create a dev AppState
            let app_state = crate::tests::test_get_appstate().await.unwrap();
            let app_name = "app100".to_string();

            // Call the function
            let result = get_knowledge_nodes_chart_handler(
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
                    end_timestamp: None,
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
    fn test_failure_get_knowledge_nodes_chart_handler_no_app_found() {
        let rt = Runtime::new().unwrap();

        rt.block_on(async {
            // Create a dev AppState and app_name
            let app_state = crate::tests::test_get_appstate().await.unwrap();
            let app_name = "non_existent_app".to_string();

            // Call the function
            let result = get_knowledge_nodes_chart_handler(
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
                    end_timestamp: None,
                    utc_start_timestamp: Some(Utc::now()),
                    utc_end_timestamp: Some(Utc::now()),
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
    fn test_success_get_knowledge_nodes_chart_handler_utc_end_timestamp_missing() {
        let rt = Runtime::new().unwrap();

        rt.block_on(async {
            // Create a dev AppState and app_name
            let app_state = crate::tests::test_get_appstate().await.unwrap();
            let app_name = "app100".to_string();

            // Call the function
            let result = get_knowledge_nodes_chart_handler(
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
                    end_timestamp: None,
                    utc_start_timestamp: Some(Utc::now()),
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
    fn test_success_get_knowledge_nodes_chart_handler_utc_start_timestamp_missing() {
        let rt = Runtime::new().unwrap();

        rt.block_on(async {
            // Create a dev AppState and app_name
            let app_state = crate::tests::test_get_appstate().await.unwrap();
            let app_name = "app100".to_string();

            // Call the function
            let result = get_knowledge_nodes_chart_handler(
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
                    end_timestamp: None,
                    utc_start_timestamp: None,
                    utc_end_timestamp: Some(Utc::now()),
                }),
                State(app_state),
            )
            .await;

            // Check if the function returns Ok
            assert!(result.is_ok());
        });
    }

    use serde_json::json;

    #[derive(Serialize, Deserialize, Debug, Default, PartialEq)]
    struct TestStruct {
        field: String,
    }

    #[test]
    fn test_success_doc_to_type() {
        let doc = json!({
            "field": "test"
        });

        let result: Result<TestStruct, _> = doc_to_type(doc);
        assert_eq!(
            result.unwrap(),
            TestStruct {
                field: "test".to_string()
            }
        );
    }

    #[test]
    fn test_failure_doc_to_type() {
        let doc = json!({
            "wrong_field": "test"
        });

        let result: Result<TestStruct, _> = doc_to_type(doc);
        // If the function returns Err, check the status code and message
        let (status_code, Json(message)) = result.err().unwrap();
        assert_eq!(status_code, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(
            message.get("error").unwrap().as_str().unwrap(),
            "Deserializing error occurred from DB result(s)"
        );
    }
}
