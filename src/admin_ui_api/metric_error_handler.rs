/*
 * Created Date:   Feb 23, 2024
 * -----
 * Copyright (c) 2024 Tresle.ai or its affiliates. All Rights Reserved.
 */
//! This module contains the asynchronous GET handler for fetching the number of errors made to the app.
//! The handler is used by the admin UI to fetch the number of errors made to the app.
//! The handler is mounted at `/api/v1.1/admin/metric/logs`.
//! The handler returns the number of errors if it exists, else returns an error message.
//! The handler returns a 200 status code if the errors are fetched successfully.
//! The handler returns a 400 status code if an error occurs while fetching the errors.
//! The handler returns a 500 status code if an error occurs while fetching the errors.
//! The handler returns a JSON response with the status and message.
//!

use crate::service::state::AppState;
use axum::body::Body;
use axum::extract::Query;
use axum::http::Request;
use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use logging_utils::create_ref_id_helper::create_ref_id;
use logging_utils::create_task_id_helper::create_task_id;
use logging_utils::create_task_ref_id_helper::create_task_ref_collection;
use serde::Deserialize;
use std::sync::Arc;
use tracing::{debug, error, instrument};

const METRIC_ERRORS_ENDPOINT: &str = "api/log/severity-count";

/// GET handler to fetch the number of errors made to the app.
#[utoipa::path(
    get,
    path = "/api/v1.1/admin/metric/logs",
    params(
        (
            "app_name" = inline(String), 
            Query,
            description = "app name.",
        ),
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
            "severity" = inline(Option<String>), 
            Query,
            description = "severity level.",
        ),
        (
            "count_only" = inline(Option<bool>), 
            Query,
            description = "severity count only.",
        )
    ),
    responses(
        (status = 200, description = "Metric errors fetched succesfully."),
        (status = StatusCode::BAD_REQUEST, description = "Invalid Request", body = [ErrorResponse]),
        (status = StatusCode::INTERNAL_SERVER_ERROR, description = "Tresle error occurred. Please save reference id: {} and contact support.")
    )
)]
#[instrument(skip_all)]
pub async fn get_metric_errors(
    State(app_state): State<Arc<AppState>>,
    request: Request<Body>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    // Create a reference ID ,task ID and initialize the documentdb variables
    let ref_id = create_ref_id();
    let service_type = "GetMetricError".to_string();
    let app_name = &app_state.app_settings.tracing_layer_system_app_name;
    let task_id = create_task_id(app_name, service_type);
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

    #[derive(Deserialize)]
    struct GetCallsParams {
        app_name: String,
        start_timestamp: String,
        end_timestamp: String,
        count_only: Option<bool>,
    }

    let param: Query<GetCallsParams> = match Query::try_from_uri(request.uri()) {
        Ok(param) => param,
        Err(e) => {
            // Handle the error here. You might want to log the error and return a default value or an error response.
            let error_message = format!("Failed to parse query parameters: {:?}", e);
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
            let _ = create_task_ref_collection(
                mongo_url,
                mongo_db_name,
                id_collection,
                app_name.to_string(),
                task_id,
                ref_id,
            )
            .await;
            return Err((
                axum::http::StatusCode::BAD_REQUEST,
                axum::Json(serde_json::json!({ "error": "Bad request" })),
            ));
        }
    };

    println!("{}", param.app_name);
    println!("{}", param.start_timestamp);
    println!("{}", param.end_timestamp);
    println!("{}", request.uri().path());

    debug!("Retrieving data from the metric microservice.");
    let url = format!(
        "{}/{}/{}",
        app_state
            .app_settings
            .tresleai_urls
            .logging_service_url
            .clone(),
        METRIC_ERRORS_ENDPOINT,
        param.app_name.clone()
    );

    debug!(
        "Making a Get request to the logging microservice at URL: {}",
        url
    );
    let client = reqwest::Client::new();

    let response = client
        .get(url.clone())
        .header("accept", "application/json")
        .query(&[
            ("start_timestamp", param.start_timestamp.clone()),
            ("end_timestamp", param.end_timestamp.clone()),
            ("count_only", param.count_only.unwrap_or(false).to_string()),
        ])
        .send()
        .await;

    match response {
        Ok(resp) => {
            let body = resp
                .text()
                .await
                .unwrap_or_else(|_| String::from("Failed to read response body"));
            let body = axum::body::Body::from(body);
            let response = axum::response::Response::new(body);
            Ok(response)
        }
        Err(_) => {
            let error_message = "Failed to send request".to_string();
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
            let _ = create_task_ref_collection(
                mongo_url,
                mongo_db_name,
                id_collection,
                app_name.to_string(),
                task_id,
                ref_id,
            )
            .await;
            let body = axum::body::Body::from("Failed to send request");
            let response = axum::response::Response::new(body);
            Ok(response)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::Request;
    use tokio::runtime::Runtime;

    #[test]
    fn test_success_get_metric_errors() {
        let rt = Runtime::new().unwrap();

        rt.block_on(async {
            // Create a dev AppState
            let app_state = crate::tests::test_get_appstate().await.unwrap();

            let request = Request::builder()
                .uri("/metricerrors?app_name=app12&start_timestamp=2024-04-02T23%3A59%3A59Z&end_timestamp=2024-04-03T23%3A59%3A59Z")
                .header("accept" , "application/json")
                .body(Body::empty())
                .unwrap();
            // Call the function
            let result = get_metric_errors(State(app_state), request).await;

            // Check that the result is as expected
            assert!(result.is_ok());
        });
    }
}
