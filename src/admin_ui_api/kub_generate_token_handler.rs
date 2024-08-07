/*
 * Created Date:   Feb 23, 2024
 * -----
 * Copyright (c) 2024 Tresle.ai or its affiliates. All Rights Reserved.
 */
//! This module contains the GET handler to generate a token to login into kubernetes dashboard.
//! The handler is used by the admin UI to generate a token to login into kubernetes dashboard.
//! The handler is mounted at `/api/v1.1/admin/token`.
//! The handler returns the token if it exists, else returns an error message.
//! The handler returns a 200 status code if the token is generated successfully.
//! The handler returns a 400 status code if an error occurs while generating the token.
//! The handler returns a 500 status code if an error occurs while generating the token.
//! The handler returns a JSON response with the status and message.
//!

use crate::service::state::AppState;
use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use k8s_openapi::api::core::v1::Secret;
use kube::{api::Api, Client};
use logging_utils::create_ref_id_helper::create_ref_id;
use logging_utils::create_task_id_helper::create_task_id;
use logging_utils::create_task_ref_id_helper::create_task_ref_collection;
use serde_json::json;
use std::str;
use std::sync::Arc;
use tracing::{debug, error, instrument};

/// GET handler to generate a token to login into kubernetes dashboard.
#[utoipa::path(
    get,
    path = "/api/v1.1/admin/token",
    responses(
        (status = 200, description = "Token generated succesfully."),
        (status = StatusCode::BAD_REQUEST, description = "Invalid Request", body = [ErrorResponse]),
        (status = StatusCode::INTERNAL_SERVER_ERROR, description = "Tresle error occurred. Please save reference id: {} and contact support.")
    )
)]
#[instrument(skip_all)]
pub async fn get_kubernetes_token(
    State(app_state): State<Arc<AppState>>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    // Create a reference ID ,task ID and initialize the documentdb variables
    let ref_id = create_ref_id();
    let service_type = "GetKubToken".to_string();
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

    // Create a kubernetes client
    let client = match Client::try_default().await {
        Ok(client) => client,
        Err(_) => {
            let error_message = "Failed to create Kubernetes client.";
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
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": error_message })),
            ));
        }
    };

    let namespace = &app_state.app_settings.kubernetes.namespace;

    // Create an API object for secrets in the specified namespace
    let secrets: Api<Secret> = Api::namespaced(client, namespace);

    // Fetch the required secret
    let secret_name = &app_state.app_settings.kubernetes.secret_name;
    match secrets.get(secret_name).await {
        Ok(secret) => {
            // Once secret found, extract the token from it
            match secret.data.as_ref().and_then(|map| map.get("token")) {
                Some(token) => {
                    let token_vec = token.0.to_vec();
                    match String::from_utf8(token_vec) {
                        Ok(token_str) => {
                            let success_message = "Token generated successfully.";
                            debug!(message = success_message);
                            Ok(Json(json!({"status": "success", "token": token_str })))
                        }
                        Err(_) => {
                            let error_message = "Failed to convert kubernetes token to string.";
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
                            Err((
                                StatusCode::INTERNAL_SERVER_ERROR,
                                Json(json!({"status": "error", "message": error_message})),
                            ))
                        }
                    }
                }
                None => {
                    let error_message =
                        format!("Failed to find 'token' key in '{}' secret.", secret_name);
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
                    Err((
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(json!({"status": "error", "message": error_message})),
                    ))
                }
            }
        }
        Err(_) => {
            let error_message = format!("Failed to find '{}' secret.", secret_name);
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
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"status": "error", "message": error_message})),
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::runtime::Runtime;

    #[test]
    #[ignore = "This test is ignored because it requires a kubernetes cluster to be running."]
    fn test_success_get_kubernetes_token() {
        let rt = Runtime::new().unwrap();

        rt.block_on(async {
            // Create a dev AppState
            let app_state = crate::tests::test_get_appstate().await.unwrap();

            // Call the function
            let result = get_kubernetes_token(State(app_state)).await;

            // Check if the function returns Ok
            assert!(result.is_ok());
        });
    }
}
