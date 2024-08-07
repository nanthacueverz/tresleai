/*
 * Created Date:   Feb 23, 2024
 * -----
 * Copyright (c) 2024 Tresle.ai or its affiliates. All Rights Reserved.
 */
//! This module contains the asynchronous GET handler for fetching the logging data for the app.
//! The handler is used by the admin UI to fetch the logging data for the app.
//! The handler is mounted at `/api/v1.1/admin/logs`.
//! The handler is called by the admin UI to fetch the logging data for the app.
//! The handler returns the logging data if it exists, else returns an error message.
//! The handler returns a 200 status code if the logging data is fetched successfully.
//! The handler returns a 400 status code if an error occurs while fetching the logging data.
//! The handler returns a 500 status code if an error occurs while fetching the logging data.
//! The handler returns a JSON response with the status and message.
//!

use crate::service::state::AppState;
use axum::body::Body;
use axum::http::Request;
use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use std::sync::Arc;
use tracing::{debug, instrument};

const METRIC_CALLS_ENDPOINT: &str = "api/all-logs/";

/// GET handler to fetch the logging data for the app.
#[utoipa::path(
    get,
    path = "/api/v1.1/admin/logs",
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
        )
    ),
    responses(
        (status = 200, description = "Logs calls retrieved successfully."),
        (status = StatusCode::BAD_REQUEST, description = "Invalid Request", body = [ErrorResponse]),
        (status = StatusCode::INTERNAL_SERVER_ERROR, description = "Tresle error occurred. Please save reference id: {} and contact support.")
    )
)]
#[instrument(skip_all)]
pub async fn get_logs(
    State(app_state): State<Arc<AppState>>,
    request: Request<Body>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    println!("{}", request.uri().path());
    let query_string = request.uri().query().unwrap_or_default();

    debug!("Retrieving data from the logging microservice.");
    let url = format!(
        "{}/{}?{}",
        app_state
            .app_settings
            .tresleai_urls
            .logging_service_url
            .clone(),
        METRIC_CALLS_ENDPOINT,
        query_string
    );

    debug!(
        "Making a Get request to the log microservice at URL: {}",
        url
    );
    let client = reqwest::Client::new();

    let response = client
        .get(url)
        .header("accept", "application/json")
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
    fn test_success_get_logs() {
        let rt = Runtime::new().unwrap();

        rt.block_on(async {
            // Create a dev AppState
            let app_state = crate::tests::test_get_appstate().await.unwrap();

            let request = Request::builder()
                .uri("/metriccalls?app_name=app12&start_timestamp=2024-02-23T00:00:00Z&end_timestamp=2024-02-23T23:59:59Z")
                .header("accept" , "application/json")
                .body(Body::empty())
                .unwrap();
            // Call the function
            let result = get_logs(State(app_state), request).await;

            // Check that the result is as expected
            assert!(result.is_ok());
        });
    }
}
