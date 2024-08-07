/*
 * Created Date: Friday February 16th 2024
 * -----
 * Copyright (c) 2024 Tresle.ai or its affiliates. All Rights Reserved.
 */
//! This module contains error handling functions for the retrieval module. The external errors are sent to user and
//! internal errors are persisted in DocumentDB through the logging microservice.

use axum::http::StatusCode;
use chrono::Utc;
use error_utils::TresleAppError;
use std::error::Error as StdError;
use tracing::{debug, error};

#[derive(thiserror::Error, Debug)]
pub enum TresleFacadeCommonError {
    #[error("{ext_message}")]
    RouteNotFound {
        task_id: String,
        time_stamp: String,
        error_code: StatusCode,
        source: Box<dyn StdError + Send + Sync + 'static>,
        reference_id: String,
        ext_message: String,
    },
    #[error("{ext_message}")]
    ApiKeyError {
        time_stamp: String,
        error_code: StatusCode,
        reference_id: String,
        ext_message: String,
    },
    #[error("{ext_message}")]
    FetchAppNameError {
        time_stamp: String,
        error_code: StatusCode,
        reference_id: String,
        ext_message: String,
    },
    #[error("{ext_message}")]
    RetrievalRequestBodyError {
        time_stamp: String,
        error_code: StatusCode,
        reference_id: String,
        ext_message: String,
    },
    #[error("{ext_message}")]
    DocumentCreationError {
        time_stamp: String,
        error_code: StatusCode,
        reference_id: String,
        ext_message: String,
    },
    // #[error("{ext_message}")]
    // PolicyValidationError {
    //     time_stamp: String,
    //     error_code: StatusCode,
    //     reference_id: String,
    //     ext_message: String,
    // },
    #[error("{ext_message}")]
    HistoryDocRetrievalInProgress {
        time_stamp: String,
        error_code: StatusCode,
        reference_id: String,
        ext_message: String,
    },
    #[error("{ext_message}")]
    HistoryDocRetrievalError {
        time_stamp: String,
        error_code: StatusCode,
        reference_id: String,
        ext_message: String,
    },
    #[error("{ext_message}")]
    TaskIdUpdateError {
        time_stamp: String,
        error_code: StatusCode,
        reference_id: String,
        ext_message: String,
    },
}

impl TresleFacadeCommonError {
    #[tracing::instrument(skip_all)]
    pub fn missing_api_key(reference_id: &String, task_id: &String, ext_message: &String) -> Self {
        let ext_message = format!("{} Use reference ID: {}", ext_message, reference_id);
        error!(
            task_id = task_id,
            ext_message = ext_message,
            "x-api-key header is missing."
        );
        let time_stamp = Utc::now().to_rfc3339();
        TresleFacadeCommonError::ApiKeyError {
            time_stamp,
            error_code: StatusCode::BAD_REQUEST,
            reference_id: reference_id.to_string(),
            ext_message: ext_message.to_string(),
        }
    }

    #[tracing::instrument(skip_all)]
    pub fn invalid_api_key(reference_id: &String, task_id: &String, ext_message: &String) -> Self {
        let ext_message = format!("{} Use reference ID: {}", ext_message, reference_id);
        error!(
            task_id = task_id,
            ext_message = ext_message,
            "Invalid value for 'x-api-key' header."
        );
        let time_stamp = Utc::now().to_rfc3339();
        TresleFacadeCommonError::ApiKeyError {
            time_stamp,
            error_code: StatusCode::BAD_REQUEST,
            reference_id: reference_id.to_string(),
            ext_message: ext_message.to_string(),
        }
    }

    #[tracing::instrument(skip_all)]
    pub fn failed_to_fetch_app_name_from_db(
        reference_id: &String,
        task_id: &String,
        e: impl StdError,
        ext_message: &String,
    ) -> Self {
        let ext_message = format!("{} Use reference ID: {}", ext_message, reference_id);
        let internal_message = format!("Failed to fetch app name from DocumentDB. Error: {}", e);
        error!(
            task_id = task_id,
            ext_message = ext_message,
            message = &internal_message
        );
        let time_stamp = Utc::now().to_rfc3339();
        TresleFacadeCommonError::FetchAppNameError {
            time_stamp,
            error_code: StatusCode::INTERNAL_SERVER_ERROR,
            reference_id: reference_id.to_string(),
            ext_message: ext_message.to_string(),
        }
    }

    #[tracing::instrument(skip_all)]
    pub fn no_app_name_key_found(
        reference_id: &String,
        task_id: &String,
        ext_message: &String,
    ) -> Self {
        let ext_message = format!("{} Use reference ID: {}", ext_message, reference_id);
        error!(
            task_id = task_id,
            ext_message = ext_message,
            "Failed to fetch app_name. No app_name key found in document."
        );
        let time_stamp = Utc::now().to_rfc3339();
        TresleFacadeCommonError::FetchAppNameError {
            time_stamp,
            error_code: StatusCode::NOT_FOUND,
            reference_id: reference_id.to_string(),
            ext_message: ext_message.to_string(),
        }
    }

    #[tracing::instrument(skip_all)]
    pub fn no_app_name_found_for_given_api_key(
        reference_id: &String,
        task_id: &String,
        ext_message: &String,
    ) -> Self {
        let ext_message = format!("{} Use reference ID: {}", ext_message, reference_id);
        error!(
            task_id = task_id,
            ext_message = ext_message,
            "No document found for the given api_key."
        );
        let time_stamp = Utc::now().to_rfc3339();
        TresleFacadeCommonError::FetchAppNameError {
            time_stamp,
            error_code: StatusCode::NOT_FOUND,
            reference_id: reference_id.to_string(),
            ext_message: ext_message.to_string(),
        }
    }

    #[tracing::instrument(skip_all)]
    pub fn failed_to_read_retrieval_request_body(
        reference_id: &String,
        task_id: &String,
        ext_message: &String,
    ) -> Self {
        let ext_message = format!("{} Use reference ID: {}", ext_message, reference_id);
        error!(
            task_id = task_id,
            ext_message = ext_message,
            "Failed to read request body."
        );
        let time_stamp = Utc::now().to_rfc3339();
        TresleFacadeCommonError::RetrievalRequestBodyError {
            time_stamp,
            error_code: StatusCode::BAD_REQUEST,
            reference_id: reference_id.to_string(),
            ext_message: ext_message.to_string(),
        }
    }

    #[tracing::instrument(skip_all)]
    pub fn failed_to_parse_retrieval_request_body(
        reference_id: &String,
        task_id: &String,
        e: impl StdError,
        ext_message: &String,
    ) -> Self {
        let ext_message = format!("{} Use reference ID: {}", ext_message, reference_id);
        let internal_message = format!("Failed to parse request body: {}", e);
        error!(
            task_id = task_id,
            ext_message = ext_message,
            message = internal_message
        );
        let time_stamp = Utc::now().to_rfc3339();
        TresleFacadeCommonError::RetrievalRequestBodyError {
            time_stamp,
            error_code: StatusCode::BAD_REQUEST,
            reference_id: reference_id.to_string(),
            ext_message: ext_message.to_string(),
        }
    }

    #[tracing::instrument(skip_all)]
    pub fn failed_to_create_document_in_db(
        app_name: &String,
        reference_id: &String,
        task_id: &String,
        doc_type: &str,
        e: impl StdError,
        ext_message: &String,
    ) -> Self {
        let ext_message = format!("{} Use reference ID: {}", ext_message, reference_id);
        let internal_message = format!(
            "Failed to create {} document in DocumentDB. Error: {}",
            doc_type, e
        );
        error!(
            app_name = app_name,
            task_id = task_id,
            ext_message = ext_message,
            message = internal_message
        );
        let time_stamp = Utc::now().to_rfc3339();
        TresleFacadeCommonError::DocumentCreationError {
            time_stamp,
            error_code: StatusCode::INTERNAL_SERVER_ERROR,
            reference_id: reference_id.to_string(),
            ext_message: ext_message.to_string(),
        }
    }

    #[tracing::instrument(skip_all)]
    pub fn failed_to_convert_bson_to_document(
        app_name: &String,
        reference_id: &String,
        task_id: &String,
        ext_message: &String,
    ) -> Self {
        let ext_message = format!("{} Use reference ID: {}", ext_message, reference_id);
        let internal_message = "Failed to convert BSON to Document.".to_string();
        error!(
            app_name = app_name,
            task_id = task_id,
            ext_message = ext_message,
            message = internal_message
        );
        let time_stamp = Utc::now().to_rfc3339();
        TresleFacadeCommonError::DocumentCreationError {
            time_stamp,
            error_code: StatusCode::INTERNAL_SERVER_ERROR,
            reference_id: reference_id.to_string(),
            ext_message: ext_message.to_string(),
        }
    }

    // #[tracing::instrument(skip_all)]
    // pub fn failed_to_validate_iam_policies(
    //     app_name: &String,
    //     reference_id: &String,
    //     task_id: &String,
    //     unvalidated_policies: String,
    //     ext_message: &String,
    // ) -> Self {
    //     let ext_message = format!("{} Use reference ID: {}", ext_message, reference_id);
    //     let internal_message = format!(
    //         "Failed to validate IAM policies. Error: {}",
    //         unvalidated_policies
    //     );
    //     error!(
    //         app_name = app_name,
    //         task_id = task_id,
    //         ext_message = ext_message,
    //         message = internal_message
    //     );
    //     let time_stamp = Utc::now().to_rfc3339();
    //     TresleFacadeCommonError::PolicyValidationError {
    //         time_stamp,
    //         error_code: StatusCode::BAD_REQUEST,
    //         reference_id: reference_id.to_string(),
    //         ext_message: ext_message.to_string(),
    //     }
    // }

    #[tracing::instrument(skip_all)]
    pub fn no_history_document_found_but_request_accepted(
        app_name: &String,
        reference_id_query_param: &String,
        reference_id: &String,
        task_id: &String,
        ext_message: &String,
    ) -> Self {
        //let ext_message = format!("{} Use reference ID: {}", ext_message, reference_id);
        let internal_message = format!(
            "History document request in progress for the reference ID: '{}' and app_name: '{}'.",
            reference_id_query_param, app_name
        );
        debug!(
            task_id = task_id,
            ext_message = ext_message,
            message = internal_message
        );
        let time_stamp = Utc::now().to_rfc3339();
        TresleFacadeCommonError::HistoryDocRetrievalInProgress {
            time_stamp,
            error_code: StatusCode::ACCEPTED,
            reference_id: reference_id.to_string(),
            ext_message: ext_message.to_string(),
        }
    }

    #[tracing::instrument(skip_all)]
    pub fn failed_to_retrieve_history_document(
        app_name: &String,
        reference_id_query_param: &String,
        reference_id: &String,
        task_id: &String,
        e: impl StdError,
        ext_message: &String,
    ) -> Self {
        let ext_message = format!("{} Use reference ID: {}", ext_message, reference_id);
        let internal_message = format!(
            "Failed to retrieve history document with reference ID: '{}'. Error: {}",
            reference_id_query_param, e
        );
        error!(
            app_name = app_name,
            task_id = task_id,
            ext_message = ext_message,
            message = internal_message
        );
        let time_stamp = Utc::now().to_rfc3339();
        TresleFacadeCommonError::HistoryDocRetrievalError {
            time_stamp,
            error_code: StatusCode::INTERNAL_SERVER_ERROR,
            reference_id: reference_id.to_string(),
            ext_message: ext_message.to_string(),
        }
    }

    #[tracing::instrument(skip_all)]
    pub fn missing_reference_id_in_history_retrieval_request(
        reference_id: &String,
        task_id: &String,
        ext_message: &String,
    ) -> Self {
        let ext_message = format!("{} Use reference ID: {}", ext_message, reference_id);
        error!(
            task_id = task_id,
            ext_message = ext_message,
            "Reference ID is missing in history retrieval request."
        );
        let time_stamp = Utc::now().to_rfc3339();
        TresleFacadeCommonError::HistoryDocRetrievalError {
            time_stamp,
            error_code: StatusCode::BAD_REQUEST,
            reference_id: reference_id.to_string(),
            ext_message: ext_message.to_string(),
        }
    }

    #[tracing::instrument(skip_all)]
    pub fn failed_to_update_document_in_db(
        reference_id: &String,
        task_id: &String,
        e: impl StdError,
        ext_message: &String,
    ) -> Self {
        let ext_message = format!("{} Use reference ID: {}", ext_message, reference_id);
        let internal_message = format!(
            "Failed to update task_id for reference ID '{}'. Error: {}",
            reference_id, e
        );
        error!(
            task_id = task_id,
            ext_message = ext_message,
            message = internal_message
        );
        let time_stamp = Utc::now().to_rfc3339();
        TresleFacadeCommonError::TaskIdUpdateError {
            time_stamp,
            error_code: StatusCode::INTERNAL_SERVER_ERROR,
            reference_id: reference_id.to_string(),
            ext_message: ext_message.to_string(),
        }
    }

    #[tracing::instrument(skip_all)]
    pub fn no_document_found_to_update(
        reference_id: &String,
        task_id: &String,
        ext_message: &String,
    ) -> Self {
        let ext_message = format!("{} Use reference ID: {}", ext_message, reference_id);
        let internal_message = format!(
            "Task_id failed to update. No document found with reference ID '{}'.",
            reference_id
        );
        error!(
            task_id = task_id,
            ext_message = ext_message,
            message = internal_message
        );
        let time_stamp = Utc::now().to_rfc3339();
        TresleFacadeCommonError::TaskIdUpdateError {
            time_stamp,
            error_code: StatusCode::NOT_FOUND,
            reference_id: reference_id.to_string(),
            ext_message: ext_message.to_string(),
        }
    }

    #[tracing::instrument(skip_all)]
    pub fn failed_to_deserialize_update_response(
        reference_id: &String,
        task_id: &String,
        e: impl StdError,
        ext_message: &String,
    ) -> Self {
        let ext_message = format!("{} Use reference ID: {}", ext_message, reference_id);
        let internal_message = format!(
            "Failed to deserialize update response after task_id update. Error: {}",
            e
        );
        error!(
            task_id = task_id,
            ext_message = ext_message,
            message = internal_message
        );
        let time_stamp = Utc::now().to_rfc3339();
        TresleFacadeCommonError::TaskIdUpdateError {
            time_stamp,
            error_code: StatusCode::INTERNAL_SERVER_ERROR,
            reference_id: reference_id.to_string(),
            ext_message: ext_message.to_string(),
        }
    }
}

impl TresleAppError for TresleFacadeCommonError {
    fn error_response(&self) -> error_utils::ApiErrorResponse {
        let (error_code, reference_id) = match self {
            TresleFacadeCommonError::RouteNotFound {
                error_code,
                reference_id,
                ..
            } => (*error_code, reference_id),
            TresleFacadeCommonError::ApiKeyError {
                error_code,
                reference_id,
                ..
            } => (*error_code, reference_id),
            TresleFacadeCommonError::FetchAppNameError {
                error_code,
                reference_id,
                ..
            } => (*error_code, reference_id),
            TresleFacadeCommonError::RetrievalRequestBodyError {
                error_code,
                reference_id,
                ..
            } => (*error_code, reference_id),
            TresleFacadeCommonError::DocumentCreationError {
                error_code,
                reference_id,
                ..
            } => (*error_code, reference_id),
            // TresleFacadeCommonError::PolicyValidationError {
            //     error_code,
            //     reference_id,
            //     ..
            // } => (*error_code, reference_id),
            TresleFacadeCommonError::HistoryDocRetrievalInProgress {
                error_code,
                reference_id,
                ..
            } => (*error_code, reference_id),
            TresleFacadeCommonError::HistoryDocRetrievalError {
                error_code,
                reference_id,
                ..
            } => (*error_code, reference_id),
            TresleFacadeCommonError::TaskIdUpdateError {
                error_code,
                reference_id,
                ..
            } => (*error_code, reference_id),
        };

        error_utils::ApiErrorResponse::new(
            self.to_string(),
            Some(reference_id.to_string()),
            error_code,
            None,
            None,
            None,
        )
    }

    fn source(&self) -> String {
        match self {
            TresleFacadeCommonError::RouteNotFound { source, .. } => source.to_string(),
            _ => "No source available".to_string(), // Default case
        }
    }

    fn task_id(&self) -> String {
        match self {
            TresleFacadeCommonError::RouteNotFound { task_id, .. } => task_id.to_string(),
            _ => "No task_id available".to_string(), // Default case
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{self, ErrorKind};

    #[test]
    fn test_success_missing_api_key() {
        let reference_id = "test_reference_id".to_string();
        let task_id = "test_task_id".to_string();
        let ext_message = "Internal Error. Please contact tresleai support team.".to_string();
        let error = TresleFacadeCommonError::missing_api_key(&reference_id, &task_id, &ext_message);
        assert!(error
            .to_string()
            .contains("Internal Error. Please contact tresleai support team. Use reference ID:"));
    }

    #[test]
    fn test_success_invalid_api_key() {
        let reference_id = "test_reference_id".to_string();
        let task_id = "test_task_id".to_string();
        let ext_message = "Internal Error. Please contact tresleai support team.".to_string();
        let error = TresleFacadeCommonError::invalid_api_key(&reference_id, &task_id, &ext_message);
        assert!(error
            .to_string()
            .contains("Internal Error. Please contact tresleai support team. Use reference ID:"));
    }

    #[test]
    fn test_sucess_failed_to_fetch_app_name_from_db() {
        let reference_id = "test_reference_id".to_string();
        let task_id = "test_task_id".to_string();
        let ext_message = "Internal Error. Please contact tresleai support team.".to_string();
        let e = io::Error::new(
            ErrorKind::Other,
            "Failed to fetch app name from DocumentDB. Error: Some error".to_string(),
        );
        let error = TresleFacadeCommonError::failed_to_fetch_app_name_from_db(
            &reference_id,
            &task_id,
            e,
            &ext_message,
        );
        assert!(error
            .to_string()
            .contains("Internal Error. Please contact tresleai support team. Use reference ID:"));
    }

    #[test]
    fn test_success_no_app_name_key_found() {
        let reference_id = "test_reference_id".to_string();
        let task_id = "test_task_id".to_string();
        let ext_message = "Internal Error. Please contact tresleai support team.".to_string();
        let error =
            TresleFacadeCommonError::no_app_name_key_found(&reference_id, &task_id, &ext_message);
        assert!(error
            .to_string()
            .contains("Internal Error. Please contact tresleai support team. Use reference ID:"));
    }

    #[test]
    fn test_success_no_app_name_found_for_given_api_key() {
        let reference_id = "test_reference_id".to_string();
        let task_id = "test_task_id".to_string();
        let ext_message = "Internal Error. Please contact tresleai support team.".to_string();
        let error = TresleFacadeCommonError::no_app_name_found_for_given_api_key(
            &reference_id,
            &task_id,
            &ext_message,
        );
        assert!(error
            .to_string()
            .contains("Internal Error. Please contact tresleai support team. Use reference ID:"));
    }

    #[test]
    fn test_success_failed_to_read_retrieval_request_body() {
        let reference_id = "test_reference_id".to_string();
        let task_id = "test_task_id".to_string();
        let ext_message = "Internal Error. Please contact tresleai support team.".to_string();
        let error = TresleFacadeCommonError::failed_to_read_retrieval_request_body(
            &reference_id,
            &task_id,
            &ext_message,
        );
        assert!(error
            .to_string()
            .contains("Internal Error. Please contact tresleai support team. Use reference ID:"));
    }

    #[test]
    fn test_success_failed_to_parse_retrieval_request_body() {
        let reference_id = "test_reference_id".to_string();
        let task_id = "test_task_id".to_string();
        let ext_message = "Internal Error. Please contact tresleai support team.".to_string();
        let e = io::Error::new(ErrorKind::Other, "Some error".to_string());
        let error = TresleFacadeCommonError::failed_to_parse_retrieval_request_body(
            &reference_id,
            &task_id,
            e,
            &ext_message,
        );
        assert!(error
            .to_string()
            .contains("Internal Error. Please contact tresleai support team. Use reference ID:"));
    }

    #[test]
    fn test_success_failed_to_create_document_in_db() {
        let app_name = "app1".to_string();
        let reference_id = "test_reference_id".to_string();
        let task_id = "test_task_id".to_string();
        let doc_type = "history".to_string();
        let e = io::Error::new(ErrorKind::Other, "Some error".to_string());
        let ext_message = "Internal Error. Please contact tresleai support team.".to_string();
        let error = TresleFacadeCommonError::failed_to_create_document_in_db(
            &app_name,
            &reference_id,
            &task_id,
            &doc_type,
            e,
            &ext_message,
        );
        assert!(error
            .to_string()
            .contains("Internal Error. Please contact tresleai support team. Use reference ID:"));
    }

    #[test]
    fn test_success_failed_to_convert_bson_to_document() {
        let app_name = "app1".to_string();
        let reference_id = "test_reference_id".to_string();
        let task_id = "test_task_id".to_string();
        let ext_message = "Internal Error. Please contact tresleai support team.".to_string();
        let error = TresleFacadeCommonError::failed_to_convert_bson_to_document(
            &app_name,
            &reference_id,
            &task_id,
            &ext_message,
        );
        assert!(error
            .to_string()
            .contains("Internal Error. Please contact tresleai support team. Use reference ID:"));
    }

    // #[test]
    // fn test_success_failed_to_validate_iam_policies() {
    //     let app_name = "app1".to_string();
    //     let reference_id = "test_reference_id".to_string();
    //     let task_id = "test_task_id".to_string();
    //     let unvalidated_policies = "Some policies".to_string();
    //     let ext_message = "Internal Error. Please contact tresleai support team.".to_string();
    //     let error = TresleFacadeCommonError::failed_to_validate_iam_policies(
    //         &app_name,
    //         &reference_id,
    //         &task_id,
    //         unvalidated_policies,
    //         &ext_message,
    //     );
    //     assert!(error
    //         .to_string()
    //         .contains("Internal Error. Please contact tresleai support team. Use reference ID:"));
    // }

    #[test]
    fn test_success_no_history_document_found_but_request_accepted() {
        let app_name = "app1".to_string();
        let reference_id_query_param = "test_reference_id".to_string();
        let reference_id = "test_task_id".to_string();
        let task_id = "test_task_id".to_string();
        let ext_message = "Retrieval in progress.".to_string();
        let error = TresleFacadeCommonError::no_history_document_found_but_request_accepted(
            &app_name,
            &reference_id_query_param,
            &reference_id,
            &task_id,
            &ext_message,
        );
        assert!(error.to_string().contains("Retrieval in progress."));
    }

    #[test]
    fn test_success_failed_to_retrieve_history_document() {
        let app_name = "app1".to_string();
        let reference_id_query_param = "test_reference_id_query_param".to_string();
        let reference_id = "test_reference_id".to_string();
        let task_id = "test_task_id".to_string();
        let e = io::Error::new(ErrorKind::Other, "Some error".to_string());
        let ext_message = "Internal Error. Please contact tresleai support team.".to_string();
        let error = TresleFacadeCommonError::failed_to_retrieve_history_document(
            &app_name,
            &reference_id_query_param,
            &reference_id,
            &task_id,
            e,
            &ext_message,
        );
        assert!(error
            .to_string()
            .contains("Internal Error. Please contact tresleai support team. Use reference ID:"));
    }

    #[test]
    fn test_success_missing_reference_id_in_history_retrieval_request() {
        let reference_id = "test_reference_id".to_string();
        let task_id = "test_task_id".to_string();
        let ext_message = "Internal Error. Please contact tresleai support team.".to_string();
        let error = TresleFacadeCommonError::missing_reference_id_in_history_retrieval_request(
            &reference_id,
            &task_id,
            &ext_message,
        );
        assert!(error
            .to_string()
            .contains("Internal Error. Please contact tresleai support team. Use reference ID:"));
    }

    #[test]
    fn test_success_failed_to_update_document_in_db() {
        let reference_id = "test_reference_id".to_string();
        let task_id = "test_task_id".to_string();
        let ext_message = "Internal Error. Please contact tresleai support team.".to_string();
        let e = io::Error::new(ErrorKind::Other, "Some error".to_string());
        let error = TresleFacadeCommonError::failed_to_update_document_in_db(
            &reference_id,
            &task_id,
            e,
            &ext_message,
        );
        assert!(error
            .to_string()
            .contains("Internal Error. Please contact tresleai support team. Use reference ID:"));
    }

    #[test]
    fn test_success_no_document_found_to_update() {
        let reference_id = "test_reference_id".to_string();
        let task_id = "test_task_id".to_string();
        let ext_message = "Internal Error. Please contact tresleai support team.".to_string();
        let error = TresleFacadeCommonError::no_document_found_to_update(
            &reference_id,
            &task_id,
            &ext_message,
        );
        assert!(error
            .to_string()
            .contains("Internal Error. Please contact tresleai support team. Use reference ID:"));
    }

    #[test]
    fn test_success_failed_to_deserialize_update_response() {
        let reference_id = "test_reference_id".to_string();
        let task_id = "test_task_id".to_string();
        let e = io::Error::new(ErrorKind::Other, "Some error".to_string());
        let ext_message = "Internal Error. Please contact tresleai support team.".to_string();
        let error = TresleFacadeCommonError::failed_to_deserialize_update_response(
            &reference_id,
            &task_id,
            e,
            &ext_message,
        );
        assert!(error
            .to_string()
            .contains("Internal Error. Please contact tresleai support team. Use reference ID:"));
    }

    #[test]
    fn test_error_response() {
        let error = TresleFacadeCommonError::RouteNotFound {
            task_id: "test_task_id".to_string(),
            time_stamp: "test_time_stamp".to_string(),
            error_code: StatusCode::BAD_REQUEST,
            source: "test source".to_string().into(),
            reference_id: "test_reference_id".to_string(),
            ext_message: "test_ext_message".to_string(),
        };

        let response = error.error_response();
        assert_eq!(response.error_code(), 400);
    }

    #[test]
    fn test_source() {
        let error = TresleFacadeCommonError::RouteNotFound {
            task_id: "test_task_id".to_string(),
            time_stamp: "test_time_stamp".to_string(),
            error_code: StatusCode::BAD_REQUEST,
            source: "test source".to_string().into(),
            reference_id: "test_reference_id".to_string(),
            ext_message: "test_ext_message".to_string(),
        };

        assert_eq!(
            error_utils::TresleAppError::source(&error),
            "test source".to_string()
        );

        let error = TresleFacadeCommonError::ApiKeyError {
            time_stamp: "test_time_stamp".to_string(),
            error_code: StatusCode::BAD_REQUEST,
            reference_id: "test_reference_id".to_string(),
            ext_message: "test_ext_message".to_string(),
        };

        assert_eq!(
            error_utils::TresleAppError::source(&error),
            "No source available".to_string()
        );
    }

    #[test]
    fn test_task_id() {
        let error = TresleFacadeCommonError::RouteNotFound {
            task_id: "test_task_id".to_string(),
            time_stamp: "test_time_stamp".to_string(),
            error_code: StatusCode::BAD_REQUEST,
            source: "test source".to_string().into(),
            reference_id: "test_reference_id".to_string(),
            ext_message: "test_ext_message".to_string(),
        };

        assert_eq!(error.task_id(), "test_task_id".to_string());

        let error = TresleFacadeCommonError::ApiKeyError {
            time_stamp: "test_time_stamp".to_string(),
            error_code: StatusCode::BAD_REQUEST,
            reference_id: "test_reference_id".to_string(),
            ext_message: "test_ext_message".to_string(),
        };

        assert_eq!(error.task_id(), "No task_id available".to_string());
    }
}
