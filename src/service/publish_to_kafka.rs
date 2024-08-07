/*
 * Created Date:   Feb 23, 2024
 * -----
 * Copyright (c) 2024 Tresle.ai or its affiliates. All Rights Reserved.
 */

//! This module contains the function to publish data to Kafka

use crate::onboarding::schema::app_onboarding_request::AppDataSource;
use crate::onboarding::schema::app_onboarding_request::FileStore;
use crate::service::state::AppState;
use axum::{http::StatusCode, Json};
use kafka_utils::kafka_producer_client::KafkaProClient;
use kafka_utils::kafka_producer_client_builder::KafkaClientProdBuilder;
use serde::Serialize;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, error, info, instrument};

/// Asynchronous function to create a Kafka client
#[instrument(skip_all)]
pub async fn create_kafka_client(
    app_state: &Arc<AppState>,
    app_name: &str,
) -> Result<KafkaProClient, (StatusCode, Json<serde_json::Value>)> {
    let brokers = app_state.app_settings.kafka_brokers.clone();
    // Creating a Kafka client instance
    match KafkaClientProdBuilder::default()
        .set_bootstrap_servers(brokers)
        .build()
    {
        Ok(kafka_client_pro) => Ok(kafka_client_pro),
        Err(e) => {
            let error_message = format!("Failed to build Kafka Producer client. Error: {:?}", e);
            error!(
                app_name = app_name,
                ext_message = error_message,
                message = error_message
            );
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "status": "error","message": error_message})),
            ))
        }
    }
}

/// Function to serialize a message to JSON
pub fn serialize_to_json<T: Serialize>(
    message: &T,
    app_name: Option<&str>,
) -> Result<String, (StatusCode, Json<serde_json::Value>)> {
    match serde_json::to_string(message) {
        Ok(serialized_message) => Ok(serialized_message),
        Err(e) => {
            let error_message = format!(
                "Failed to serialize datasources data to JSON. Error: {:?}",
                e
            );
            error!(
                app_name = app_name,
                ext_message = error_message,
                message = error_message
            );
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "status": "error","message": error_message})),
            ))
        }
    }
}

/// Asynchronous function to send data to Kafka
#[instrument(skip_all)]
pub async fn send_to_kafka(
    kafka_client: &KafkaProClient,
    app_name: Option<&str>,
    topic: &str,
    key: &str,
    message: &str,
) -> Result<(), (StatusCode, Json<serde_json::Value>)> {
    debug!("Publishing the data to Kafka.");
    match kafka_client.produce(topic, key, message).await {
        Ok((partition, offset)) => {
            let success_message = format!(
                "Data published to Kafka successfully. Partition: {}, Offset: {}",
                partition, offset
            );
            info!(app_name = app_name, message = success_message);
            Ok(())
        }
        Err(e) => {
            let error_message = format!("Failed to publish data to Kafka. Error: {}", e);
            error!(
                app_name = app_name,
                ext_message = error_message,
                message = error_message
            );
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "status": "error","message": error_message})),
            ))
        }
    }
}

/// Asynchronous function to notify Kafka about app onboarding or updating an app
#[instrument(skip_all)]
pub async fn app_onboard_or_update_notify_kafka(
    app_state: &Arc<AppState>,
    app_name: &str,
    new_app_datasource: &AppDataSource,
    existing_app_datasource: Option<&AppDataSource>,
    task_id: String,
) -> Result<(), (StatusCode, Json<serde_json::Value>)> {
    let key = app_name;
    let topic = app_state.app_settings.kafka_client.onboarding_topic.clone();
    let kafka_client = create_kafka_client(app_state, app_name).await?;
    let trailing_message = &app_state.app_settings.kafka_trailing_message;
    let message: (String, &AppDataSource, Option<&AppDataSource>, &String);

    // If updating an existing app, send the new and existing datasources to Kafka, only if they are different.
    if let Some(existing_datasource) = existing_app_datasource {
        message = (
            task_id,
            new_app_datasource,
            Some(existing_datasource),
            trailing_message,
        );
    // If onboarding a new app, send the datasources to Kafka. There's no existing datasource in this case.
    } else {
        message = (task_id, new_app_datasource, None, trailing_message);
    }
    let serialized_message = serialize_to_json(&message, Some(app_name))?;

    send_to_kafka(
        &kafka_client,
        Some(app_name),
        &topic,
        key,
        &serialized_message,
    )
    .await?;
    Ok(())
}

/// Asynchronous function to notify Kafka about app deletion
#[instrument(skip_all)]
pub async fn app_deletion_notify_kafka(
    app_state: &Arc<AppState>,
    app_name: &str,
    sqs_key: &str,
    filestore: &HashMap<String, Vec<FileStore>>,
    task_id: String,
) -> Result<(), (StatusCode, Json<serde_json::Value>)> {
    let key = app_name;
    let topic = app_state.app_settings.kafka_client.deletion_topic.clone();
    let kafka_client = create_kafka_client(app_state, app_name).await?;
    let message: (String, &HashMap<String, Vec<FileStore>>, &str) = (task_id, filestore, sqs_key);
    let serialized_message = serialize_to_json(&message, None)?;
    send_to_kafka(&kafka_client, None, &topic, key, &serialized_message).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::runtime::Runtime;

    #[test]
    fn test_success_create_kafka_client() {
        let rt = Runtime::new().unwrap();

        rt.block_on(async {
            // Create a dev AppState
            let app_state = crate::tests::test_get_appstate().await.unwrap();
            let app_name = "test_app_name";

            // Call the function
            let result = create_kafka_client(&app_state, &app_name).await;

            // Check that the function returns Ok
            assert!(result.is_ok());
        });
    }

    #[derive(Serialize)]
    struct TestStruct {
        field: String,
    }

    #[test]
    fn test_success_serialize_to_json() {
        let message = TestStruct {
            field: "test".to_string(),
        };
        let result = serialize_to_json(&message, Some("test_app"));

        // Check that the function returns Ok and the serialized message is correct
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "{\"field\":\"test\"}");
    }

    use serde::ser::{self, Serialize};
    struct FailingToSerialize;

    impl Serialize for FailingToSerialize {
        fn serialize<S>(&self, _serializer: S) -> Result<S::Ok, S::Error>
        where
            S: ser::Serializer,
        {
            Err(ser::Error::custom("failed to serialize"))
        }
    }

    #[test]
    fn test_failure_serialize_to_json() {
        let message = FailingToSerialize;
        let result = serialize_to_json(&message, Some("test_app"));

        // Check that the function returns an error
        assert!(result.is_err());
    }
}
