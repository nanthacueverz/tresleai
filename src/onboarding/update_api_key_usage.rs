/*
*  Created Date:  Mar 17, 2024
 * -----
 * Copyright (c) 2024 Tresle.ai or its affiliates. All Rights Reserved.
 */
//! This module contains the function to check if the api key is associated with the usage plan.
//! If not, associates the api key with the usage plan.
//! The function returns a boolean value indicating if the api key is associated with the usage plan.
//! The function returns a 500 status code if an error occurs while checking the usage plan.
//! The function returns a JSON response with the status and message.
//!

use crate::service::state::AppState;
use aws_config::meta::region::RegionProviderChain;
use aws_config::{BehaviorVersion, Region};
use axum::{http::StatusCode, Json};
use logging_utils::create_ref_id_helper::create_ref_id;
use logging_utils::create_task_ref_id_helper::create_task_ref_collection;
use serde_json::json;
use std::sync::Arc;
use tracing::{debug, error, info, instrument};

/// Asynchronous function to check if the api key is associated with the usage plan.
#[instrument(skip_all)]
pub async fn check_usage_plan_with_api_key(
    client: aws_sdk_apigateway::Client,
    usage_plan_id: String,
    api_key_id: String,
) -> Result<bool, (StatusCode, Json<serde_json::Value>)> {
    let mut position: Option<String> = None;

    loop {
        let mut request = client
            .get_usage_plan_keys()
            .usage_plan_id(usage_plan_id.clone());

        if let Some(p) = position {
            request = request.position(p);
        }

        let response = request.send().await;

        let response = match response {
            Ok(response) => response,
            Err(e) => {
                let error_message = format!("Failed to get usage plan keys: {:?}", e);
                error!(ext_message = error_message, message = error_message);
                return Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({"status": "error","message": error_message})),
                ));
            }
        };

        let keys = match response.items {
            Some(k) => k,
            None => Vec::new(),
        };

        let is_associated = keys
            .iter()
            .any(|item| item.id.as_ref() == Some(&api_key_id));

        if is_associated || response.position.is_none() {
            return Ok(is_associated);
        }

        position = response.position;
    }
}

/// Asynchronous function to associate the api key with the usage plan.
#[instrument(skip_all)]
pub async fn associate_api_key_with_usage_plan(
    client: aws_sdk_apigateway::Client,
    usage_plan_id: String,
    api_key_id: String,
    key_type: String,
    task_id: String,
    app_name: String,
) -> Result<(), (StatusCode, Json<serde_json::Value>)> {
    let res = client
        .create_usage_plan_key()
        .usage_plan_id(usage_plan_id.clone())
        .key_id(api_key_id.clone())
        .key_type(key_type)
        .send()
        .await;
    match res {
        Ok(response) => {
            let success_message = format!(
                "API key associated with the usage plan successfully : {:?}.",
                response
            );
            info!(
                app_name = app_name,
                task_id = task_id,
                message = success_message
            );
            Ok(())
        }
        Err(e) => {
            let error_message = format!(
                "Failed to associate 'API key:{}' with 'usage plan id:{}': {:?}",
                api_key_id, usage_plan_id, e
            );
            error!(
                app_name = app_name,
                task_id = task_id,
                ext_message = error_message,
                message = error_message
            );
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"status": "error","message": error_message})),
            ))
        }
    }
}

/// Asynchronous function to check if the usage plan exists.
#[instrument(skip_all)]
pub async fn check_usage_plan_exists(
    client: aws_sdk_apigateway::Client,
    usage_plan_id: String,
    app_name: String,
    task_id: String,
) -> Result<bool, (StatusCode, Json<serde_json::Value>)> {
    match client
        .get_usage_plan()
        .usage_plan_id(usage_plan_id.clone())
        .send()
        .await
    {
        Ok(_) => Ok(true),
        Err(e) => {
            let error_message = format!("Failed to get 'usage plan id:{}': {:?}", usage_plan_id, e);
            error!(
                app_name = app_name,
                task_id = task_id,
                ext_message = error_message,
                message = error_message
            );
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"status": "error","message": error_message})),
            ))
        }
    }
}

/// Asynchronous function to update the API key usage plan for the app.
#[instrument(skip_all)]
pub async fn update_api_key_with_usage_plan(
    app_state: &Arc<AppState>,
    api_key_id: String,
    task_id: String,
    app_name: &String,
) -> Result<(), (StatusCode, Json<serde_json::Value>)> {
    // Create a reference ID ,task ID and initialize the documentdb variables
    let ref_id = create_ref_id();
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
    debug!("Updating the API key usage plan for the app.");
    let region = app_state.app_settings.aws_api_gateway.region.clone();
    let usage_plan_id = app_state.app_settings.aws_api_gateway.usage_plan_id.clone();
    let key_type = app_state
        .app_settings
        .aws_api_gateway
        .usage_plan_key_type
        .clone();
    let region_provider = RegionProviderChain::first_try(Region::new(region));
    let config = aws_config::defaults(BehaviorVersion::latest())
        .region(region_provider)
        .load()
        .await;
    //create the api gateway client
    let client = aws_sdk_apigateway::Client::new(&config);

    //check if the usage plan exists
    let usage_plan_exists = match check_usage_plan_exists(
        client.clone(),
        usage_plan_id.clone(),
        app_name.clone(),
        task_id.clone(),
    )
    .await
    {
        Ok(usage_plan_exists) => usage_plan_exists,
        Err((status_code, json)) => {
            let error_message = format!("Usage plan : '{}' does not exist.", usage_plan_id);
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
                ref_id.clone(),
            )
            .await;
            error!(
                app_name = app_name,
                task_id = task_id,
                ext_message = ext_message,
                message = error_message
            );
            return Err((status_code, json));
        }
    };
    //if usage plan does not exist return error
    if !usage_plan_exists {
        let error_message = format!("Usage plan : '{}' does not exist.", usage_plan_id);
        error!(
            app_name = app_name,
            task_id = task_id,
            ext_message = error_message,
            message = error_message
        );
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"status": "error","message": error_message})),
        ));
    }
    //check if the api key is already associated with the usage plan
    let is_associated = match check_usage_plan_with_api_key(
        client.clone(),
        usage_plan_id.clone(),
        api_key_id.clone(),
    )
    .await
    {
        Ok(is_associated) => is_associated,
        Err((status_code, json)) => {
            return Err((status_code, json));
        }
    };
    match is_associated {
        true => {
            let success_message = format!(
                "'API key:{}' already associated with the 'usage plan id:{}'.",
                api_key_id, usage_plan_id
            );
            info!(
                app_name = app_name,
                task_id = task_id,
                message = success_message
            );
            return Ok(());
        }
        false => {
            //associate the api key with the usage plan
            match associate_api_key_with_usage_plan(
                client.clone(),
                usage_plan_id.clone(),
                api_key_id.clone(),
                key_type.clone(),
                task_id.clone(),
                app_name.clone(),
            )
            .await
            {
                Ok(_) => {
                    return Ok(());
                }
                Err((status_code, json)) => {
                    return Err((status_code, json));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn test_success_check_usage_plan_with_api_key() {
        let region_provider = RegionProviderChain::default_provider();
        let config = aws_config::defaults(BehaviorVersion::latest())
            .region(region_provider)
            .load()
            .await;
        let client = aws_sdk_apigateway::Client::new(&config);
        let usage_plan_id = "bqpvmk".to_string();
        let api_key_id = "ozlmxq2imj".to_string();
        let result = check_usage_plan_with_api_key(client, usage_plan_id, api_key_id)
            .await
            .unwrap();
        assert_eq!(result, true);
    }
    #[tokio::test]
    async fn test_success_check_usage_plan_with_api_key_failed() {
        let region_provider = RegionProviderChain::default_provider();
        let config = aws_config::defaults(BehaviorVersion::latest())
            .region(region_provider)
            .load()
            .await;
        let client = aws_sdk_apigateway::Client::new(&config);
        let usage_plan_id = "bqpvmk".to_string();
        let api_key_id = "non_existent_api_key".to_string();
        let result = check_usage_plan_with_api_key(client, usage_plan_id, api_key_id)
            .await
            .unwrap();
        assert_eq!(result, false);
    }
    #[tokio::test]
    async fn test_success_associate_api_key_with_usage_plan() {
        let region_provider = RegionProviderChain::default_provider();
        let config = aws_config::defaults(BehaviorVersion::latest())
            .region(region_provider)
            .load()
            .await;
        let client = aws_sdk_apigateway::Client::new(&config);
        let usage_plan_id = "bqpvmk".to_string();
        let api_key_id = "04vlv7l5q3".to_string();
        let key_type = "API_KEY".to_string();
        let task_id = "sample_task_id_unit_test".to_string();
        let app_name = "app100".to_string();
        let result = associate_api_key_with_usage_plan(
            client.clone(),
            usage_plan_id.clone(),
            api_key_id.clone(),
            key_type,
            task_id,
            app_name,
        )
        .await
        .unwrap();
        assert_eq!(result, ());
        let _res = client
            .delete_usage_plan_key()
            .usage_plan_id(usage_plan_id)
            .key_id(api_key_id)
            .send()
            .await
            .unwrap();
    }
    #[tokio::test]
    async fn test_success_associate_api_key_with_usage_plan_failed() {
        let region_provider = RegionProviderChain::default_provider();
        let config = aws_config::defaults(BehaviorVersion::latest())
            .region(region_provider)
            .load()
            .await;
        let client = aws_sdk_apigateway::Client::new(&config);
        let usage_plan_id = "bqpvmk".to_string();
        let api_key_id = "ozlmxq2imj".to_string();
        let key_type = "API_KEY".to_string();
        let task_id = "sample_task_id_unit_test".to_string();
        let app_name = "app100".to_string();
        let result = associate_api_key_with_usage_plan(
            client.clone(),
            usage_plan_id.clone(),
            api_key_id.clone(),
            key_type,
            task_id,
            app_name,
        )
        .await
        .unwrap_err();
        assert_eq!(result.0, StatusCode::INTERNAL_SERVER_ERROR);
    }
    #[tokio::test]
    async fn test_success_check_usage_plan_exists() {
        let region_provider = RegionProviderChain::default_provider();
        let config = aws_config::defaults(BehaviorVersion::latest())
            .region(region_provider)
            .load()
            .await;
        let client = aws_sdk_apigateway::Client::new(&config);
        let usage_plan_id = "bqpvmk".to_string();
        let app_name = "app100".to_string();
        let task_id = "sample_task_id_unit_test".to_string();
        let result = check_usage_plan_exists(client, usage_plan_id, app_name, task_id)
            .await
            .unwrap();
        assert_eq!(result, true);
    }
    #[tokio::test]
    async fn test_success_check_usage_plan_exists_failed() {
        let region_provider = RegionProviderChain::default_provider();
        let config = aws_config::defaults(BehaviorVersion::latest())
            .region(region_provider)
            .load()
            .await;
        let client = aws_sdk_apigateway::Client::new(&config);
        let usage_plan_id = "bqpmk".to_string();
        let app_name = "app100".to_string();
        let task_id = "sample_task_id_unit_test".to_string();
        let result = check_usage_plan_exists(client, usage_plan_id, app_name, task_id)
            .await
            .unwrap_err();
        assert_eq!(result.0, StatusCode::INTERNAL_SERVER_ERROR);
    }
}
