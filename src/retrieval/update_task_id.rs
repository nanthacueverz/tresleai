/*
 * Created Date:   Feb 23, 2024
 * -----
 * Copyright (c) 2024 Tresle.ai or its affiliates. All Rights Reserved.
 */
//! This module contains the function to update the 'task_id' corresponding to a 'reference_id' once initial
//! retrieval is complete.
//!

use crate::admin_ui_api::schema::UpdateResponse;
use crate::service::error::TresleFacadeCommonError;
use crate::service::state::AppState;
use api_utils::errors::error_interceptor::ErrorInterceptor;
use axum::{response::IntoResponse, Json};
use error_utils::AxumApiError;
use mongodb::bson::doc;
use serde_json::json;
use std::sync::Arc;
use tracing::{info, instrument};

#[instrument(skip_all)]
pub async fn update_task_id(
    app_state: &Arc<AppState>,
    app_name: &String,
    reference_id: &String,
    initial_task_id: &String,
    updated_task_id: &String,
) -> Result<impl IntoResponse, AxumApiError<TresleFacadeCommonError>> {
    let filter = doc! {"reference_id": &reference_id};
    let collection_name = &app_state.app_settings.mongo_db.mongo_db_id_collection;

    let ext_message = app_state.app_settings.general_message.clone();

    // Update the task_id in the app document
    let updated_document = doc! {"app_name": app_name, "task_id": updated_task_id};

    match app_state
        .db
        .update_document(collection_name, filter, updated_document)
        .await
        .map_err(ErrorInterceptor::from)
    {
        Ok(json_result) => {
            let result: UpdateResponse = match serde_json::from_value(json_result) {
                Ok(result) => result,
                Err(e) => {
                    return Err(error_utils::AxumApiError {
                        inner: TresleFacadeCommonError::failed_to_deserialize_update_response(
                            reference_id,
                            initial_task_id,
                            e,
                            &ext_message,
                        ),
                    })
                }
            };
            // Check if the update was successful
            if result.modifiedCount == 0 {
                Err(error_utils::AxumApiError {
                    inner: TresleFacadeCommonError::no_document_found_to_update(
                        reference_id,
                        initial_task_id,
                        &ext_message,
                    ),
                })
            } else {
                let success_message =
                    format!("Task_id updated to '{}' successfully.", updated_task_id);
                info!(app_name = app_name, message = success_message);
                Ok(Json(
                    json!({"status": "success", "message": success_message, "app_name": app_name}),
                ))
            }
        }
        Err(e) => Err(error_utils::AxumApiError {
            inner: TresleFacadeCommonError::failed_to_update_document_in_db(
                reference_id,
                initial_task_id,
                e,
                &ext_message,
            ),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::runtime::Runtime;

    #[test]
    fn test_failure_update_task_id_no_document_found() {
        let rt = Runtime::new().unwrap();

        rt.block_on(async {
            // Create a dev AppState
            let app_state = crate::tests::test_get_appstate().await.unwrap();

            let app_name = "app100".to_string();
            let reference_id = "non_existent_reference_id".to_string();
            let initial_task_id = "initial_task_id".to_string();
            let updated_task_id = "updated_task_id".to_string();

            // Call the function
            let result = update_task_id(
                &app_state,
                &app_name,
                &reference_id,
                &initial_task_id,
                &updated_task_id,
            )
            .await;

            // Check if the function returns an error and the error is of type TaskIdUpdateError
            assert!(result.is_err());
            match result.err().unwrap().inner {
                TresleFacadeCommonError::TaskIdUpdateError { .. } => assert!(true),
                _ => assert!(false, "Expected TaskIdUpdateError"),
            }
        });
    }
}
