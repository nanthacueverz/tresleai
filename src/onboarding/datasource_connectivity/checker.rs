/*
 * Created Date:  Mar 17, 2024
 * -----
 * Copyright (c) 2024 Tresle.ai or its affiliates. All Rights Reserved.
 */

//! This module defines the Checker structs for the 'filestore' and 'datastore' data sources and the implementation
//! of the CheckerTrait for these structs.
//! The CheckerTrait defines the connectivity function that checks the connectivity to the data source.
//! The CheckerTrait is implemented for the FilestoreChecker and DatastoreChecker structs.
//! The FilestoreChecker struct is used to check the connectivity to 'filestore' data sources.
//! The DatastoreChecker struct is used to check the connectivity to 'datastore' data sources.
//!

use crate::onboarding::datasource_connectivity::datastore::datastore_check_connectivity;
use crate::onboarding::datasource_connectivity::filestore::filestore_check_connectivity;
use crate::onboarding::schema::app_onboarding_request::AppDataSource;
use crate::service::state::AppState;
use axum::{http::StatusCode, Json};
use std::sync::Arc;

/// Checker struct for 'filestore' data sources
pub struct FilestoreChecker;
/// Checker struct for 'datastore' data sources
pub struct DatastoreChecker;

pub trait CheckerTrait {
    // Function to check the connectivity to the data source
    async fn connectivity(
        &self,
        key: &str,
        app_state: &Arc<AppState>,
        app_data_source: &AppDataSource,
    ) -> Result<Vec<String>, (StatusCode, Json<serde_json::Value>)>;
}

impl CheckerTrait for FilestoreChecker {
    async fn connectivity(
        &self,
        key: &str,
        app_state: &Arc<AppState>,
        app_data_source: &AppDataSource,
    ) -> Result<Vec<String>, (StatusCode, Json<serde_json::Value>)> {
        filestore_check_connectivity(key, app_state, app_data_source).await
    }
}

impl CheckerTrait for DatastoreChecker {
    async fn connectivity(
        &self,
        key: &str,
        app_state: &Arc<AppState>,
        app_data_source: &AppDataSource,
    ) -> Result<Vec<String>, (StatusCode, Json<serde_json::Value>)> {
        datastore_check_connectivity(key, app_state, app_data_source).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::test_get_appstate;
    use std::fs::File;
    use std::io::Read;
    use tracing::info;
    use tracing_test::traced_test;

    /// test_filestore_checker_connectivity positive test case
    #[tokio::test]
    async fn test_success_filestore_checker_connectivity() {
        let checker = FilestoreChecker {};
        let app_state = test_get_appstate().await.unwrap();

        let mut file = File::open("src/test/app_data_source.json").unwrap();
        let mut buff = String::new();
        file.read_to_string(&mut buff).unwrap();

        let app_data_source: AppDataSource = serde_json::from_str(&buff).unwrap();

        print!("app_data_source:{:?}\n", app_data_source);

        let result = checker
            .connectivity("s3", &app_state, &app_data_source)
            .await;

        // Assert that the result is Ok
        assert!(result.is_ok());

        println!("results:{:?}\n", result);

        let result = result.unwrap();
        // Assert that the result is a Vec of length 0
        for res in result.iter() {
            assert_eq!(res.len(), 0);
        }
    }

    #[tokio::test]
    #[traced_test]
    /// test_datastore_checker_connectivity positive test case
    async fn test_sucess_datastore_checker_connectivity() {
        let checker = DatastoreChecker {};
        let app_state = test_get_appstate().await.unwrap();
        let _guard = crate::tests::TEST_DB_MUTEX.lock().unwrap();

        let mut file = File::open("src/test/app_data_source.json").unwrap();
        let mut buff = String::new();
        file.read_to_string(&mut buff).unwrap();

        let app_data_source: AppDataSource = serde_json::from_str(&buff).unwrap();

        info!("app_data_source:{:#?}\n", app_data_source);

        let result_rds = checker
            .connectivity("rds_mysql", &app_state, &app_data_source)
            .await;
        let result_opensearch = checker
            .connectivity("opensearch", &app_state, &app_data_source)
            .await;

        let error_count_rds = result_rds.unwrap().len();
        let error_count_opensearch = result_opensearch.unwrap().len();
        let total_error_count = error_count_rds + error_count_opensearch;

        // Assert that the result is an empty Vec
        // Adjust this according to your expected result
        assert_eq!(total_error_count, 0);
    }

    #[tokio::test]
    #[traced_test]
    /// test_datastore_checker_connectivity failure test case
    async fn test_fail_datastore_checker_connectivity() {
        let checker = DatastoreChecker {};
        let app_state = test_get_appstate().await.unwrap();
        let _guard = crate::tests::TEST_DB_MUTEX.lock().unwrap();

        let mut file = File::open("src/test/app_data_source_bad.json").unwrap();
        let mut buff = String::new();
        file.read_to_string(&mut buff).unwrap();

        let app_data_source: AppDataSource = serde_json::from_str(&buff).unwrap();

        info!("app_data_source:{:#?}\n", app_data_source);

        let result_rds = checker
            .connectivity("rds_mysql", &app_state, &app_data_source)
            .await;
        let result_opensearch = checker
            .connectivity("opensearch", &app_state, &app_data_source)
            .await;

        let error_count_rds = result_rds.unwrap().len();
        let error_count_opensearch = result_opensearch.unwrap().len();
        let total_error_count = error_count_rds + error_count_opensearch;

        assert_ne!(total_error_count, 0);
    }
}
