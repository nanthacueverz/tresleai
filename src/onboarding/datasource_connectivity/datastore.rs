/*
 * Created Date: Feb 28, 2024
 * -----
 * Copyright (c) 2024 Tresle.ai or its affiliates. All Rights Reserved.
 */

//! This module contains the functions to check the connectivity to the RDS databases and to get the data for sending to Kafka.
//! The functions are used by the onboarding service to check the connectivity to the RDS databases and to get the data for sending to Kafka.
//!

use crate::onboarding::schema::app_onboarding_request::AppDataSource;
use crate::onboarding::schema::app_onboarding_request::DataStore;
use crate::service::state::AppState;
use authentication_utils::AwsAuthentication;
use axum::{http::StatusCode, Json};
use futures::stream::StreamExt;
use opensearch_utils::OpenSearchClient;
use relational_db_utils::RelationalDbClient;
use std::sync::Arc;
use tracing::{debug, instrument};

#[instrument(skip_all)]
/// Function to check the connectivity to the RDS databases. Returns a vector of strings representing
/// the connectivity check failures, if any.
pub async fn datastore_check_connectivity(
    data_source: &str,
    app_state: &Arc<AppState>,
    app_datasource: &AppDataSource,
) -> Result<Vec<String>, (StatusCode, Json<serde_json::Value>)> {
    // Get the databases of a particular type like mysql, postgres etc.
    let datastore = &app_datasource.datastore;
    let databases = match datastore.get(data_source) {
        Some(databases) => databases,
        None => {
            let error_message =
                format!("Error: No databases found for data source: {}", data_source);
            debug!("{}", error_message);
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                "status": "error",
                "error": error_message})),
            ));
        }
    };

    // Holds the connectivity errors, if any
    let mut connectivity_errors = Vec::new();

    // Create an AWS authentication instance
    let mut aws_auth_builder = AwsAuthentication::builder();
    aws_auth_builder = match &app_state.app_settings.aws {
        Some(aws) => aws_auth_builder
            .set_aws_access_key_id(aws.access_key_id.clone())
            .set_aws_secret_access_key(aws.secret_access_key.clone())
            .set_aws_default_region(aws.default_region.clone()),
        None => aws_auth_builder,
    };

    let aws_auth = match aws_auth_builder.build().await {
        Ok(auth) => auth,
        Err(e) => {
            let error_message = format!("Error: Failed to create AWS authentication: {}", e);
            debug!("{}", error_message);
            connectivity_errors.push(error_message.clone());
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                "status": "error",
                "error": error_message})),
            ));
        }
    };

    // Get the timeout for waiting for the connection to be established
    let timeout_sec = app_state
        .app_settings
        .datastore
        .connection_timeout_seconds
        .clone();

    // TODO: Check if cloning can be avoided. also check for potential improvements
    let connectivity_errors = futures::stream::iter(
        databases
            .clone()
            .into_iter()
            .map(|db| process_database(timeout_sec.clone(), aws_auth.clone(), db)),
    )
    .buffer_unordered(app_state.app_settings.datastore.max_concurrent_requests)
    .filter_map(|result| async move { result })
    .collect::<Vec<_>>()
    .await;
    Ok(connectivity_errors)
}

/// Function to process the each database. Returns connectivity check failure as a string, if any.
async fn process_database(
    timeout_sec: String,
    aws_auth: AwsAuthentication,
    db: DataStore,
) -> Option<String> {
    match &db.db_type[..] {
        "mysql" | "postgres" => {
            let client = RelationalDbClient::builder()
                .set_database_type(&db.db_type)
                .set_secret_name(db.secret_name.clone())
                .set_host(&db.host)
                .set_port(&db.port)
                .set_database(&db.database)
                .set_timeout(&timeout_sec)
                .set_aws_auth(aws_auth.clone())
                .build()
                .await;

            match client {
                Ok(client) => {
                    // TODO: Check if third case ok to be empty
                    let table_query = match &db.db_type[..] {
                        "mysql" => {
                            "SELECT COUNT(*) FROM information_schema.tables WHERE table_name = ?"
                        }
                        "postgres" => {
                            "SELECT COUNT(*) FROM pg_catalog.pg_tables WHERE tablename = $1"
                        }
                        _ => "",
                    };

                    // Check the connectivity to each table in the database
                    for table in &db.tables {
                        let count_result =
                            client.check_if_table_exists(&table.name, table_query).await;
                        match count_result {
                            Ok(count) => {
                                // Table does not exist
                                if count == 0 {
                                    let error_message = format!(
                                        "Error: Table '{}' does not exist in '{}' database",
                                        table.name, db.database
                                    );
                                    debug!("{}", error_message);
                                    return Some(error_message);
                                }
                            }
                            Err(e) => {
                                let error_message = format!("Error: Failed to check if table '{}' exists in '{}' database: {}", table.name, db.database, e);
                                debug!("{}", error_message);
                                return Some(error_message);
                            }
                        }
                    }
                    None
                }
                Err(e) => {
                    let error_message = format!(
                        "Error: Failed to connect to '{}' database '{}': {}",
                        db.db_type, db.host, e
                    );
                    debug!("{}", error_message);
                    Some(error_message)
                }
            }
        }
        "opensearch" => {
            let client = OpenSearchClient::builder()
                .set_database_type(&db.db_type)
                .set_secret_name(db.secret_name.clone())
                .set_host(&db.host)
                .set_port(&db.port)
                .set_database(&db.database)
                .set_timeout(&timeout_sec)
                .set_aws_auth(aws_auth.clone())
                .set_aws_service_name(&db.aws_service_name.clone().unwrap_or_default())
                .build()
                .await;

            match client {
                Ok(client) => {
                    // Check the connectivity to each table in the database
                    for table in &db.tables {
                        let count_result = client.check_if_index_exists(&table.name).await;
                        match count_result {
                            Ok(count) => {
                                // Table does not exist
                                if count == 0 {
                                    let error_message = format!(
                                        "Error: Table '{}' does not exist in OpenSearch database: 'https://{}:{}'",
                                        table.name, db.host, db.port
                                    );
                                    debug!("{}", error_message);
                                    return Some(error_message);
                                }
                            }
                            Err(e) => {
                                let error_message = format!("Error: Failed to check if table '{}' exists in '{}' database: {}", table.name, db.database, e);
                                debug!("{}", error_message);
                                return Some(error_message);
                            }
                        }
                    }
                    None
                }
                Err(e) => {
                    let error_message = format!(
                        "Error: Failed to connect to '{}' database '{}': {}",
                        db.db_type,
                        db.host,
                        e // CHANGE DB.HOST TO AWS OPENSEARH URL FORMAT
                    );
                    debug!("{}", error_message);
                    Some(error_message)
                }
            }
        }
        _ => {
            let error_message = format!("Error: Unsupported database type found: {}", db.db_type);
            debug!("{}", error_message);
            Some(error_message)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::test_get_appstate;
    use std::fs::File;
    use std::io::Read;
    use tokio::runtime::Runtime;
    use tracing::info;
    use tracing_test::traced_test;

    #[test]
    #[traced_test]
    fn test_success_datastore_check_connectivity() {
        let rt = Runtime::new().unwrap();

        rt.block_on(async {
            let app_state = test_get_appstate().await.unwrap();
            let _guard = crate::tests::TEST_DB_MUTEX.lock().unwrap();

            let mut file = File::open("src/test/app_data_source.json").unwrap();
            let mut buff = String::new();
            file.read_to_string(&mut buff).unwrap();

            let app_data_source: AppDataSource = serde_json::from_str(&buff).unwrap();

            info!("app_data_source:{:#?}\n", app_data_source);

            let result_rds =
                datastore_check_connectivity("rds_mysql", &app_state, &app_data_source).await;

            let result_opensearch =
                datastore_check_connectivity("opensearch", &app_state, &app_data_source).await;

            info!("results_opensearch:{:#?}\n", result_opensearch);

            assert!(result_rds.is_ok());
            assert!(result_opensearch.is_ok());
        });
    }
}
