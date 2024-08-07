/*
*  Created Date:  Mar 17, 2024
 * -----
 * Copyright (c) 2024 Tresle.ai or its affiliates. All Rights Reserved.
 */
//! This module contains the function to create an API key for the app.
//! The function is used by the onboarding service to create an API key for the app.
//! The function returns the API key and API key ID if the API key is created successfully.
//! The function returns a 500 status code if an error occurs while creating the API key.
//! The function returns a JSON response with the status and message.
//!

use crate::service::state::AppState;
use aws_config::meta::region::RegionProviderChain;
use aws_config::{BehaviorVersion, Region};
use axum::{http::StatusCode, Json};
use serde_json::json;
use std::sync::Arc;
use tracing::{debug, error, info, instrument};

/// Asynchronous function to create an API key for the app.
#[instrument(skip_all)]
pub async fn create_api_key(
    app_state: &Arc<AppState>,
    app_name: &String,
) -> Result<(String, String), (StatusCode, Json<serde_json::Value>)> {
    debug!("Creating an API key for the app.");
    let api_key_name = format!(
        "{}-{}-{}",
        app_state.app_settings.product_name.clone(),
        app_state.app_settings.env_identifier.clone(),
        app_name
    );

    let region = app_state.app_settings.aws_api_gateway.region.clone();
    let region_provider = RegionProviderChain::first_try(Region::new(region));

    let config = aws_config::defaults(BehaviorVersion::latest())
        .region(region_provider)
        .load()
        .await;
    let client = aws_sdk_apigateway::Client::new(&config);

    match client
        .create_api_key()
        .name(api_key_name)
        .enabled(true)
        .send()
        .await
    {
        Ok(response) => match (response.value(), response.id()) {
            (Some(api_key), Some(api_key_id)) => {
                let success_message = format!("API key created successfully for app {}.", app_name);
                info!(app_name = app_name, message = success_message);
                Ok((api_key.to_string(), api_key_id.to_string()))
            }
            (_, _) => {
                let error_message =
                    format!("API key creation validation failed for app {}.", app_name);
                error!(ext_message = error_message, message = error_message);
                Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({"status": "error","message": error_message})),
                ))
            }
        },
        Err(e) => {
            let error_message =
                format!("API key creation failed for app {}. Error: {}", app_name, e);
            error!(ext_message = error_message, message = error_message);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"status": "error","message": error_message})),
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    //  use super::*;
    //  use tokio::runtime::Runtime;
    //  use axum::Json;

    /* to fix test
    #[test]
    #[ignore="for now to prevent creating multiple API keys in API gateway"]
    fn test_success_create_api_key() {
        let rt = Runtime::new().unwrap();

        rt.block_on(async {
            // Create a dev AppState and app_name
            let app_state = crate::tests::test_get_appstate().await.unwrap();
            let app_name = "test".to_string();

            // Call the function
            let result = create_api_key(&app_state, &app_name).await;

            // If the function returns Ok, check the API key
            assert!(result.is_ok());
            let api_key = result.unwrap();
            assert!(!api_key.is_empty());
        });
    }

    #[test]
    #[ignore="until until api gateway returns an error"]
    fn test_failure_create_api_key_error_in_creation() {
        let rt = Runtime::new().unwrap();

        rt.block_on(async {
            // Create a dev AppState and app_name
            let app_state = crate::tests::test_get_appstate().await.unwrap();
            let app_name = "test".to_string();

            // Call the function
            let result = create_api_key(&app_state, &app_name).await;

            // If the function returns Err, check the status code and message
            let (status_code, Json(message)) = result.unwrap_err();
            assert_eq!(status_code, StatusCode::INTERNAL_SERVER_ERROR);
            assert_eq!(message.get("status").unwrap().as_str().unwrap(), "error");
            assert!(message.get("message").unwrap().as_str().unwrap().contains("API key creation failed."));
        });
    }
    */
}
