/*
 * Created Date:   Feb 23, 2024
 * -----
 * Copyright (c) 2024 Tresle.ai or its affiliates. All Rights Reserved.
 */
//! This module contains the asynchronous POST handler for retrieving a document from the history collection in DocumentDB
//! based on the reference_id provided in the query parameters.

use crate::admin_ui_api::schema::QueryParams;
use crate::retrieval::fetch_app_name::fetch_app_name;
use crate::service::error::TresleFacadeCommonError;
use crate::service::generate_and_insert_document::*;
use crate::service::state::AppState;
use axum::body::Body;
use axum::extract::Query;
use axum::http::Request;
use axum::{extract::State, response::IntoResponse, Json};
use error_utils::AxumApiError;
use mongodb::bson::doc;
use serde_json::json;
use std::sync::Arc;
use tracing::{info, instrument};
use uuid::Uuid;

const HISTORY_COLLECTION_SUFFIX: &str = "-history";

#[utoipa::path(
    get,
    path = "/api/v1.0/history/retrieval",
    params(
        (
            "reference_id" = inline(String), 
            Query,
            description = "Reference id.",
        )
    ),
    responses(
        (status = 200, description = "History document retrieved successfully."),
        (status = StatusCode::BAD_REQUEST, description = "Internal Error. Please contact tresleai support team. Use reference ID: "),
        (status = StatusCode::NOT_FOUND, description = "Internal Error. Please contact tresleai support team. Use reference ID: "),
        (status = StatusCode::ACCEPTED, description = "Internal Error. Please contact tresleai support team. Use reference ID: "),
        (status = StatusCode::INTERNAL_SERVER_ERROR, description = "Internal Error. Please contact tresleai support team. Use reference ID: ")
    )
)]

/// GET handler to extract a specific document from an application's history collection, with an input 'reference_id' as the basis for retrieval.
///
/// This API is designed to be used after initiating a retrieval process via the retrieval API. The retrieval API returns a 'reference_id' which is then used as the basis for retrieving the corresponding document from the history collection of an application within the database. The document contains the initiated query and its generated response, among other relevant details.
///
/// #### API Key
/// - The application's API key is required to authenticate the request.
/// - This API key is created during the application onboarding process and is persisted in the API gateway of the concerned AWS account.
/// - It must be included in the `x-api-key` header of the request to associate it with an application.
///
/// #### Example
///
/// Consider an application, named as "test_app", equipped with an API key "a8VYYvaey38pajBi4jrMt8pGNdw5w0pn8oCytuQB". This application has previously executed a retrieval operation, and obtained a reference ID "14b1456d-2708-45bc-8989-eac2d2eba4db" from the retrieval API.
/// The application can now extract the document associated with this specific reference ID from its history collection.
///
/// To accomplish this, a GET request would be made to the below endpoint, with the `reference_id` as a query parameter:
///
/// ```
/// GET /api/v1.0/history/retrieval?reference_id="14b1456d-2708-45bc-8989-eac2d2eba4db"
/// x-api-key: a8VYYvaey38pajBi4jrMt8pGNdw5w0pn8oCytuQB
/// ```
///
/// Upon successful retrieval of the document, the response would be returned as follows:
///
/// ```
/// {
///    "status": "success",
///    "message": "History document with reference ID: "14b1456d-2708-45bc-8989-eac2d2eba4db" retrieved successfully.",
///    "app_name": "test_app"
///    "data": {
///        "_id": {
///           "$oid": "<id of the document in DocumentDB>"
///        },
///        "reference_id": "14b1456d-2708-45bc-8989-eac2d2eba4db",
///        "task_id": "<task_id>",
///        "query": "<query>",
///        "response": "<response>",
///        "timestamp": "<timestamp>"
///    }
/// }
/// ```
///
/// Please note that the document is created in the database only upon successful generation of the response.
/// Until this point, the document is in the 'processing' state, indicated by a 202 (ACCEPTED) status code,
/// as demonstrated in the following example response (including a sample reference ID):
///
/// ```
/// {
///     "message": "Retrieval in progress.",
///     "reference_id": "0103f9a3-bbd7-40a6-9545-4122d63fa0a8",
///     "error_code": 202
/// }
/// ```

#[instrument(skip_all)]
pub async fn get_history_handler(
    State(app_state): State<Arc<AppState>>,
    Query(params): Query<QueryParams>,
    request: Request<Body>,
) -> Result<impl IntoResponse, AxumApiError<TresleFacadeCommonError>> {
    // Generate reference ID and task ID and initialize the app_name (generic app_name = "tresleai-system")
    let reference_id = Uuid::new_v4().to_string();
    let task_id = Uuid::new_v4().to_string();
    let app_name = app_state.app_settings.tracing_layer_system_app_name.clone();

    // Fetch general message to be returned to client, in case of an error
    let ext_message = app_state.app_settings.general_message.clone();
    let ext_msg_inprogress = app_state.app_settings.retrieval_progress_msg.to_string();

    // Generate and insert the ID document
    let id_document = generate_id_document(&app_name, reference_id.clone(), task_id.clone()).await;
    create_document_in_db(
        &app_state,
        &id_document,
        DocType::ID,
        &app_state.app_settings.mongo_db.mongo_db_id_collection,
        &app_name,
        &reference_id,
        &task_id,
    )
    .await?;

    // Extract the API key from the request headers
    let headers = request.headers();
    let api_key = headers
        .get("x-api-key")
        .ok_or_else(|| {
            TresleFacadeCommonError::missing_api_key(&reference_id, &task_id, &ext_message)
        })?
        .to_str()
        .map_err(|_| {
            TresleFacadeCommonError::invalid_api_key(&reference_id, &task_id, &ext_message)
        })?;

    // Fetch the app name corresponding to the API key
    let app_name =
        fetch_app_name(&app_state, &api_key.to_string(), &task_id, &reference_id).await?;

    // Extract the reference_id from the query params
    let reference_id_query_param = match params.reference_id {
        Some(reference_id_query_param) => reference_id_query_param,
        None => {
            return Err(error_utils::AxumApiError {
                inner: TresleFacadeCommonError::missing_reference_id_in_history_retrieval_request(
                    &reference_id,
                    &task_id,
                    &ext_message,
                ),
            })
        }
    };
    let filter = doc! {"reference_id": &reference_id_query_param};
    let history_collection_name = format!("{}{}", &app_name, HISTORY_COLLECTION_SUFFIX);

    match app_state
        .db
        .get_document(&history_collection_name, filter)
        .await
        .map_err(|e| {
            TresleFacadeCommonError::failed_to_retrieve_history_document(
                &app_name,
                &reference_id_query_param,
                &reference_id,
                &task_id,
                e,
                &ext_message,
            )
        }) {
        Ok(Some(history_document)) => {
            let success_message = format!(
                "History document with reference ID: '{}' retrieved successfully.",
                reference_id_query_param
            );
            info!(app_name = app_name, message = success_message);
            Ok(Json(
                json!({"status": "success", "message": success_message, "app_name": app_name, "data": history_document}),
            ))
        }
        Ok(None) => Err(error_utils::AxumApiError {
            inner: TresleFacadeCommonError::no_history_document_found_but_request_accepted(
                &app_name,
                &reference_id_query_param,
                &reference_id,
                &task_id,
                &ext_msg_inprogress,
            ),
        }),
        Err(e) => {
            return Err(error_utils::AxumApiError {
                inner: TresleFacadeCommonError::failed_to_retrieve_history_document(
                    &app_name,
                    &reference_id_query_param,
                    &reference_id,
                    &task_id,
                    e,
                    &ext_message,
                ),
            })
        }
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::tests::*;
    use api_utils::retrieval_model::RetrievalRequest;
    use std::fs::File;
    use std::io::Read;
    use tokio::runtime::Runtime;

    #[test]
    // #[ignore="until posting to core service is implemented"]
    pub fn test_success_post_history_handler() {
        let rt = Runtime::new().unwrap();

        rt.block_on(async {
            // Create a dev AppState
            let app_state = crate::tests::test_get_appstate().await.unwrap(); // Note global.yaml need to point to localhost:8003

            let path = app_state.app_settings.knowledge_engine.endpoint.to_string();

            let mut mock_server = MOCK_SERVER.lock().unwrap();
            mock_server
                .mock("POST", path.as_str())
                .with_status(200)
                .with_header("content-type", "application/json")
                .with_body("{\"status\": \"ok\"}")
                .create();

            // Create a mock RetrievalRequest
            let mut file = File::open("src/test/retrieval_request.json").unwrap();
            let mut buff = String::new();
            file.read_to_string(&mut buff).unwrap();

            let retrieval_request: RetrievalRequest = serde_json::from_str(&buff).unwrap();

            let retrieval_request_json = serde_json::to_string(&retrieval_request).unwrap();
            let body = Body::from(retrieval_request_json);

            // Create a Request<Body>
            let mut request = Request::post("/").body(body).unwrap();
            request.headers_mut().insert(
                "x-api-key",
                "GC7Ldy1i6I7eEffBJ4bW52N7rNWtxSTv2bu9TQ5C".parse().unwrap(),
            );

            let mut query_params = QueryParams::default();
            query_params.reference_id = Some("reference_id".to_string());

            // Call the function
            let result = get_history_handler(State(app_state), Query(query_params), request).await;

            // Check that the result is as expected
            println!("{:?}", result.err());
            //assert!(result.is_ok());
            //  mock_server.assert();
        });
    }
    #[test]
    // #[ignore="until posting to core service is implemented"]
    pub fn test_failed_post_history_handler_bad_api_key() {
        let rt = Runtime::new().unwrap();

        rt.block_on(async {
            // Create a dev AppState
            let app_state = crate::tests::test_get_appstate().await.unwrap(); // Note global.yaml need to point to localhost:8003

            // Create a mock RetrievalRequest
            let mut file = File::open("src/test/retrieval_request.json").unwrap();
            let mut buff = String::new();
            file.read_to_string(&mut buff).unwrap();

            let app_config: RetrievalRequest = serde_json::from_str(&buff).unwrap();

            // Convert the OnboardingRequest to JSON and then to a Body
            let app_config_json = serde_json::to_string(&app_config).unwrap();
            let body = Body::from(app_config_json);

            // Create a Request<Body>
            let mut request = Request::post("/").body(body).unwrap();
            request
                .headers_mut()
                .insert("x-api-key", "🚀  bad key".parse().unwrap());

            let mut query_params = QueryParams::default();

            query_params.reference_id = Some("reference_id".to_string());

            // Call the function
            let result = get_history_handler(State(app_state), Query(query_params), request).await;

            // Check that the result is as expected
            assert!(result.is_err());
            match result.err().unwrap().inner {
                TresleFacadeCommonError::ApiKeyError { .. } => assert!(true),
                _ => assert!(false, "Expected ApiKeyError"),
            }
        });
    }

    #[test]
    // #[ignore="until posting to core service is implemented"]
    pub fn test_failed_post_history_handler_missing_api_key() {
        let rt = Runtime::new().unwrap();

        rt.block_on(async {
            // Create a dev AppState
            let app_state = crate::tests::test_get_appstate().await.unwrap(); // Note global.yaml need to point to localhost:8003

            // Create a mock RetrievalRequest
            let mut file = File::open("src/test/retrieval_request.json").unwrap();
            let mut buff = String::new();
            file.read_to_string(&mut buff).unwrap();

            let app_config: RetrievalRequest = serde_json::from_str(&buff).unwrap();

            // Convert the OnboardingRequest to JSON and then to a Body
            let app_config_json = serde_json::to_string(&app_config).unwrap();
            let body = Body::from(app_config_json);

            // Create a Request<Body>
            let request = Request::post("/").body(body).unwrap();

            let mut query_params = QueryParams::default();
            query_params.reference_id = Some("reference_id".to_string());

            // Call the function
            let result = get_history_handler(State(app_state), Query(query_params), request).await;

            // Check that the result is as expected
            assert!(result.is_err());
            match result.err().unwrap().inner {
                TresleFacadeCommonError::ApiKeyError { .. } => assert!(true),
                _ => assert!(false, "Expected ApiKeyError"),
            }
        });
    }
}
