/*
 * Created Date:  Mar 17, 2024
 * -----
 * Copyright (c) 2024 Tresle.ai or its affiliates. All Rights Reserved.
 */

//! This module makes a POST request to the core microservice with a request body and receives
//! a response from it.
//! The function is used by the retrieval service to fetch data from the core microservice.
//! The function returns a 500 status code if an error occurs while fetching data from the core microservice.
//!

use crate::service::state::AppState;
use api_utils::retrieval_model::RetrievalRequest;
use reqwest::header::CONTENT_TYPE;
use std::sync::Arc;
use tracing::{debug, instrument};

#[derive(thiserror::Error, Debug)]

pub enum TresleFacadeRetrievalError {
    #[error("Error in making a POST request to the core microservice.")]
    ReqwestError(#[from] reqwest::Error),
    #[error("Error in serializing the request body.")]
    SerdeJsonError(#[from] serde_json::Error),
}

/// Function to make a POST request to the core with the request body and receive a response from it.
#[instrument(skip_all)]
pub async fn retrieve_from_knowledge_engine(
    app_state: &Arc<AppState>,
    mut body: RetrievalRequest,
    app_name: &str,
    task_id: &str,
) -> Result<String, TresleFacadeRetrievalError> {
    // Add app_name and task_id to the body
    body.app_name = Some(app_name.to_owned());
    body.task_id = Some(task_id.to_owned());

    debug!("Retrieving data from the core microservice.");
    let url = format!(
        "{}/{}",
        app_state
            .app_settings
            .tresleai_urls
            .core_service_url
            .clone(),
        app_state.app_settings.knowledge_engine.endpoint.clone()
    );

    debug!(
        "Making a POST request to the core microservice at URL: {}",
        url
    );
    let client = reqwest::Client::new();

    // Send serialized body as request payload to the core
    let serialized_body = serde_json::to_string(&body)?;

    let response = client
        .post(url)
        .header(CONTENT_TYPE, "application/json")
        .body(serialized_body)
        .send()
        .await?
        .text()
        .await?;

    Ok(response)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::*;
    use std::fs::File;
    use std::io::Read;
    use tokio::runtime::Runtime;

    #[test]
    // #[ignore = "until posting to core service is implemented"]
    fn test_success_check_connectivity() {
        let rt = Runtime::new().unwrap();

        rt.block_on(async {
            // Create a dev AppState
            let app_state = crate::tests::test_get_appstate().await.unwrap();

            let path = app_state.app_settings.knowledge_engine.endpoint.to_string();

            let mut mock_server = MOCK_SERVER.lock().unwrap();
            mock_server
                .mock("POST", path.as_str())
                .with_status(200)
                .with_header("content-type", "application/json")
                .with_body("{\"status\": \"ok\"}")
                .create();

            let mut file = File::open("src/test/retrieval_request.json").unwrap();
            let mut buff = String::new();
            file.read_to_string(&mut buff).unwrap();

            let retrieval_request: RetrievalRequest = serde_json::from_str(&buff).unwrap();
            let app_name = String::from("app1");
            let task_id =
                String::from("TSK-47829-app_223-Onboarding-2024-04-04 05:52:22.755295 UTC");

            // Call the function
            let result =
                retrieve_from_knowledge_engine(&app_state, retrieval_request, &app_name, &task_id)
                    .await;

            println!("results:{:?}\n", result);
            // Check that the result is as expected
            assert!(result.is_ok());
        });
    }
}
