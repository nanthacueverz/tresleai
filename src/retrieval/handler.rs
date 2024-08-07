/*
 * Created Date:   Feb 23, 2024
 * -----
 * Copyright (c) 2024 Tresle.ai or its affiliates. All Rights Reserved.
 */
//! This module contains the asynchronous POST handler for information retrieval and calls helper functions
//! to validate IAM policies and fetch data from the knowledge engine microservice.

use crate::retrieval::fetch_app_name::fetch_app_name;
use crate::retrieval::fetch_from_knowledge_engine::retrieve_from_knowledge_engine;
use crate::retrieval::update_task_id::update_task_id;
use crate::service::error::TresleFacadeCommonError;
use crate::service::generate_and_insert_document::DocType;
use crate::service::generate_and_insert_document::*;
use crate::AppState;
use api_utils::retrieval_model::RetrievalRequest;
use axum::body::{to_bytes, Body};
use axum::http::Request;
use axum::{extract::State, response::IntoResponse, Json};
use chrono::{DateTime, Utc};
use error_utils::AxumApiError;
use logging_utils::create_ref_id_helper::create_ref_id;
use logging_utils::create_task_id_helper::create_task_id;
use logging_utils::create_task_ref_id_helper::create_task_ref_collection;
use serde_json::json;
use std::sync::Arc;
use tracing::{error, info, instrument};

const HISTORY_COLLECTION_SUFFIX: &str = "-history";

#[instrument(skip_all)]
/// Asynchronous function to perform background operations with knowledge engine/core microservice and DocumentDB
async fn background_tasks(
    app_state: Arc<AppState>,
    app_name: String,
    user_id: String,
    body: RetrievalRequest,
    reference_id: String,
    task_id: String,
    request_timestamp: DateTime<Utc>,
) {
    // Retrieve data from the knowledge engine microservice
    match retrieve_from_knowledge_engine(&app_state, body.clone(), &app_name, &task_id).await {
        Ok(response) => {
            let retrieval_success_timestamp = Utc::now();
            let history_collection_name = format!("{}{}", &app_name, HISTORY_COLLECTION_SUFFIX);
            // Generate the history document and insert it in the history collection of that app in DocumentDB
            let history_document = generate_history_document(
                reference_id.clone(),
                task_id.clone(),
                &body.query,
                &response,
                retrieval_success_timestamp.to_string(),
                app_state.app_settings.disclaimer_text.clone(),
            )
            .await;
            if create_document_in_db(
                &app_state,
                &history_document,
                DocType::History,
                &history_collection_name,
                &app_name,
                &reference_id,
                &task_id,
            )
            .await
            .is_err()
            {
                return;
            }

            // Calculate the time taken to retrieve the data
            let retrieval_duration = format!(
                "{} ms",
                (retrieval_success_timestamp - request_timestamp).num_milliseconds()
            );
            let success_message = "Data retrieved successfully.".to_string();

            // Sending data to logs, audit and metrics microservices
            info!(app_name = &app_name, message = success_message);
            info!(
                service = "audit_microservice",
                task_id = task_id,
                app_name = &app_name,
                user_id = user_id,
                action = "Data Retrieval",
                details = success_message,
                message = success_message
            );
            info!(
                service = "metric",
                task_id = task_id,
                app_name = &app_name,
                metrics_name = "Data Retrieval Duration",
                metrics_value = retrieval_duration
            );
        }
        Err(error) => {
            let error_message = format!(
                "Failed to retrieve data from knowledge engine. Error: {}",
                error
            );
            error!(app_name = &app_name, message = error_message);

            // Send error to history collection
            let history_collection_name = format!("{}{}", &app_name, HISTORY_COLLECTION_SUFFIX);
            let history_document = generate_history_document(
                reference_id.clone(),
                task_id.clone(),
                &body.query,
                &error.to_string(),
                "Retrieval failed.".to_string(),
                app_state.app_settings.disclaimer_text.clone(),
            )
            .await;
            if create_document_in_db(
                &app_state,
                &history_document,
                DocType::History,
                &history_collection_name,
                &app_name,
                &reference_id,
                &task_id,
            )
            .await
            .is_err()
            {
                return;
            }
        }
    }
}

#[utoipa::path(
    post,
    path = "/api/v1.0/retrieval",
    request_body = RetrievalRequest,
    responses(
        (status = 200, description = "Retrieval in progress."),
        (status = StatusCode::BAD_REQUEST, description = "Internal Error. Please contact tresleai support team. Use reference ID: "),
        (status = StatusCode::NOT_FOUND, description = "Internal Error. Please contact tresleai support team. Use reference ID: "),
        (status = StatusCode::INTERNAL_SERVER_ERROR, description = "Internal Error. Please contact tresleai support team. Use reference ID: "),
    )
)]

/// POST handler to initiate the retrieval process for a response corresponding to a specific query.
///
/// This API triggers a retrieval operation in the backend, which fetches data based on the user details and query provided in the request. The retrieval process involves validating user policies, and asynchronously initiating the response retrieval by passing on the query and user details to the engine.
///
/// The request comprises several important components:
///
/// #### User Details
/// - The 'user_details' field contains the user's information, inclusive of their ID and IAM policy particulars.
/// - The 'access_details' field holds the details of the IAM policy and database policy.
/// - The IAM policies tied to the user outline their permissions and access rights to resources in the AWS environment. These policies are validated before proceeding with retrieval, else the request is terminated.
/// - The database policy details include the name of the database and table.
/// - Each policy is linked with a unique name and ARN (Amazon Resource Name).
/// - For instance, a policy might allow the user access to certain S3 buckets, or grant permissions to operate on other AWS resources. This would shape a tailored response based on the resources the user can access.
///
/// #### Query and additional prompt
/// - The 'query' field contains the query to initiate the retrieval.
/// - For enhanced context, the 'additional_prompt' field can be utilized.
///
/// #### API Key
/// - The application's API key is required to authenticate the request.
/// - This API key is created during the application onboarding process and is persisted in the API gateway of the concerned AWS account.
/// - It must be included in the `x-api-key` header of the request to associate it with an application.
///
/// #### Example
///
/// Consider a user with ID "sample_user@example.com", with access granted through two policies: "policy1" ("arn:aws:iam::aws:policy/policy1"), and "policy2" ("arn:aws:iam::aws:policy/policy2"). The application's API key is "a8VYYvaey38pajBi4jrMt8pGNdw5w0pn8oCytuQB". The user wants to initiate a retrieval with the query "provide a list of all accessible documents" and an additional prompt "related to policy1".
///
/// To initiate retrieval, a POST request would be made to the below endpoint:
///
/// ```
/// POST /api/v1.0/retrieval
/// x-api-key: a8VYYvaey38pajBi4jrMt8pGNdw5w0pn8oCytuQB
/// ```
///
/// Below is an example of how the request payload might look like:
///
/// ```
/// {
///     "user_details": {
///         "user_id": "sample_user@example.com",
///         "access_details": {
///             "iam_policy_details": [
///                 {
///                     "policy_name": "policy1",
///                     "policy_arn": "arn:aws:iam::aws:policy/policy1"
///                 },
///                 {
///                     "policy_name": "policy2",
///                     "policy_arn": "arn:aws:iam::aws:policy/policy2"
///                 }
///             ]
///            "db_policy_details": [
///                {
///                   "database_name": "database1",
///                  "table_name": "table1"
///                 },
///               {
///               "database_name": "database2",
///              "table_name": "table2"
///               }
///             ]
///         }
///     },
///     "query": "provide a list of all accessible documents",
///     "additional_prompt": "related to policy1"
/// }
/// ```
///
/// Upon successful initiation, the response would be returned as follows:
///
/// ```
/// {
///     "status": "success",
///     "message": "Retrieval initiated successfully.",
///     "reference_id": "14b1456d-2708-45bc-8989-eac2d2eba4db"
/// }
/// ```
///
/// In the above response, the returned reference ID is then used to call the history retrieval API to fetch the response document from the database. It can also be used to contact Tresle support team with any questions or concerns.

#[instrument(skip_all)]
pub async fn post_retrieval_handler(
    State(app_state): State<Arc<AppState>>,
    request: Request<Body>,
) -> Result<impl IntoResponse, AxumApiError<TresleFacadeCommonError>> {
    let request_timestamp = Utc::now();

    // Generate reference ID and task ID and initialize the app_name (generic app_name = "tresleai-system")
    let reference_id = create_ref_id();
    let mut app_name = app_state.app_settings.tracing_layer_system_app_name.clone();
    let service_type = "Retrieval".to_string();
    let initial_task_id = create_task_id(&app_name, service_type);
    // Fetch general message to be returned to client, in case of an error
    let ext_message = app_state.app_settings.general_message.clone();

    // Generate and insert the initial ID document in DocumentDB
    let id_document =
        generate_id_document(&app_name, reference_id.clone(), initial_task_id.clone()).await;
    create_document_in_db(
        &app_state,
        &id_document,
        DocType::ID,
        &app_state.app_settings.mongo_db.mongo_db_id_collection,
        &app_name,
        &reference_id,
        &initial_task_id,
    )
    .await?;

    // Extract the API key from the request headers
    let headers = request.headers();
    let api_key = headers
        .get("x-api-key")
        .ok_or_else(|| {
            TresleFacadeCommonError::missing_api_key(&reference_id, &initial_task_id, &ext_message)
        })?
        .to_str()
        .map_err(|_| {
            TresleFacadeCommonError::invalid_api_key(&reference_id, &initial_task_id, &ext_message)
        })?;

    // Fetch and update the app name corresponding to the API key
    app_name = fetch_app_name(
        &app_state,
        &api_key.to_string(),
        &initial_task_id,
        &reference_id,
    )
    .await?;

    // Extract the request body and deserialize it
    let body_bytes = to_bytes(request.into_body(), usize::MAX)
        .await
        .map_err(|_| {
            TresleFacadeCommonError::failed_to_read_retrieval_request_body(
                &reference_id,
                &initial_task_id,
                &ext_message,
            )
        })?;

    let body: RetrievalRequest = serde_json::from_slice(&body_bytes).map_err(|e| {
        TresleFacadeCommonError::failed_to_parse_retrieval_request_body(
            &reference_id,
            &initial_task_id,
            e,
            &ext_message,
        )
    })?;
    //Verify if both access_details in the request body are empty, if so, return an error
    let access_details = &body.user_details.access_details;
    if access_details.iam_policy_details.is_none() && access_details.db_policy_details.is_none() {
        let ext_message = "Access details cannot be empty".to_string();
        let msg = format!("access_details cannot be empty : {:?}", access_details);
        error!(
            app_name = &app_name,
            task_id = &initial_task_id,
            ext_message = ext_message,
            message = msg
        );
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
            app_name,
            initial_task_id,
            reference_id.clone(),
        )
        .await;
        return Ok(Json(
            json!({"status": "failed", "message": ext_message, "reference_id": reference_id}),
        ));
    }

    // Call to 'Retrieval' - generate the UI summary document and insert it in DocumentDB
    let ui_summary_document =
        generate_ui_summary_document(&app_name, "Retrieval", 1, request_timestamp.to_string())
            .await;
    create_document_in_db(
        &app_state,
        &ui_summary_document,
        DocType::UiSummary,
        &app_state
            .app_settings
            .mongo_db
            .mongo_db_ui_summary_collection,
        &app_name,
        &reference_id,
        &initial_task_id,
    )
    .await?;

    let user_id = &body.user_details.user_id;
    let _iam_policy_details = &body.user_details.access_details.iam_policy_details;

    // Generate task ID
    let updated_task_id = create_task_id(&app_name, "Retrieval".to_string());

    // Now that we have the app_name, update id_document with new task_id and app_name
    update_task_id(
        &app_state,
        &app_name,
        &reference_id,
        &initial_task_id,
        &updated_task_id,
    )
    .await?;

    // Instrument function call counter
    info!(
        service = "metric",
        app_name = app_name,
        task_id = updated_task_id,
        metrics_name = "Data Retrieval Counter",
        metrics_value = "1"
    );

    // Spawn a background async task to perform operations with knowledge engine/core microservice and DocumentDB
    tokio::spawn(background_tasks(
        Arc::clone(&app_state),
        app_name,
        user_id.clone(),
        body,
        reference_id.clone(),
        updated_task_id,
        request_timestamp,
    ));

    Ok(Json(
        json!({"status": "success", "message": "Retrieval in progress.","reference_id": reference_id}),
    ))
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::tests::*;
    use std::fs::File;
    use std::io::Read;
    use tokio::runtime::Runtime;

    #[test]
    // #[ignore="until posting to core service is implemented"]
    pub fn test_success_post_retrieval_handler() {
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

            let app_config: RetrievalRequest = serde_json::from_str(&buff).unwrap();

            // Convert the OnboardingRequest to JSON and then to a Body
            let app_config_json = serde_json::to_string(&app_config).unwrap();
            let body = Body::from(app_config_json);

            // Create a Request<Body>
            let mut request = Request::post("/").body(body).unwrap();
            request.headers_mut().insert(
                "x-api-key",
                "GC7Ldy1i6I7eEffBJ4bW52N7rNWtxSTv2bu9TQ5C".parse().unwrap(),
            );

            // Call the function
            let result = post_retrieval_handler(State(app_state), request).await;

            // Check that the result is as expected
            println!("{:?}", result.err());
            //assert!(result.is_ok());
            //  mock_server.assert();
        });
    }

    #[test]
    fn test_success_background_tasks() {
        let rt = Runtime::new().unwrap();

        rt.block_on(async {
            // Create a dev AppState
            let app_state = crate::tests::test_get_appstate().await.unwrap(); // Note global.yaml need to point to localhost:8003

            // Create a mock RetrievalRequest
            let mut file = File::open("src/test/retrieval_request.json").unwrap();
            let mut buff = String::new();
            file.read_to_string(&mut buff).unwrap();

            let app_config: RetrievalRequest = serde_json::from_str(&buff).unwrap();

            // Call the function
            background_tasks(
                Arc::clone(&app_state),
                "test".to_string(),
                "test".to_string(),
                app_config,
                "test".to_string(),
                "test".to_string(),
                Utc::now(),
            )
            .await;
            std::thread::sleep(std::time::Duration::from_secs(2));
        });
    }

    #[test]
    fn test_failed_post_retrieval_handler_missing_api_key() {
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

            // Call the function
            let result = post_retrieval_handler(State(app_state), request).await;

            // Check that the result is as expected
            assert!(result.is_err());
            match result.err().unwrap().inner {
                TresleFacadeCommonError::ApiKeyError { .. } => assert!(true),
                _ => assert!(false, "Expected ApiKeyError"),
            }
        });
    }

    #[test]
    fn test_failed_post_retrieval_handler_bad_api_key() {
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

            // Call the function
            let result = post_retrieval_handler(State(app_state), request).await;

            // Check that the result is as expected
            assert!(result.is_err());
            match result.err().unwrap().inner {
                TresleFacadeCommonError::ApiKeyError { .. } => assert!(true),
                _ => assert!(false, "Expected ApiKeyError"),
            }
        });
    }

    #[test]
    fn test_failed_post_retrieval_handler_empty_body() {
        let rt = Runtime::new().unwrap();

        rt.block_on(async {
            // Create a dev AppState
            let app_state = crate::tests::test_get_appstate().await.unwrap(); // Note global.yaml need to point to localhost:8003

            // Create a Request<Body>
            let mut request = Request::post("/").body(Body::empty()).unwrap();
            request.headers_mut().insert(
                "x-api-key",
                "1ytmOsUYKI2ZGg7WzzSfH3YU87i6UtZ50uMgVCc5".parse().unwrap(),
            );

            // Call the function
            let result = post_retrieval_handler(State(app_state), request).await;

            // Check that the result is as expected
            assert!(result.is_err());
            match result.err().unwrap().inner {
                TresleFacadeCommonError::RetrievalRequestBodyError { .. } => assert!(true),
                _ => assert!(false, "Expected RetrievalRequestBodyError"),
            }
        });
    }
}
