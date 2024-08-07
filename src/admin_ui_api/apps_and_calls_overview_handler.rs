/*
 * Created Date:   Feb 23, 2024
 * -----
 * Copyright (c) 2024 Tresle.ai or its affiliates. All Rights Reserved.
 */
//! This module contains the GET handler for fetching the overview of calls made from different apps during the last 6 months.
//! The handler is used by the admin UI to fetch the overview of calls made from different apps during the last 6 months.
//! The handler is mounted at `/api/v1.1/admin/overview`.
//! The handler returns the overview of apps and calls if it exists, else returns an error message.
//! The handler returns a 200 status code if the overview is fetched successfully.
//! The handler returns a 400 status code if an error occurs while fetching the overview.
//! The handler returns a 500 status code if an error occurs while fetching the overview.
//! The handler returns a JSON response with the status and message.
//!
use crate::service::state::AppState;
use api_utils::errors::error_interceptor::ErrorInterceptor;
use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use chrono::{Duration, Utc};
use mongodb::bson::doc;
use serde_json::json;
use std::sync::Arc;
use tracing::{debug, instrument};

/// GET handler to fetch the overview of calls made from different apps during the last 6 months.
#[utoipa::path(
    get,
    path = "/api/v1.1/admin/overview",
    responses(
        (status = 200, description = "Overview of apps and calls fetched successfully."),
        (status = StatusCode::BAD_REQUEST, description = "Invalid Request", body = [ErrorResponse]),
        (status = StatusCode::INTERNAL_SERVER_ERROR, description = "Tresle error occurred. Please save reference id: {} and contact support.")
    )
)]
#[instrument(skip_all)]
pub async fn get_apps_and_calls_overview_handler(
    State(app_state): State<Arc<AppState>>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    let collection_name = &app_state
        .app_settings
        .mongo_db
        .mongo_db_ui_summary_collection;
    let iso_date_6_months_ago = match Utc::now().checked_sub_signed(Duration::days(180)) {
        Some(date) => date,
        None => {
            let error_message = "Failed to calculate the date 6 months ago from the current date.";
            debug!(message = error_message);
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"status": "error", "message": error_message})),
            ));
        }
    };
    let date_string = iso_date_6_months_ago.to_rfc3339();

    // Create an aggregation pipeline
    let aggregation_pipeline = vec![
        // Filter out the documents with timestamp within the last 6 months from the current date
        doc! {
            "$match": {
                "timestamp": {
                    "$gte": date_string,
                }
            }
        },
        // Group by month and year. Then for each group, get all the unique apps and total calls made by those apps.
        doc! {
            "$group": {
                "_id": {
                    "month": {
                        "$month": {
                            "$dateFromString": {
                                "dateString": "$timestamp",
                            },
                        },
                    },
                    "year": {
                        "$year": {
                            "$dateFromString": {
                                "dateString": "$timestamp",
                            },
                        },
                    },
                },
                "app_names": {
                    "$addToSet": "$app_name",
                },
                "total_calls": {
                    "$sum": 1,
                },
            }
        },
        // Get total no. of unique apps. Value '1' for 'total_count' indicates copying the field as is from the previous doc/stage
        doc! {
            "$project": {
                "total_apps": {
                    "$size": "$app_names"
                },
                "total_calls": 1
            },
        },
        // Sort by 'year' first and then by 'month' (in case of same year). Value '1' for ascending order and '-1' for descending order
        doc! {
            "$sort": {
                "_id.year": 1,
                "_id.month": 1
            }
        },
    ];

    match app_state
        .db
        .aggregation_ops_on_documents(collection_name, aggregation_pipeline)
        .await
        .map_err(ErrorInterceptor::from)
    {
        Ok(results) => {
            let success_message = format!(
                "Overview of apps and calls fetched successfully from {} onwards",
                iso_date_6_months_ago
            );
            debug!(message = success_message);
            Ok(Json(
                json!({"status": "success", "message": success_message, "data": results}),
            ))
        }
        Err(e) => {
            let error_message = format!(
                "Failed to fetch the overview of apps and calls from {} onwards. Error: {}",
                iso_date_6_months_ago, e
            );
            debug!(message = error_message);
            Err(e.intercept_error().await)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::runtime::Runtime;

    #[test]
    fn test_success_apps_and_calls_overview_handler() {
        let rt = Runtime::new().unwrap();

        rt.block_on(async {
            // Create a dev AppState
            let app_state = crate::tests::test_get_appstate().await.unwrap();

            // Call the function
            let result = get_apps_and_calls_overview_handler(State(app_state)).await;

            // Check if the function returns Ok
            assert!(result.is_ok());
        });
    }
    /*  todo : fix this test
    #[test]
    #[ignore="until aggregation_ops_on_documents returns an error"]
    fn test_failure_apps_and_calls_overview_handler() {
        let rt = Runtime::new().unwrap();

        rt.block_on(async {
            // Create a dev AppState
            let app_state = crate::tests::test_get_appstate().await.unwrap();

            // Call the function
            let result = apps_and_calls_overview_handler(State(app_state)).await;

            // If the function returns Err, check the status code and message
            let (status_code, Json(message)) = result.err().unwrap();
            assert_eq!(status_code, StatusCode::INTERNAL_SERVER_ERROR);
            assert_eq!(message.get("status").unwrap().as_str().unwrap(), "error");
            assert!(message.get("message").unwrap().as_str().unwrap().contains("Failed to fetch the overview of apps and calls from "));
        });
    }
    */
}
