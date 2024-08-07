/*
* Created Date:  Mar 17, 2024
 * -----
 * Copyright (c) 2024 Tresle.ai or its affiliates. All Rights Reserved.
 */
//! This module contains the setting

use secrecy::Secret;
use serde::Deserialize;
use std::net::{IpAddr, SocketAddr};

#[derive(Debug, thiserror::Error)]
pub enum SettingsError {
    #[error("Failed to parse config file: {0}")]
    Config(config::ConfigError),
}

/// Top level settings for the Tresle Facade Service
#[derive(Debug, Deserialize)]
pub struct TresleFacadeServiceSettings {
    // Global settings
    pub tresle_version: String,
    pub release_date: String,
    pub environment: String,
    pub env_identifier: String,
    pub region: String,
    pub service_name: String,
    pub product_name: String,
    pub kafka_brokers: String,
    pub kafka_trailing_message: String,
    pub global_artifact: String,
    pub supported_data_source_types: SupportedDataSourceTypes,
    pub supported_file_types: SupportedFileTypes,
    pub cors_allowed_origins: Vec<String>,
    pub mongo_db: MongoDBSettings,
    pub knowledge_engine: KnowledgeEngineSettings,
    pub tresleai_urls: TresleaiUrls,
    pub tracing_layer_levels: TracingLayerLevels,
    pub tracing_layer_system_app_name: String,
    pub general_message: String,

    // Local settings
    pub application: ApplicationSettings,
    pub aws: Option<AWSSettings>,
    pub aws_s3: AWSS3Settings,
    pub aws_iam: AWSIAMSettings,
    pub aws_api_gateway: AWSApiGatewaySettings,
    pub kafka_client: KafkaClientSettings,
    pub kubernetes: KubernetesSettings,
    pub app_generated_config: AppGeneratedConfigSettings,
    pub datastore: DatastoreSettings,
    pub disclaimer_text: String,
    pub tracing_layer_debug_mode: bool,
    pub onboard_inprogress_status: String,
    pub onboard_complete_status: String,
    pub sqs_key_value: String,
    pub retrieval_progress_msg: String,
}

/// Supported data source types.
#[derive(Debug, Deserialize)]
pub struct SupportedDataSourceTypes {
    pub file_store: Vec<String>,
    pub data_store: Vec<String>,
}

/// Supported file types.
#[derive(Debug, Deserialize)]
pub struct SupportedFileTypes {
    pub image: Vec<String>,
    pub text: Vec<String>,
}

/// MongoDB specific settings.
#[derive(Debug, Deserialize)]
pub struct MongoDBSettings {
    pub mongo_db_url: String,
    pub mongo_db_database_name: String,
    pub mongo_db_app_collection: String,
    pub mongo_db_id_collection: String,
    pub mongo_db_ui_summary_collection: String,
}

/// Knowledge Engine specific settings.
#[derive(Debug, Deserialize)]
pub struct KnowledgeEngineSettings {
    pub endpoint: String,
}

/// Tresleai specific URLs.
#[derive(Debug, Deserialize)]
pub struct TresleaiUrls {
    pub admin_ui_url: String,
    pub audit_service_url: String,
    pub core_service_url: String,
    pub event_processor_service_url: String,
    pub facade_service_url: String,
    pub knowledge_extraction_url: String,
    pub logging_service_url: String,
    pub metric_service_url: String,
    pub product_app_url: String,
}

/// Tresleai Tracing Layer Levels
#[derive(Debug, Deserialize, Clone, Default)]
pub struct TracingLayerLevels {
    pub fmt_layer_level: String,
    pub fmt_layer_service_exception_level: String,
    pub peripheral_services_layer_level: String,
}

/// Application and HTTP server specific settings
#[derive(Debug, Deserialize)]
pub struct ApplicationSettings {
    pub name: String,
    pub address: IpAddr,
    pub port: u16,
    pub cors: Cors,
    pub timestamp_format: String,
}

impl ApplicationSettings {
    pub fn address(&self) -> SocketAddr {
        SocketAddr::new(self.address, self.port)
    }
}

/// CORS specific settings
#[derive(Debug, Deserialize)]
pub struct Cors {
    pub enabled: bool,
    pub allowed_methods: Vec<String>,
    pub allowed_headers: Vec<String>,
    pub allow_credentials: bool,
}

/// AWS specific settings
#[derive(Debug, Deserialize)]
pub struct AWSSettings {
    pub access_key_id: Option<Secret<String>>,
    pub secret_access_key: Option<Secret<String>>,
    pub default_region: Option<String>,
}

/// AWS S3 specific settings
#[derive(Debug, Deserialize)]
pub struct AWSS3Settings {
    pub max_concurrent_requests: usize,
}

/// AWS IAM specific settings
#[derive(Debug, Deserialize)]
pub struct AWSIAMSettings {
    pub region: String,
}

/// AWS API Gateway specific settings
#[derive(Debug, Deserialize)]
pub struct AWSApiGatewaySettings {
    pub region: String,
    pub usage_plan_id: String,
    pub usage_plan_key_type: String,
}

/// Kafka client specific settings
#[derive(Debug, Deserialize)]
pub struct KafkaClientSettings {
    pub group_id: String,
    pub onboarding_topic: String,
    pub deletion_topic: String,
    pub kafka_enable_partition_eof: String,
    pub kafka_auto_offset_reset: String,
}

/// Kubernetes specific settings
#[derive(Debug, Deserialize)]
pub struct KubernetesSettings {
    pub namespace: String,
    pub secret_name: String,
}

/// App generated config specific settings
#[derive(Debug, Deserialize)]
pub struct AppGeneratedConfigSettings {
    pub knowledge_graph_config: KnowledgeGraphConfigSettings,
}

/// Knowledge Graph specific settings
#[derive(Debug, Deserialize)]
pub struct KnowledgeGraphConfigSettings {
    pub vectordb_config: VectorDBConfigSettings,
    pub parser_config: ParserConfigSettings,
    pub logging: LoggingSettings,
    pub audit: AuditSettings,
    pub metric: MetricSettings,
}

/// VectorDB specific settings
#[derive(Debug, Deserialize)]
pub struct VectorDBConfigSettings {
    pub text_collection_name_prefix: String,
    pub multimodal_collection_name_prefix: String,
    pub general_collection_name_prefix: String,
    pub session_history_collection_name_prefix: String,
    pub error_collection_name_prefix: String,
    pub insight_collection_name_prefix: String,
    pub retention: String,
    pub s3_storage_prefix: String,
}

/// Parser specific settings
#[derive(Debug, Deserialize)]
pub struct ParserConfigSettings {
    pub s3_storage_prefix: String,
}

/// Logging specific settings
#[derive(Debug, Deserialize)]
pub struct LoggingSettings {
    pub collection: String,
    pub retention: String,
    pub s3_prefix: String,
}

/// Audit specific settings
#[derive(Debug, Deserialize)]
pub struct AuditSettings {
    pub collection: String,
    pub retention: String,
    pub s3_prefix: String,
}

/// Metric specific settings
#[derive(Debug, Deserialize)]
pub struct MetricSettings {
    pub collection: String,
    pub retention: String,
    pub s3_prefix: String,
}

/// RDS specific settings
#[derive(Debug, Deserialize)]
pub struct DatastoreSettings {
    pub connection_timeout_seconds: String,
    pub max_concurrent_requests: usize,
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    #[allow(non_snake_case)]
    fn test_success_SettingsError() {
        let set_error = SettingsError::Config(config::ConfigError::Frozen);

        println!("Now {:?} will print!", set_error);
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_success_ApplicationSettings() {
        let appp_set = ApplicationSettings {
            name: "name".to_string(),
            address: IpAddr::V4(std::net::Ipv4Addr::new(127, 0, 0, 1)),
            port: 8080,
            cors: Cors {
                enabled: true,
                allowed_methods: vec!["GET".to_string(), "POST".to_string()],
                allowed_headers: vec!["header1".to_string(), "header2".to_string()],
                allow_credentials: true,
            },
            timestamp_format: "timestamp_format".to_string(),
        };

        let _addr = appp_set.address();
    }
}
