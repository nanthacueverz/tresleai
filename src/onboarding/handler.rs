/*
*  Created Date:  Mar 17, 2024
 * -----
 * Copyright (c) 2024 Tresle.ai or its affiliates. All Rights Reserved.
 */
//! This module contains the POST handler for onboarding/updating an app and calls helper functions to
//! perform operations with DocumentDB and Kafka.
//! The POST handler is used by the onboarding service to onboard/update an app to the product/platform.
//! The handler returns a 201 status code if the app is onboarded/updated successfully.
//! The handler returns a 400 status code if the app already exists or doesn't exist for an update request.
//! The handler returns a 500 status code if an error occurs while performing operations with DocumentDB and Kafka.
//! The handler returns a JSON response with the status, message, api_key, app_id and reference_id.
//!

use crate::admin_ui_api::schema::QueryParams;
use crate::onboarding::create_api_key::create_api_key;
use crate::onboarding::update_api_key_usage::update_api_key_with_usage_plan;
use crate::onboarding::{
    check_connectivity::check_datasource_connectivity,
    check_datasource_change::check_datasource_change, fetch_api_key::fetch_api_key,
    schema::app_onboarding_request::OnboardingRequest, schema::response::*, update_app::update_app,
};
use crate::service::generate_and_insert_document::*;
use crate::service::publish_to_kafka::app_onboard_or_update_notify_kafka;
use crate::service::{check_app_existence::check_app_existence, state::AppState};
use axum::{extract::Query, extract::State, http::StatusCode, response::IntoResponse, Json};
use chrono::{DateTime, Utc};
use serde_json::json;
use std::sync::Arc;
use tracing::{error, info, instrument};
use uuid::Uuid;

#[instrument(skip_all)]
/// Asynchronous function to perform background operations with DocumentDB and Kafka.
#[allow(clippy::too_many_arguments)]
async fn background_tasks(
    app_state: Arc<AppState>,
    body: OnboardingRequest,
    app_id: String,
    api_key: String,
    api_key_id: String,
    reference_id: String,
    task_id: String,
    request_timestamp: DateTime<Utc>,
    is_update: bool,
) {
    // Generate the ID document and insert it in DocumentDB
    let id_document =
        generate_id_document(&body.app_name, reference_id.clone(), task_id.clone()).await;
    if create_document_in_db(
        &app_state,
        &id_document,
        DocType::ID,
        &app_state.app_settings.mongo_db.mongo_db_id_collection,
        &body.app_name,
        &reference_id,
        &task_id,
    )
    .await
    .is_err()
    {
        return;
    };

    // CASE 1: If it's an onboarding request
    // 1. Generate the app document and insert it in DocumentDB.
    // 2. Publish the datasources to Kafka
    if !is_update {
        // has_datasource_changed is set to true for onboarding requests
        let has_datasource_changed = true;
        let app = match generate_app_document(
            &app_state,
            body.clone(),
            app_id,
            api_key,
            api_key_id,
            has_datasource_changed,
        )
        .await
        {
            Ok(app) => app,
            Err(_) => return,
        };
        if create_document_in_db(
            &app_state,
            &app,
            DocType::App,
            &app_state.app_settings.mongo_db.mongo_db_app_collection,
            &body.app_name,
            &reference_id,
            &task_id,
        )
        .await
        .is_err()
        {
            return;
        };
        if app_onboard_or_update_notify_kafka(
            &app_state,
            &body.app_name,
            &body.app_datasource,
            None,
            task_id.clone(),
        )
        .await
        .is_err()
        {
            return;
        };

    // CASE 2: If it's an update request
    // 1. Check if the datasources have changed. If yes, update the app document in DocumentDB and publish both the new and existing datasources to Kafka.
    // 2. If the datasources are identical, just update the app document in DocumentDB (since fields other than datasources may have changed)
    // but don't publish to Kafka.
    } else {
        let (has_datasource_changed, existing_app_datasource) =
            match check_datasource_change(&app_state, &body.app_name, &body.app_datasource).await {
                Ok(result) => result,
                Err(_) => return,
            };
        if update_app(
            &app_state,
            &body,
            app_id,
            api_key,
            api_key_id,
            has_datasource_changed,
        )
        .await
        .is_err()
        {
            return;
        };
        // if the datasources have changed, publish the new datasources to Kafka
        if has_datasource_changed {
            if let Some(existing_app_datasource) = existing_app_datasource {
                if app_onboard_or_update_notify_kafka(
                    &app_state,
                    &body.app_name,
                    &body.app_datasource,
                    Some(&existing_app_datasource),
                    task_id.clone(),
                )
                .await
                .is_err()
                {
                    return;
                };
            }
        }
    }

    // Calculate the time taken to onboard the app
    let onboarding_success_timestamp = Utc::now();
    let onboarding_duration = format!(
        "{} ms",
        (onboarding_success_timestamp - request_timestamp).num_milliseconds()
    );
    let success_message: String = format!("'{}' onboarded/updated successfully.", &body.app_name);

    // Sending data to logs, audit and metrics microservices
    info!(app_name = &body.app_name, message = success_message);
    info!(
        service = "audit_microservice",
        task_id = task_id,
        app_name = &body.app_name,
        action = "App Onboarded/updated",
        details = success_message,
        message = success_message
    );
    info!(
        service = "metric",
        task_id = task_id,
        app_name = &body.app_name,
        metrics_name = "App Onboarding/update Duration",
        metrics_value = onboarding_duration
    );
}

/// POST handler to onboard/update an application to the product/ platform.
#[utoipa::path(
    post,
    path = "/api/v1.1/admin/apps/onboard",
    request_body = OnboardingRequest,
    params(
        (
            "is_update" = inline(Option<String>),
            Query,
            description = "Onboarding or update request.",
        )
    ),
    responses(
        (status = 200, description = "Onboarding/update initiated successfully.", body = [AppCreateResponse]),
        (status = StatusCode::BAD_REQUEST, description = "Invalid Request", body = [ErrorResponse]),
        (status = StatusCode::INTERNAL_SERVER_ERROR, description = "Tresle error occurred. Please save reference id: {} and contact support.")
    )
)]
#[instrument(skip_all)]
pub async fn post_app_onboarding_handler(
    Query(params): Query<QueryParams>,
    State(app_state): State<Arc<AppState>>,
    Json(body): Json<OnboardingRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    let request_timestamp = Utc::now();

    // Check if the app already exists
    let app_exists = check_app_existence(&app_state, &body.app_name).await?;

    // Check if the request is an onboarding request (is_update = false) or an update request (is_update = true)
    let is_update = params.is_update.unwrap_or(false);

    // Check if it's an update request and app doesn't exist. If so, return error
    if is_update && !app_exists {
        let error_message = format!("App '{}' doesn't exist. Cannot update.", &body.app_name);
        error!(ext_message = error_message, message = error_message);
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({"status": "error", "message": error_message})),
        ));

    // Check if it's an onboarding request and app exists. If so, return error
    } else if !is_update && app_exists {
        let error_message = format!("App '{}' already exists. Cannot onboard.", &body.app_name);
        error!(ext_message = error_message, message = error_message);
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({"status": "error", "message": error_message})),
        ));
    }

    // Call to 'Onboarding' - generate the UI summary document and insert it in DocumentDB
    let ui_summary_document = generate_ui_summary_document(
        &body.app_name,
        "Onboarding",
        1,
        request_timestamp.to_string(),
    )
    .await;
    create_document_in_db(
        &app_state,
        &ui_summary_document,
        DocType::UiSummary,
        &app_state
            .app_settings
            .mongo_db
            .mongo_db_ui_summary_collection,
        &body.app_name,
        &"".to_string(),
        &"".to_string(),
    )
    .await
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"status": "error", "message": e.to_string()})),
        )
    })?;

    // Check the connectivity to the provided data sources
    check_datasource_connectivity(&app_state, &body.app_datasource, &body.app_name).await?;

    // If it's an onboarding request, create an API key, else fetch the given app's api key and app_id from DocumentDB
    let (api_key, api_key_id, app_id) = if !is_update {
        let (api_key, api_key_id) = create_api_key(&app_state, &body.app_name).await?;
        let app_id = Uuid::new_v4().to_string();
        (api_key, api_key_id, app_id)
    } else {
        fetch_api_key(&app_state, &body.app_name).await?
    };

    // Generate the app ID, reference ID and task ID
    let random_num: u32 = (rand::random::<u32>() % 90000) + 10000;
    let task_id = format!(
        "{}-{}-{}-{}-{}",
        "TSK", random_num, &body.app_name, "Onboarding", request_timestamp
    );
    let reference_id = Uuid::new_v4().to_string();

    //function to update the usage plan for the api key
    update_api_key_with_usage_plan(
        &app_state,
        api_key_id.clone(),
        task_id.clone(),
        &body.app_name,
    )
    .await?;

    // Instrument function call counter
    info!(
        service = "metric",
        app_name = &body.app_name,
        task_id = task_id,
        metrics_name = "App Onboarding Counter",
        metrics_value = "1"
    );

    // Spawn a background task to perform operations with DocumentDB and Kafka
    tokio::spawn(background_tasks(
        Arc::clone(&app_state),
        body,
        app_id.clone(),
        api_key.clone(),
        api_key_id.clone(),
        reference_id.clone(),
        task_id,
        request_timestamp,
        is_update,
    ));

    Ok((
        StatusCode::CREATED,
        Json(AppCreateResponse {
            status: "success".to_string(),
            message: "Datasource validation done. Onboarding in progress.".to_string(),
            api_key,
            app_id,
            reference_id,
        }),
    ))
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::admin_ui_api::app_delete_handler::tests::test_success_delete_app;
    use crate::onboarding::schema::app_onboarding_request::OnboardingRequest;
    use rand::{distributions::Alphanumeric, Rng}; // 0.8
    use std::fs::File;
    use std::io::Read;
    use std::time::Duration;
    use tokio::runtime::Runtime;

    #[test]
    #[ignore = "to fix"]
    pub fn test_success_post_app_onboarding_handler() {
        let rt = Runtime::new().unwrap();

        rt.block_on(async {
            // Create a dev AppState
            let app_state = crate::tests::test_get_appstate().await.unwrap();

            // Create a mock OnboardingRequest
            let mut file = File::open("src/test/app_config.json").unwrap();
            let mut buff = String::new();
            file.read_to_string(&mut buff).unwrap();

            let mut app_config: OnboardingRequest = serde_json::from_str(&buff).unwrap();

            let rand_string: String = rand::thread_rng()
                .sample_iter(&Alphanumeric)
                .take(7)
                .map(char::from)
                .collect();

            let app_name = format!("facade-app-{}", rand_string).clone();
            app_config.app_name = app_name.clone();

            let mut query_params = QueryParams::default();
            query_params.is_update = Some(false);

            // Call the function
            let result = post_app_onboarding_handler(
                Query(query_params),
                State(app_state),
                axum::Json(app_config),
            )
            .await;

            tokio::time::sleep(Duration::from_secs(2)).await; // wait for the background task to complete  (database writes)
                                                              // clean up
            test_success_delete_app(app_name).await;

            // println!("results:{:?}\n", result.err());
            // Check that the result is Ok
            assert!(result.is_ok());
        });
    }

    #[test]
    #[ignore = "to fix"]
    pub fn test_success_post_app_onboarding_handler_update() {
        let rt = Runtime::new().unwrap();

        rt.block_on(async {
            // Create a dev AppState
            let app_state = crate::tests::test_get_appstate().await.unwrap();

            // Create a mock OnboardingRequest
            let mut file = File::open("src/test/app_config2.json").unwrap();
            let mut buff = String::new();
            file.read_to_string(&mut buff).unwrap();

            let app_config: OnboardingRequest = serde_json::from_str(&buff).unwrap();

            // Call the function
            let mut query_params = QueryParams::default();
            query_params.is_update = Some(true);
            let result = post_app_onboarding_handler(
                Query(query_params),
                State(app_state),
                axum::Json(app_config),
            )
            .await;

            tokio::time::sleep(Duration::from_secs(2)).await; // wait for the background task to complete  (database writes)

            // println!("results:{:?}\n", result.err());
            // Check that the result is Ok
            assert!(result.is_ok());
        });
    }
}
