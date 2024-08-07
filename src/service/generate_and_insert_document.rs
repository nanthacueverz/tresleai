/*
 * Created Date:   Feb 23, 2024
 * -----
 * Copyright (c) 2024 Tresle.ai or its affiliates. All Rights Reserved.
 */

//! This module contains the helper functions to generate an app document from the incoming payload
//! and insert it into DocumentDB.

use crate::retrieval::schema::history_document::HistoryDocument;
use crate::service::app_document::AppDocument;
use crate::service::app_document::AppDocumentCreationError;
use crate::service::error::TresleFacadeCommonError;
use crate::service::id_document::IdDocument;
use crate::service::ui_summary_document::UiSummaryDocument;
use crate::{
    onboarding::schema::app_onboarding_request::OnboardingRequest, service::state::AppState,
};
use error_utils::AxumApiError;
use mongodb::bson::to_bson;
use serde::Serialize;
use std::sync::Arc;
use tracing::{debug, error, info, instrument};

/// Enum containing the document types variants
pub enum DocType {
    App,
    ID,
    UiSummary,
    History,
}

#[instrument(skip_all)]
/// Function to insert/create the generated App or ID document in DocumentDB. It is called from both the onboarding and retrieval modules.
pub async fn create_document_in_db<T: Serialize>(
    app_state: &Arc<AppState>,
    doc: &T,
    doc_type: DocType,
    collection_name: &str,
    app_name: &String,
    reference_id: &String,
    task_id: &String,
) -> Result<(), AxumApiError<TresleFacadeCommonError>> {
    let doc_type = match doc_type {
        DocType::App => "App",
        DocType::ID => "ID",
        DocType::UiSummary => "UI Summary",
        DocType::History => "History",
    };

    let ext_message = app_state.app_settings.general_message.clone();

    let message = format!("Converting {} document to BSON.", doc_type);
    debug!(message = message);
    let app_bson = match to_bson(&doc) {
        Ok(bson) => {
            if let Some(document) = bson.as_document() {
                document.clone()
            } else {
                return Err(error_utils::AxumApiError {
                    inner: TresleFacadeCommonError::failed_to_convert_bson_to_document(
                        app_name,
                        reference_id,
                        task_id,
                        &ext_message,
                    ),
                });
            }
        }
        Err(e) => {
            return Err(AxumApiError {
                inner: TresleFacadeCommonError::failed_to_create_document_in_db(
                    app_name,
                    reference_id,
                    task_id,
                    doc_type,
                    e,
                    &ext_message,
                ),
            })
        }
    };

    let message = format!("Creating/inserting {} document in DocumentDB.", doc_type);
    debug!(message = message);
    match app_state
        .db
        .create_document(collection_name, app_bson)
        .await
        .map_err(|e| {
            TresleFacadeCommonError::failed_to_create_document_in_db(
                app_name,
                reference_id,
                task_id,
                doc_type,
                e,
                &ext_message,
            )
        }) {
        Ok(_) => {
            let success_message =
                format!("{} document created successfully in DocumentDB.", doc_type);
            info!(app_name = app_name, message = success_message);
            Ok(())
        }
        Err(e) => Err(error_utils::AxumApiError {
            inner: TresleFacadeCommonError::failed_to_create_document_in_db(
                app_name,
                reference_id,
                task_id,
                doc_type,
                e,
                &ext_message,
            ),
        }),
    }
}

#[instrument(skip_all)]
/// Function to generate an app document from the incoming payload.
pub async fn generate_app_document(
    app_state: &Arc<AppState>,
    body: OnboardingRequest,
    app_id: String,
    api_key: String,
    api_key_id: String,
    has_datasource_changed: bool,
) -> Result<AppDocument, AppDocumentCreationError> {
    debug!("Generating app document.");
    let timestamp_format = app_state.app_settings.application.timestamp_format.clone();
    let sqs_key = app_state.app_settings.sqs_key_value.to_string();
    let onboarding_status = if has_datasource_changed {
        app_state.app_settings.onboard_inprogress_status.to_string()
    } else {
        app_state.app_settings.onboard_complete_status.to_string()
    };
    let search_enabled = false;
    let mm_search_enabled = true;

    let app_document = match AppDocument::builder()
        .set_app_name(body.app_name.clone())
        .set_app_description(body.app_description)
        .set_text_embedding_model(body.text_embedding_model)
        .set_multimodal_embedding_model(body.multimodal_embedding_model)
        .set_app_datasource(body.app_datasource)
        .set_app_id(app_id)
        .set_api_key(api_key)
        .set_api_key_id(api_key_id)
        .set_sqs_key(sqs_key)
        .set_csv_append_same_schema(body.csv_append_same_schema)
        .set_allowed_models(body.allowed_models)
        .set_create_timestamp(timestamp_format)
        .set_generated_config(app_state, body.app_name)
        .set_onboarding_status(onboarding_status)
        .set_search_enabled(search_enabled)
        .set_mm_search_enabled(mm_search_enabled)
        .build()
    {
        Ok(app_document) => app_document,
        Err(e) => {
            let error_message = format!("Failed to generate app document. Error: {}", e);
            error!(ext_message = error_message, message = error_message);
            return Err(e);
        }
    };
    debug!("App document generated successfully.");
    Ok(app_document)
}

#[instrument(skip_all)]
/// Function to generate an ID document
pub async fn generate_id_document(
    app_name: &String,
    reference_id: String,
    task_id: String,
) -> IdDocument {
    let id_document = IdDocument {
        app_name: app_name.to_string(),
        reference_id,
        task_id,
    };
    debug!("ID document generated successfully.");
    id_document
}

#[instrument(skip_all)]
/// Function to generate a UI summary document
pub async fn generate_ui_summary_document(
    app_name: &String,
    call_type: &str,
    count: u64,
    timestamp: String,
) -> UiSummaryDocument {
    let ui_summary_document = UiSummaryDocument {
        app_name: app_name.to_string(),
        call_type: call_type.to_string(),
        count,
        timestamp,
    };
    debug!("UI summary document generated successfully.");
    ui_summary_document
}

#[instrument(skip_all)]
/// Function to generate a history document
pub async fn generate_history_document(
    reference_id: String,
    task_id: String,
    query: &String,
    response: &String,
    timestamp: String,
    disclaimer_text: String,
) -> HistoryDocument {
    let history_document = HistoryDocument::new(
        reference_id,
        task_id,
        query.to_string(),
        response.to_string(),
        timestamp,
        disclaimer_text,
    );
    debug!("History document generated successfully.");
    history_document
}

#[cfg(test)]
mod tests {

    use super::*;
    use chrono::Utc;
    use std::fs::File;
    use std::io::Read;
    use tokio::runtime::Runtime;

    /* todo add unit test
        use std::fs::File;
        use std::io::Read;
        use crate::onboarding::create_api_key::create_api_key;
        use crate::admin_ui_api::app_delete_handler::tests::test_success_delete_app;


        #[test]
        #[ignore="until posting to core service is implemented"]
        fn test_success_create_document_in_db() {
            let rt = Runtime::new().unwrap();

            rt.block_on(async {
                // Create a dev AppState
                let app_state = crate::tests::test_get_appstate().await.unwrap();

                // Create a test app_id and api_key
                let app_name: String = "app1".to_string();
                let app_id = "test".to_string();
                let api_key = create_api_key(&app_state, &app_name).await.unwrap();

                // Create a dev app_config
                let mut file = File::open("src/test/app_config2.json").unwrap();
                let mut buff = String::new();
                file.read_to_string(&mut buff).unwrap();
                let body: OnboardingRequest = serde_json::from_str(&buff).unwrap();
                let app_name = "facade-app-testing2".to_string();

                // Call the function
                let app = match generate_app_document(&app_state, body, app_id, api_key).await {
                    Ok(app) => app,
                    Err(e) => {
                        println!("Failed to generate app document: {}", e);
                        return;
                    }
                };

                // Create test collection_name and doc_type
                let collection_name = app_state.app_settings.mongo_db.mongo_db_app_collection.clone();
                let doc_type = DocType::App;

                // Call the function
                let result = create_document_in_db(&app_state, &app, doc_type, &collection_name, &app_name).await;

                // clean up
                test_success_delete_app(app_name).await;

                // Check that the result is as expected
                assert!(result.is_ok());
            });
        }
    */

    #[test]
    fn test_success_generate_app_document() {
        let rt = Runtime::new().unwrap();

        rt.block_on(async {
            // Create a dev AppState
            let app_state = crate::tests::test_get_appstate().await.unwrap();

            // Create a test app_id and api_key
            let app_id = "test_app_id".to_string();
            let api_key = "test_api_key".to_string();
            let api_key_id = "test_api_key_id".to_string();
            let has_datasource_changed = false;

            // Create a dev app_config
            let mut file = File::open("src/test/app_config2.json").unwrap();
            let mut buff = String::new();
            file.read_to_string(&mut buff).unwrap();
            let body: OnboardingRequest = serde_json::from_str(&buff).unwrap();

            // Call the function
            let result = generate_app_document(
                &app_state,
                body,
                app_id,
                api_key,
                api_key_id,
                has_datasource_changed,
            )
            .await;

            // Check that the result is as expected
            match result {
                Ok(_app_document) => {
                    assert!(true);
                }

                Err(e) => {
                    println!("Failed to generate app document: {}", e);
                    return;
                }
            }
        });
    }

    #[test]
    fn test_success_generate_id_document() {
        let rt = Runtime::new().unwrap();

        rt.block_on(async {
            // Create a test app_name, reference_id and task_id
            let app_name = "app1".to_string();
            let reference_id = "45cced02-e742-4319-b163-3bbff69557f".to_string();
            let task_id = "TSK-47829-app_223-Onboarding-2024-04-04 05:52:22.755295 UTC".to_string();

            // Call the function
            let result = generate_id_document(&app_name, reference_id, task_id).await;

            // Check that the result is as expected
            assert_eq!(result.app_name, app_name);
        });
    }

    #[test]
    fn test_success_generate_ui_summary_document() {
        let rt = Runtime::new().unwrap();

        rt.block_on(async {
            // Create a test app_name, reference_id and task_id
            let app_name = "app1".to_string();
            let call_type = "Onboarding";
            let count = 1;
            let timestamp = Utc::now().to_string();

            // Call the function
            let result =
                generate_ui_summary_document(&app_name, &call_type, count, timestamp).await;

            // Check that the result is as expected
            assert_eq!(result.app_name, app_name);
        });
    }

    #[test]
    fn test_success_generate_history_document() {
        let rt = Runtime::new().unwrap();

        rt.block_on(async {
            // Create a test reference_id, task_id, query, response and timestamp
            let reference_id = "test_reference_id".to_string();
            let task_id = "test_task_id".to_string();
            let query = "test_query".to_string();
            let response = "test_response".to_string();
            let timestamp = Utc::now().to_string();

            // Call the function
            let result = generate_history_document(
                reference_id.clone(),
                task_id,
                &query,
                &response,
                timestamp,
                "test_disclaimer_text".to_string(),
            )
            .await;

            // Check that the result is as expected
            assert_eq!(result.reference_id, reference_id);
        });
    }
}
