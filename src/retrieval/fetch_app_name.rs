/*
*  Created Date:  Mar 17, 2024
 * -----
 * Copyright (c) 2024 Tresle.ai or its affiliates. All Rights Reserved.
 */
//! This module contains the function to fetch app name from DocumentDB corresponding to the input API key
//! during the information retrieval process.
//!
//!

use crate::service::error::TresleFacadeCommonError;
use crate::service::state::AppState;
use error_utils::AxumApiError;
use mongodb::bson::doc;
use std::sync::Arc;
use tracing::{info, instrument};

/// Asynchronous function to fetch the app name from an input API key.
#[instrument(skip_all)]
pub async fn fetch_app_name(
    app_state: &Arc<AppState>,
    api_key: &String,
    task_id: &String,
    reference_id: &String,
) -> Result<String, AxumApiError<TresleFacadeCommonError>> {
    let filter = doc! {"api_key": api_key};
    let collection_name = &app_state.app_settings.mongo_db.mongo_db_app_collection;
    let ext_message = app_state.app_settings.general_message.clone();

    match app_state
        .db
        .get_document(collection_name, filter)
        .await
        .map_err(|e| {
            TresleFacadeCommonError::failed_to_fetch_app_name_from_db(
                reference_id,
                task_id,
                e,
                &ext_message,
            )
        }) {
        Ok(Some(response)) => {
            if let Some(app_name) = response
                .get("app_name")
                .and_then(|app_name| app_name.as_str())
            {
                let success_message =
                    "App name fetched successfully for given api_key.".to_string();
                info!(app_name = app_name, message = success_message);
                Ok(app_name.to_string())
            } else {
                Err(error_utils::AxumApiError {
                    inner: TresleFacadeCommonError::no_app_name_key_found(
                        reference_id,
                        task_id,
                        &ext_message,
                    ),
                })
            }
        }
        Ok(None) => Err(error_utils::AxumApiError {
            inner: TresleFacadeCommonError::no_app_name_found_for_given_api_key(
                reference_id,
                task_id,
                &ext_message,
            ),
        }),
        Err(e) => Err(error_utils::AxumApiError {
            inner: TresleFacadeCommonError::failed_to_fetch_app_name_from_db(
                reference_id,
                task_id,
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
    fn test_success_fetch_app_name() {
        let rt = Runtime::new().unwrap();

        rt.block_on(async {
            // Create a dev AppState
            let app_state = crate::tests::test_get_appstate().await.unwrap();

            // Call the function
            let result = fetch_app_name(
                &app_state,
                &"1ytmOsUYKI2ZGg7WzzSfH3YU87i6UtZ50uMgVCc5".to_string(),
                &"sample_task_id_unit_test".to_string(),
                &"sample_reference_id_unit_test".to_string(),
            )
            .await;

            // If the function returns Ok, check the app_name
            assert!(result.is_ok());
            let app_name = result.unwrap();
            assert_eq!(app_name, "app100");
        });
    }

    #[test]
    fn test_failure_fetch_app_name_no_document_found() {
        let rt = Runtime::new().unwrap();

        rt.block_on(async {
            // Create a dev AppState
            let app_state = crate::tests::test_get_appstate().await.unwrap();

            // Call the function
            let result = fetch_app_name(
                &app_state,
                &"non_existent_api_key".to_string(),
                &"sample_task_id_unit_test".to_string(),
                &"sample_reference_id_unit_test".to_string(),
            )
            .await;

            // Check if the function returns an error and the error is of type FetchAppNameError
            assert!(result.is_err());
            match result.err().unwrap().inner {
                TresleFacadeCommonError::FetchAppNameError { .. } => assert!(true),
                _ => assert!(false, "Expected FetchAppNameError"),
            }
        });
    }
}
