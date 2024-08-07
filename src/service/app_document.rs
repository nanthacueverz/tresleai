/*
 * Created Date:  Mar 17, 2024
 * -----
 * Copyright (c) 2024 Tresle.ai or its affiliates. All Rights Reserved.
 */

//! This module contains the AppDocument struct and its builder. The AppDocument struct is used to represent
//! the document that is stored in DocumentDB for each app that is onboarded to the platform.

use crate::onboarding::schema::app_onboarding_request::{
    AppDataSource as OnboardingAppDataSource, EmbeddingModel as OnboardingEmbeddingModel,
    LlmModel as OnboardingLlmModel,
};
use crate::service::state::AppState;
use api_utils::app_model::*;
use chrono::Utc;
use llm_chain::llm_models::LlmModel;
use serde::Serialize;
use std::sync::Arc;

#[derive(Debug, thiserror::Error, Serialize, PartialEq)]
#[allow(clippy::enum_variant_names)]
pub enum AppDocumentCreationError {
    #[error("App name not provided")]
    AppNameNotProvided,
    #[error("App description not provided")]
    AppDescriptionNotProvided,
    #[error("App Text embedding model not provided")]
    AppTextEmbeddingModelNotProvided,
    #[error("App Multi modal embedding model not provided")]
    AppMultimodalEmbeddingModelNotProvided,
    #[error("App datasource not provided")]
    AppDataSourceNotProvided,
    #[error("App id not provided")]
    AppIdNotProvided,
    #[error("API key not provided")]
    ApiKeyNotProvided,
    #[error("API key id not provided")]
    ApiKeyIdNotProvided,
    #[error("SQS key not provided")]
    SqsKeyNotProvided,
    #[error("CSV append same schema not provided")]
    CsvAppendSameSchemaNotProvided,
    #[error("Allowed models not provided")]
    AllowedModelsNotProvided,
    #[error("Create timestamp not provided")]
    CreateTimestampNotProvided,
    #[error("Generated config not provided")]
    GeneratedConfigNotProvided,
    #[error("Onboarding status not provided")]
    OnboardingStatusNotProvided,
    #[error("Search enabled value not provided")]
    SearchEnabledNotProvided,
    #[error("Multimodal Search enabled value not provided")]
    MMSearchEnabledNotProvided,
}

/// Struct to represent the AppDocument
#[derive(Debug, Serialize)]
pub struct AppDocument {
    pub app_name: String,
    pub app_description: String,
    pub text_embedding_model: EmbeddingModel,
    pub multimodal_embedding_model: EmbeddingModel,
    pub app_datasource: AppDataSource,
    pub app_id: String,
    pub api_key: String,
    pub api_key_id: String,
    pub sqs_key: String,
    pub csv_append_same_schema: bool,
    pub allowed_models: Vec<LlmModel>,
    pub create_timestamp: String,
    pub generated_config: GeneratedConfig,
    pub onboarding_status: String,
    pub search_enabled: bool,
    pub mm_search_enabled: bool,
}

impl AppDocument {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        app_name: String,
        app_description: String,
        text_embedding_model: EmbeddingModel,
        multimodal_embedding_model: EmbeddingModel,
        app_datasource: AppDataSource,
        app_id: String,
        api_key: String,
        api_key_id: String,
        sqs_key: String,
        csv_append_same_schema: bool,
        allowed_models: Vec<LlmModel>,
        create_timestamp: String,
        generated_config: GeneratedConfig,
        onboarding_status: String,
        search_enabled: bool,
        mm_search_enabled: bool,
    ) -> Result<Self, AppDocumentCreationError> {
        Ok(AppDocument {
            app_name,
            app_description,
            text_embedding_model,
            multimodal_embedding_model,
            app_datasource,
            app_id,
            api_key,
            api_key_id,
            sqs_key,
            csv_append_same_schema,
            allowed_models,
            create_timestamp,
            generated_config,
            onboarding_status,
            search_enabled,
            mm_search_enabled,
        })
    }

    pub fn builder() -> AppDocumentBuilder {
        AppDocumentBuilder {
            app_name: None,
            app_description: None,
            text_embedding_model: None,
            multimodal_embedding_model: None,
            app_datasource: None,
            app_id: None,
            api_key: None,
            api_key_id: None,
            sqs_key: None,
            csv_append_same_schema: None,
            allowed_models: None,
            create_timestamp: None,
            generated_config: None,
            onboarding_status: None,
            search_enabled: None,
            mm_search_enabled: None,
        }
    }
}

/// Builder struct for the AppDocument
pub struct AppDocumentBuilder {
    app_name: Option<String>,
    app_description: Option<String>,
    text_embedding_model: Option<EmbeddingModel>,
    multimodal_embedding_model: Option<EmbeddingModel>,
    app_datasource: Option<AppDataSource>,
    app_id: Option<String>,
    api_key: Option<String>,
    api_key_id: Option<String>,
    sqs_key: Option<String>,
    csv_append_same_schema: Option<bool>,
    allowed_models: Option<Vec<LlmModel>>,
    create_timestamp: Option<String>,
    generated_config: Option<GeneratedConfig>,
    onboarding_status: Option<String>,
    search_enabled: Option<bool>,
    mm_search_enabled: Option<bool>,
}

impl AppDocumentBuilder {
    pub fn set_app_name(mut self, app_name: String) -> Self {
        self.app_name = Some(app_name);
        self
    }

    pub fn set_app_description(mut self, app_description: String) -> Self {
        self.app_description = Some(app_description);
        self
    }

    pub fn set_text_embedding_model(
        mut self,
        text_embedding_model: OnboardingEmbeddingModel,
    ) -> Self {
        self.text_embedding_model = Some(EmbeddingModel {
            dimension: text_embedding_model.dimension,
            model_id: text_embedding_model.model_id,
            platform: text_embedding_model.platform,
        });
        self
    }
    pub fn set_multimodal_embedding_model(
        mut self,
        multimodal_embedding_model: OnboardingEmbeddingModel,
    ) -> Self {
        self.multimodal_embedding_model = Some(EmbeddingModel {
            dimension: multimodal_embedding_model.dimension,
            model_id: multimodal_embedding_model.model_id,
            platform: multimodal_embedding_model.platform,
        });
        self
    }

    /// Function to set the app datasource. Iterates over the filestore and datastore data sources in the
    /// onboarding request and sets them in the app document to be stored in the DocumentDB.
    pub fn set_app_datasource(mut self, app_datasource: OnboardingAppDataSource) -> Self {
        self.app_datasource = Some(AppDataSource {
            filestore: {
                app_datasource
                    .filestore
                    .into_iter()
                    .map(|(k, v)| {
                        let filestore_data = v
                            .into_iter()
                            .map(|data_source| FileStore {
                                url: data_source.url,
                                hints: data_source
                                    .hints
                                    .into_iter()
                                    .map(|hint| Hint {
                                        prefix: hint.prefix,
                                        descriptions: hint.descriptions,
                                    })
                                    .collect(),
                            })
                            .collect();
                        (k, filestore_data)
                    })
                    .collect()
            },
            datastore: {
                app_datasource
                    .datastore
                    .into_iter()
                    .map(|(k, v)| {
                        let datastore_data = v
                            .into_iter()
                            .map(|data_source| DataStore {
                                host: data_source.host,
                                port: data_source.port,
                                username: data_source.username,
                                aws_service_name: data_source.aws_service_name,
                                secret_name: data_source.secret_name,
                                database: data_source.database,
                                db_type: data_source.db_type,
                                descriptions: data_source.descriptions,
                                region: data_source.region,
                                fact_phrases: data_source.fact_phrases,
                                fact_words: data_source.fact_words,
                                search_keywords: data_source.search_keywords,
                                summary: data_source.summary,
                                tables: data_source
                                    .tables
                                    .into_iter()
                                    .map(|table| Table {
                                        name: table.name,
                                        descriptions: table.descriptions,
                                        schema: table.schema,
                                        schema_json: table.schema_json,
                                        columns: Some(
                                            table
                                                .columns
                                                .clone()
                                                .unwrap_or_default()
                                                .into_iter()
                                                .map(|column| Column {
                                                    name: column.name,
                                                    descriptions: column.descriptions,
                                                })
                                                .collect(),
                                        ),
                                        sample_rows: match table.sample_rows {
                                            Some(sample_rows) => Some(SampleRows {
                                                top_rows: sample_rows.top_rows,
                                                random_rows: sample_rows.random_rows,
                                                bottom_rows: sample_rows.bottom_rows,
                                            }),
                                            None => None,
                                        },
                                        fact_phrases: table.fact_phrases,
                                        fact_words: table.fact_words,
                                        search_keywords: table.search_keywords,
                                        summary: table.summary,
                                    })
                                    .collect(),
                            })
                            .collect();
                        (k, datastore_data)
                    })
                    .collect()
            },
        });
        self
    }

    pub fn set_app_id(mut self, app_id: String) -> Self {
        self.app_id = Some(app_id);
        self
    }

    pub fn set_api_key(mut self, api_key: String) -> Self {
        self.api_key = Some(api_key);
        self
    }

    pub fn set_api_key_id(mut self, api_key_id: String) -> Self {
        self.api_key_id = Some(api_key_id);
        self
    }

    pub fn set_sqs_key(mut self, sqs_key: String) -> Self {
        self.sqs_key = Some(sqs_key);
        self
    }

    pub fn set_csv_append_same_schema(mut self, csv_append_same_schema: bool) -> Self {
        self.csv_append_same_schema = Some(csv_append_same_schema);
        self
    }

    pub fn set_allowed_models(mut self, allowed_models: Vec<OnboardingLlmModel>) -> Self {
        self.allowed_models = Some(
            allowed_models
                .into_iter()
                .map(|model| {
                    LlmModel::new(
                        model.name,
                        model.description,
                        model.model_id,
                        model.model_type,
                        model.secret_name,
                        model.secret_region,
                    )
                })
                .collect(),
        );
        self
    }

    pub fn set_create_timestamp(mut self, timestamp_format: String) -> Self {
        self.create_timestamp = Some(Utc::now().format(&timestamp_format).to_string());
        self
    }

    pub fn set_generated_config(mut self, app_state: &Arc<AppState>, app_name: String) -> Self {
        self.generated_config = Some(self.create_generated_config(app_state, &app_name));
        self
    }

    /// Function to create the generated config for the app document
    fn create_generated_config(
        &mut self,
        app_state: &Arc<AppState>,
        app_name: &String,
    ) -> GeneratedConfig {
        // Closure to set the service config to be used while creating the GeneratedConfig struct
        let set_service_config = |app_name: &String,
                                  collection: &str,
                                  retention: &str,
                                  s3_prefix: &str|
         -> ServiceConfig {
            ServiceConfig {
                collection_name_prefix: format!("{}-{}", app_name, collection),
                retention: retention.to_string(),
                s3_storage_prefix: s3_prefix.to_string(),
            }
        };

        GeneratedConfig {
            s3_prefix: app_state.app_settings.global_artifact.clone(),
            knowledge_graph_config: KnowledgeGraphConfig {
                vectordb_config: VectorDbClientConfig {
                    text_collection_name_prefix: app_state
                        .app_settings
                        .app_generated_config
                        .knowledge_graph_config
                        .vectordb_config
                        .text_collection_name_prefix
                        .clone(),
                    multimodal_collection_name_prefix: Some(
                        app_state
                            .app_settings
                            .app_generated_config
                            .knowledge_graph_config
                            .vectordb_config
                            .multimodal_collection_name_prefix
                            .clone(),
                    ),
                    general_collection_name_prefix: app_state
                        .app_settings
                        .app_generated_config
                        .knowledge_graph_config
                        .vectordb_config
                        .general_collection_name_prefix
                        .clone(),
                    error_collection_name_prefix: app_state
                        .app_settings
                        .app_generated_config
                        .knowledge_graph_config
                        .vectordb_config
                        .error_collection_name_prefix
                        .clone(),
                    insight_collection_name_prefix: app_state
                        .app_settings
                        .app_generated_config
                        .knowledge_graph_config
                        .vectordb_config
                        .insight_collection_name_prefix
                        .clone(),
                    session_history_collection_name_prefix: app_state
                        .app_settings
                        .app_generated_config
                        .knowledge_graph_config
                        .vectordb_config
                        .session_history_collection_name_prefix
                        .clone(),
                    retention: app_state
                        .app_settings
                        .app_generated_config
                        .knowledge_graph_config
                        .vectordb_config
                        .retention
                        .clone(),
                    s3_storage_prefix: app_state
                        .app_settings
                        .app_generated_config
                        .knowledge_graph_config
                        .vectordb_config
                        .s3_storage_prefix
                        .clone(),
                },
            },
            parser_config: ParserConfig {
                s3_storage_prefix: app_state
                    .app_settings
                    .app_generated_config
                    .knowledge_graph_config
                    .parser_config
                    .s3_storage_prefix
                    .clone(),
            },
            logging: set_service_config(
                app_name,
                &app_state
                    .app_settings
                    .app_generated_config
                    .knowledge_graph_config
                    .logging
                    .collection,
                &app_state
                    .app_settings
                    .app_generated_config
                    .knowledge_graph_config
                    .logging
                    .retention,
                &app_state
                    .app_settings
                    .app_generated_config
                    .knowledge_graph_config
                    .logging
                    .s3_prefix,
            ),
            audit: set_service_config(
                app_name,
                &app_state
                    .app_settings
                    .app_generated_config
                    .knowledge_graph_config
                    .audit
                    .collection,
                &app_state
                    .app_settings
                    .app_generated_config
                    .knowledge_graph_config
                    .audit
                    .retention,
                &app_state
                    .app_settings
                    .app_generated_config
                    .knowledge_graph_config
                    .audit
                    .s3_prefix,
            ),
            metric: set_service_config(
                app_name,
                &app_state
                    .app_settings
                    .app_generated_config
                    .knowledge_graph_config
                    .metric
                    .collection,
                &app_state
                    .app_settings
                    .app_generated_config
                    .knowledge_graph_config
                    .metric
                    .retention,
                &app_state
                    .app_settings
                    .app_generated_config
                    .knowledge_graph_config
                    .metric
                    .s3_prefix,
            ),
        }
    }

    pub fn set_onboarding_status(mut self, onboarding_status: String) -> Self {
        self.onboarding_status = Some(onboarding_status);
        self
    }

    pub fn set_search_enabled(mut self, search_enabled: bool) -> Self {
        self.search_enabled = Some(search_enabled);
        self
    }

    pub fn set_mm_search_enabled(mut self, mm_search_enabled: bool) -> Self {
        self.mm_search_enabled = Some(mm_search_enabled);
        self
    }

    pub fn build(self) -> Result<AppDocument, AppDocumentCreationError> {
        let app_document = AppDocument::new(
            self.app_name
                .ok_or(AppDocumentCreationError::AppNameNotProvided)?,
            self.app_description
                .ok_or(AppDocumentCreationError::AppDescriptionNotProvided)?,
            self.text_embedding_model
                .ok_or(AppDocumentCreationError::AppTextEmbeddingModelNotProvided)?,
            self.multimodal_embedding_model
                .ok_or(AppDocumentCreationError::AppMultimodalEmbeddingModelNotProvided)?,
            self.app_datasource
                .ok_or(AppDocumentCreationError::AppDataSourceNotProvided)?,
            self.app_id
                .ok_or(AppDocumentCreationError::AppIdNotProvided)?,
            self.api_key
                .ok_or(AppDocumentCreationError::ApiKeyNotProvided)?,
            self.api_key_id
                .ok_or(AppDocumentCreationError::ApiKeyIdNotProvided)?,
            self.sqs_key
                .ok_or(AppDocumentCreationError::SqsKeyNotProvided)?,
            self.csv_append_same_schema
                .ok_or(AppDocumentCreationError::CsvAppendSameSchemaNotProvided)?,
            self.allowed_models
                .ok_or(AppDocumentCreationError::AllowedModelsNotProvided)?,
            self.create_timestamp
                .ok_or(AppDocumentCreationError::CreateTimestampNotProvided)?,
            self.generated_config
                .ok_or(AppDocumentCreationError::GeneratedConfigNotProvided)?,
            self.onboarding_status
                .ok_or(AppDocumentCreationError::OnboardingStatusNotProvided)?,
            self.search_enabled
                .ok_or(AppDocumentCreationError::SearchEnabledNotProvided)?,
            self.mm_search_enabled
                .ok_or(AppDocumentCreationError::MMSearchEnabledNotProvided)?,
        )?;
        Ok(app_document)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::onboarding::schema::app_onboarding_request::AppDataSource;
    use anyhow::Result;
    use std::fs::File;
    use std::io::Read;
    use tokio::runtime::Runtime;

    fn read_app_datasource_from_file() -> Result<AppDataSource> {
        let mut file = File::open("src/test/app_data_source.json")?;
        let mut buff = String::new();
        file.read_to_string(&mut buff)?;
        let app_datasource: AppDataSource = serde_json::from_str(&buff)?;
        Ok(app_datasource)
    }

    #[test]
    fn test_success_set_app_name() {
        let builder = AppDocument::builder().set_app_name("TestApp".to_string());
        assert_eq!(builder.app_name, Some("TestApp".to_string()));
    }

    #[test]
    fn test_failure_set_app_name() {
        let builder = AppDocument::builder().set_app_name("TestApp".to_string());
        // Fail if the app name is set incorrectly
        assert_ne!(builder.app_name, Some("WrongTestApp".to_string()));
    }

    #[test]
    fn test_success_set_app_description() {
        let builder = AppDocument::builder().set_app_description("TestDescription".to_string());
        assert_eq!(builder.app_description, Some("TestDescription".to_string()));
    }

    #[test]
    fn test_failure_set_app_description() {
        let builder = AppDocument::builder().set_app_description("TestDescription".to_string());
        assert_ne!(
            builder.app_description,
            Some("WrongTestDescription".to_string())
        );
    }
    #[test]
    fn test_success_set_text_embedding_model() {
        let builder = AppDocument::builder().set_text_embedding_model(OnboardingEmbeddingModel {
            dimension: 100,
            model_id: "TestModelId".to_string(),
            platform: "TestPlatform".to_string(),
        });
        let res = builder.text_embedding_model.unwrap();
        assert_eq!(res.dimension, 100);
        assert_eq!(res.model_id, "TestModelId".to_string());
        assert_eq!(res.platform, "TestPlatform".to_string());
    }
    #[test]
    fn test_failure_set_text_embedding_model() {
        let builder = AppDocument::builder().set_text_embedding_model(OnboardingEmbeddingModel {
            dimension: 10,
            model_id: "TestModeld".to_string(),
            platform: "TestPlatf".to_string(),
        });
        let res = builder.text_embedding_model.unwrap();
        assert_ne!(res.dimension, 100);
        assert_ne!(res.model_id, "TestModelId".to_string());
        assert_ne!(res.platform, "TestPlatform".to_string());
    }
    #[test]
    fn test_success_set_multi_embedding_model() {
        let builder =
            AppDocument::builder().set_multimodal_embedding_model(OnboardingEmbeddingModel {
                dimension: 100,
                model_id: "TestModelId".to_string(),
                platform: "TestPlatform".to_string(),
            });
        let res = builder.multimodal_embedding_model.unwrap();
        assert_eq!(res.dimension, 100);
        assert_eq!(res.model_id, "TestModelId".to_string());
        assert_eq!(res.platform, "TestPlatform".to_string());
    }

    #[test]
    fn test_failure_set_multi_embedding_model() {
        let builder =
            AppDocument::builder().set_multimodal_embedding_model(OnboardingEmbeddingModel {
                dimension: 10,
                model_id: "TestModel".to_string(),
                platform: "TestPlatform".to_string(),
            });
        let res = builder.multimodal_embedding_model.unwrap();
        assert_ne!(res.dimension, 100);
        assert_ne!(res.model_id, "TestModId".to_string());
        assert_ne!(res.platform, "Testatform".to_string());
    }

    #[test]
    fn test_success_set_app_id() {
        let builder = AppDocument::builder().set_app_id("TestAppId".to_string());
        assert_eq!(builder.app_id, Some("TestAppId".to_string()));
    }

    #[test]
    fn test_failure_set_app_id() {
        let builder = AppDocument::builder().set_app_id("TestAppId".to_string());
        assert_ne!(builder.app_id, Some("WrongAppId".to_string()));
    }

    #[test]
    fn test_success_set_api_key() {
        let builder = AppDocument::builder().set_api_key("TestApiKey".to_string());
        assert_eq!(builder.api_key, Some("TestApiKey".to_string()));
    }

    #[test]
    fn test_failure_set_api_key() {
        let builder = AppDocument::builder().set_api_key("TestApiKey".to_string());
        assert_ne!(builder.api_key, Some("WrongApiKey".to_string()));
    }

    #[test]
    fn test_success_set_api_key_id() {
        let builder = AppDocument::builder().set_api_key_id("TestApiKeyId".to_string());
        assert_eq!(builder.api_key_id, Some("TestApiKeyId".to_string()));
    }

    #[test]
    fn test_failure_set_api_key_id() {
        let builder = AppDocument::builder().set_api_key("TestApiKeyId".to_string());
        assert_ne!(builder.api_key_id, Some("WrongApiKeyId".to_string()));
    }

    #[test]
    fn test_success_set_sqs_key() {
        let builder = AppDocument::builder().set_sqs_key("TestSqsKey".to_string());
        assert_eq!(builder.sqs_key, Some("TestSqsKey".to_string()));
    }
    #[test]
    fn test_failure_set_sqs_key() {
        let builder = AppDocument::builder().set_sqs_key("TestSqsKey".to_string());
        assert_ne!(builder.sqs_key, Some("WrongSqsKey".to_string()));
    }

    #[test]
    fn test_success_set_csv_append_same_schema() {
        let builder = AppDocument::builder().set_csv_append_same_schema(true);
        assert_eq!(builder.csv_append_same_schema, Some(true));
    }
    #[test]
    fn test_success_set_csv_append_same_schema_false() {
        let builder = AppDocument::builder().set_csv_append_same_schema(false);
        assert_eq!(builder.csv_append_same_schema, Some(false));
    }

    #[test]
    fn test_success_set_create_timestamp() {
        let builder = AppDocument::builder().set_create_timestamp("%Y-%m-%d".to_string());
        assert_eq!(
            builder.create_timestamp,
            Some(Utc::now().format("%Y-%m-%d").to_string())
        );
    }

    #[test]
    fn test_failure_set_create_timestamp() {
        let builder = AppDocument::builder().set_create_timestamp("%Y-%m-%d".to_string());
        assert_ne!(builder.create_timestamp, Some("WrongTimestamp".to_string()));
    }

    #[test]
    fn test_success_create_generated_config() {
        let rt = Runtime::new().unwrap();

        rt.block_on(async {
            // Create a dev AppState and app_name
            let app_state = crate::tests::test_get_appstate().await.unwrap();
            let app_name = "test_app".to_string();

            // Call the function and check if the generated config is set
            let builder = AppDocument::builder().set_generated_config(&app_state, app_name);
            assert_eq!(builder.generated_config.is_some(), true);
        });
    }

    #[test]
    fn test_success_set_onboarding_status() {
        let builder = AppDocument::builder().set_onboarding_status("In Progress".to_string());
        assert_eq!(builder.onboarding_status, Some("In Progress".to_string()));
    }

    #[test]
    fn test_failure_set_onboarding_status() {
        let builder = AppDocument::builder().set_onboarding_status("In Progress".to_string());
        assert_ne!(
            builder.onboarding_status,
            Some("WrongOnboardingStatus".to_string())
        );
    }

    #[test]
    fn test_success_set_search_enabled() {
        let builder = AppDocument::builder().set_search_enabled(true);
        assert_eq!(builder.search_enabled, Some(true));
    }

    #[test]
    fn test_failure_set_search_enabled() {
        let builder = AppDocument::builder().set_search_enabled(true);
        assert_ne!(builder.search_enabled, Some(false));
    }

    #[test]
    fn test_success_set_mm_search_enabled() {
        let builder = AppDocument::builder().set_mm_search_enabled(true);
        assert_eq!(builder.mm_search_enabled, Some(true));
    }

    #[test]
    fn test_failure_set_mm_search_enabled() {
        let builder = AppDocument::builder().set_mm_search_enabled(true);
        assert_ne!(builder.mm_search_enabled, Some(false));
    }

    #[test]
    fn test_failure_build_missing_app_name() {
        let builder = AppDocument::builder();
        let result = builder.build();
        assert_eq!(
            result.unwrap_err(),
            AppDocumentCreationError::AppNameNotProvided
        );
    }

    #[test]
    fn test_failure_build_missing_app_description() {
        let builder = AppDocument::builder().set_app_name("TestApp".to_string());
        let result = builder.build();
        assert_eq!(
            result.unwrap_err(),
            AppDocumentCreationError::AppDescriptionNotProvided
        );
    }
    #[test]
    fn test_failure_build_missing_text_embedding_model() {
        let builder = AppDocument::builder()
            .set_app_name("TestApp".to_string())
            .set_app_description("TestDescription".to_string());
        let result = builder.build();
        assert_eq!(
            result.unwrap_err(),
            AppDocumentCreationError::AppTextEmbeddingModelNotProvided
        );
    }
    #[test]
    fn test_failure_build_missing_multimodal_embedding_model() {
        let builder = AppDocument::builder()
            .set_app_name("TestApp".to_string())
            .set_app_description("TestDescription".to_string())
            .set_text_embedding_model(OnboardingEmbeddingModel {
                dimension: 100,
                model_id: "TestModelId".to_string(),
                platform: "TestPlatform".to_string(),
            });
        let result = builder.build();
        assert_eq!(
            result.unwrap_err(),
            AppDocumentCreationError::AppMultimodalEmbeddingModelNotProvided
        );
    }

    #[test]
    fn test_failure_build_missing_app_datasource() {
        let builder = AppDocument::builder()
            .set_app_name("TestApp".to_string())
            .set_app_description("TestDescription".to_string())
            .set_text_embedding_model(OnboardingEmbeddingModel {
                dimension: 100,
                model_id: "TestModelId".to_string(),
                platform: "TestPlatform".to_string(),
            })
            .set_multimodal_embedding_model(OnboardingEmbeddingModel {
                dimension: 100,
                model_id: "TestModelId".to_string(),
                platform: "TestPlatform".to_string(),
            });
        let result = builder.build();
        assert_eq!(
            result.unwrap_err(),
            AppDocumentCreationError::AppDataSourceNotProvided
        );
    }

    #[test]
    fn test_failure_build_missing_app_id() {
        let app_data_source = read_app_datasource_from_file().unwrap();
        let builder = AppDocument::builder()
            .set_app_name("TestApp".to_string())
            .set_app_description("TestDescription".to_string())
            .set_text_embedding_model(OnboardingEmbeddingModel {
                dimension: 100,
                model_id: "TestModelId".to_string(),
                platform: "TestPlatform".to_string(),
            })
            .set_multimodal_embedding_model(OnboardingEmbeddingModel {
                dimension: 100,
                model_id: "TestModelId".to_string(),
                platform: "TestPlatform".to_string(),
            })
            .set_app_datasource(app_data_source);
        let result = builder.build();
        assert_eq!(
            result.unwrap_err(),
            AppDocumentCreationError::AppIdNotProvided
        );
    }

    #[test]
    fn test_failure_build_missing_api_key() {
        let app_data_source = read_app_datasource_from_file().unwrap();
        let builder = AppDocument::builder()
            .set_app_name("TestApp".to_string())
            .set_app_description("TestDescription".to_string())
            .set_text_embedding_model(OnboardingEmbeddingModel {
                dimension: 100,
                model_id: "TestModelId".to_string(),
                platform: "TestPlatform".to_string(),
            })
            .set_multimodal_embedding_model(OnboardingEmbeddingModel {
                dimension: 100,
                model_id: "TestModelId".to_string(),
                platform: "TestPlatform".to_string(),
            })
            .set_app_datasource(app_data_source)
            .set_app_id("TestAppId".to_string());
        let result = builder.build();
        assert_eq!(
            result.unwrap_err(),
            AppDocumentCreationError::ApiKeyNotProvided
        );
    }

    #[test]
    fn test_failure_build_missing_api_key_id() {
        let app_data_source = read_app_datasource_from_file().unwrap();
        let builder = AppDocument::builder()
            .set_app_name("TestApp".to_string())
            .set_app_description("TestDescription".to_string())
            .set_text_embedding_model(OnboardingEmbeddingModel {
                dimension: 100,
                model_id: "TestModelId".to_string(),
                platform: "TestPlatform".to_string(),
            })
            .set_multimodal_embedding_model(OnboardingEmbeddingModel {
                dimension: 100,
                model_id: "TestModelId".to_string(),
                platform: "TestPlatform".to_string(),
            })
            .set_app_datasource(app_data_source)
            .set_app_id("TestAppId".to_string())
            .set_api_key("TestApiKey".to_string());
        let result = builder.build();
        assert_eq!(
            result.unwrap_err(),
            AppDocumentCreationError::ApiKeyIdNotProvided
        );
    }

    #[test]
    fn test_failure_build_missing_sqs_key() {
        let app_data_source = read_app_datasource_from_file().unwrap();
        let builder = AppDocument::builder()
            .set_app_name("TestApp".to_string())
            .set_app_description("TestDescription".to_string())
            .set_text_embedding_model(OnboardingEmbeddingModel {
                dimension: 100,
                model_id: "TestModelId".to_string(),
                platform: "TestPlatform".to_string(),
            })
            .set_multimodal_embedding_model(OnboardingEmbeddingModel {
                dimension: 100,
                model_id: "TestModelId".to_string(),
                platform: "TestPlatform".to_string(),
            })
            .set_app_datasource(app_data_source)
            .set_app_id("TestAppId".to_string())
            .set_api_key("TestApiKey".to_string())
            .set_api_key_id("TestApiKeyId".to_string());
        let result = builder.build();
        assert_eq!(
            result.unwrap_err(),
            AppDocumentCreationError::SqsKeyNotProvided
        );
    }

    #[test]
    fn test_failure_build_missing_allowed_models() {
        let app_data_source = read_app_datasource_from_file().unwrap();
        let builder = AppDocument::builder()
            .set_app_name("TestApp".to_string())
            .set_app_description("TestDescription".to_string())
            .set_text_embedding_model(OnboardingEmbeddingModel {
                dimension: 100,
                model_id: "TestModelId".to_string(),
                platform: "TestPlatform".to_string(),
            })
            .set_multimodal_embedding_model(OnboardingEmbeddingModel {
                dimension: 100,
                model_id: "TestModelId".to_string(),
                platform: "TestPlatform".to_string(),
            })
            .set_app_datasource(app_data_source)
            .set_app_id("TestAppId".to_string())
            .set_api_key("TestApiKey".to_string())
            .set_api_key_id("TestApiKeyId".to_string())
            .set_sqs_key("TestSqsKey".to_string())
            .set_csv_append_same_schema(true);
        let result = builder.build();
        assert_eq!(
            result.unwrap_err(),
            AppDocumentCreationError::AllowedModelsNotProvided
        );
    }

    #[test]
    fn test_failure_build_missing_create_timestamp() {
        let app_data_source = read_app_datasource_from_file().unwrap();
        let builder = AppDocument::builder()
            .set_app_name("TestApp".to_string())
            .set_app_description("TestDescription".to_string())
            .set_text_embedding_model(OnboardingEmbeddingModel {
                dimension: 100,
                model_id: "TestModelId".to_string(),
                platform: "TestPlatform".to_string(),
            })
            .set_multimodal_embedding_model(OnboardingEmbeddingModel {
                dimension: 100,
                model_id: "TestModelId".to_string(),
                platform: "TestPlatform".to_string(),
            })
            .set_app_datasource(app_data_source)
            .set_app_id("TestAppId".to_string())
            .set_api_key("TestApiKey".to_string())
            .set_api_key_id("TestApiKeyId".to_string())
            .set_sqs_key("TestSqsKey".to_string())
            .set_csv_append_same_schema(true)
            .set_allowed_models(vec![]);
        let result = builder.build();
        assert_eq!(
            result.unwrap_err(),
            AppDocumentCreationError::CreateTimestampNotProvided
        );
    }

    #[test]
    fn test_failure_build_missing_generated_config() {
        let app_data_source = read_app_datasource_from_file().unwrap();
        let builder = AppDocument::builder()
            .set_app_name("TestApp".to_string())
            .set_app_description("TestDescription".to_string())
            .set_text_embedding_model(OnboardingEmbeddingModel {
                dimension: 100,
                model_id: "TestModelId".to_string(),
                platform: "TestPlatform".to_string(),
            })
            .set_multimodal_embedding_model(OnboardingEmbeddingModel {
                dimension: 100,
                model_id: "TestModelId".to_string(),
                platform: "TestPlatform".to_string(),
            })
            .set_app_datasource(app_data_source)
            .set_app_id("TestAppId".to_string())
            .set_api_key("TestApiKey".to_string())
            .set_api_key_id("TestApiKeyId".to_string())
            .set_sqs_key("TestSqsKey".to_string())
            .set_csv_append_same_schema(true)
            .set_allowed_models(vec![])
            .set_create_timestamp("TestTimestamp".to_string());
        let result = builder.build();
        assert_eq!(
            result.unwrap_err(),
            AppDocumentCreationError::GeneratedConfigNotProvided
        );
    }
}
