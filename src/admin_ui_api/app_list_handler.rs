/*
 * Created Date:   Feb 23, 2024
 * -----
 * Copyright (c) 2024 Tresle.ai or its affiliates. All Rights Reserved.
 */
//! This module contains the GET handler for fetching the list of onboarded apps from DocumentDB.
//! The handler is mounted at `/api/v1.1/admin/apps`.
//! The handler is called by the admin UI to fetch the list of onboarded apps.
//! The handler returns the list of onboarded apps if they exist, else returns an error message.    
//! The handler returns a 200 status code if the list of onboarded apps is fetched successfully.
//! The handler returns a 500 status code if an error occurs while fetching the list of onboarded apps.
//! The handler returns a JSON response with the status and message.
//!

use crate::admin_ui_api::schema::{AppListFetchSchema, QueryParams};
use crate::service::state::AppState;
use api_utils::app_model::App;
use api_utils::errors::error_interceptor::ErrorInterceptor;
use axum::{extract::Query, extract::State, http::StatusCode, response::IntoResponse, Json};
use logging_utils::create_ref_id_helper::create_ref_id;
use logging_utils::create_task_id_helper::create_task_id;
use logging_utils::create_task_ref_id_helper::create_task_ref_collection;
use mongodb::bson::doc;
use serde_json::json;
use std::fmt::Debug;
use std::sync::Arc;
use tracing::{debug, error, instrument};

/// GET handler to fetch the list of apps.
#[utoipa::path(
    get,
    path = "/api/v1.1/admin/apps",
    params(
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
        (status = 200, description = "App List retrieved successfully."),
        (status = StatusCode::BAD_REQUEST, description = "Invalid Request", body = [ErrorResponse]),
        (status = StatusCode::INTERNAL_SERVER_ERROR, description = "Tresle error occurred. Please save reference id: {} and contact support.")
    )
)]
#[instrument(skip_all)]
pub async fn get_app_list(
    Query(params): Query<QueryParams>,
    State(app_state): State<Arc<AppState>>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    let filter = doc! {};
    // TODO: just 100?
    // Extract the page and limit from the query params
    let limit = params.limit.unwrap_or(100) as i64;
    let page = params.page.unwrap_or(1) as i64;

    // Get list of all onboarded apps from DocumentDB
    let collection_name = &app_state.app_settings.mongo_db.mongo_db_app_collection;
    match app_state
        .db
        .get_all_documents(collection_name, limit, page, filter)
        .await
        .map_err(ErrorInterceptor::from)
    {
        Ok(apps) => {
            let mut app_list = Vec::new();
            let mut errors = Vec::new();

            for app in apps {
                match doc_to_type::<App>(app) {
                    // If the app is successfully fetched, add it to the app_list
                    Ok(app_model) => {
                        app_list.push(AppListFetchSchema {
                            app_name: app_model.app_name,
                            app_description: app_model.app_description,
                            api_key: app_model.api_key,
                            onboarding_status: app_model.onboarding_status,
                            search_enabled: app_model.search_enabled,
                        });
                    }
                    // If the app is not fetched due to incorrect schema, add it to the errors list
                    Err(e) => {
                        errors.push(e);
                    }
                };
            }

            // Sort the app_list by ascending order of app_name
            app_list.sort_by(|a, b| a.app_name.to_lowercase().cmp(&b.app_name.to_lowercase()));

            let message;
            if !errors.is_empty() {
                message = format!("{} app(s) fetched successfully. {} app(s) failed to fetch due to incorrect schema. Error: {:?}", app_list.len(),errors.len(), errors);
                debug!(message = message);
            } else {
                message = format!(" {} app(s) fetched successfully.", app_list.len());
                debug!(message = message);
            }
            Ok(Json(
                json!({ "status": "success","message": message,"app_count": app_list.len(),"data": app_list}),
            ))
        }
        Err(e) => {
            let error_message = format!(
                "Failed to fetch list of onboarded apps from DocumentDB. Error: {:?}",
                e
            );
            let ref_id = create_ref_id();
            let service_type = "GetAppList".to_string();
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
            let _ = create_task_ref_collection(
                mongo_url,
                mongo_db_name,
                id_collection,
                app_name.clone(),
                task_id,
                ref_id.clone(),
            )
            .await;
            let ext_message = format!(
                "{} Use reference ID: {}",
                app_state.app_settings.general_message, ref_id
            );
            error!(ext_message = ext_message, message = error_message);
            error!(message = error_message);
            Err(e.intercept_error().await)
        }
    }
}

/// Converts a json value to rust type
fn doc_to_type<T>(doc: serde_json::Value) -> Result<T, (StatusCode, Json<serde_json::Value>)>
where
    T: serde::de::DeserializeOwned + Default + Debug,
{
    let model: Result<T, _> = serde_json::from_value(doc);
    match model {
        Ok(model) => Ok(model),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(
                serde_json::json!({ "status": "error","error":  format!("Failed to convert json to App type. Error: {}", e)}),
            ),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::runtime::Runtime;

    #[test]
    fn test_success_get_app_list() {
        let rt = Runtime::new().unwrap();

        rt.block_on(async {
            // Create a dev AppState
            let app_state = crate::tests::test_get_appstate().await.unwrap();

            // Call the function
            let result = get_app_list(
                Query(QueryParams {
                    page: Some(1),
                    limit: Some(10),
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
    fn test_success_get_app_list_missing_page() {
        let rt = Runtime::new().unwrap();

        rt.block_on(async {
            // Create a dev AppState
            let app_state = crate::tests::test_get_appstate().await.unwrap();

            // Call the function
            let result = get_app_list(
                Query(QueryParams {
                    page: None,
                    limit: Some(10),
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
    fn test_success_get_app_list_missing_limit() {
        let rt = Runtime::new().unwrap();

        rt.block_on(async {
            // Create a dev AppState
            let app_state = crate::tests::test_get_appstate().await.unwrap();

            // Call the function
            let result = get_app_list(
                Query(QueryParams {
                    page: Some(1),
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

    /*  todo : fix this test
    #[test]
    fn test_failure_get_app_list() {
        let rt = Runtime::new().unwrap();

        rt.block_on(async {
            // Create a dev AppState
            let app_state = crate::tests::test_get_appstate().await.unwrap();

            // Call the function
            let result = get_app_list(Query(QueryParams{page: Some(1), limit: Some(10), app_name: None, is_update: None, search_enabled: None}), State(app_state.clone())).await;

            // If the function returns Err, check the status code and message
            let (status_code, Json(message)) = result.err().unwrap();
            if status_code == StatusCode::INTERNAL_SERVER_ERROR {

                assert_eq!(message.get("status").unwrap().as_str().unwrap(), "error");
                assert!(message.get("message").unwrap().as_str().unwrap().contains("Failed to fetch list of onboarded apps from DocumentDB."));
            } else {
                assert!(true) // no bad data ... everthing is good
            }
       });
    }
    */
}
