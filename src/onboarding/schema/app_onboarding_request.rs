/*
 * Created Date:  Mar 17, 2024
 * -----
 * Copyright (c) 2024 Tresle.ai or its affiliates. All Rights Reserved.
 */
//! This module contains the schema for the app onboarding request

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use utoipa::ToSchema;

#[derive(Serialize, Deserialize, Debug, Clone, ToSchema, PartialEq)]
pub struct OnboardingRequest {
    pub app_name: String,
    pub app_description: String,
    pub text_embedding_model: EmbeddingModel,
    pub multimodal_embedding_model: EmbeddingModel,
    pub csv_append_same_schema: bool,
    pub allowed_models: Vec<LlmModel>,
    pub app_datasource: AppDataSource,
}

#[derive(Serialize, Deserialize, Debug, Clone, ToSchema, PartialEq)]
pub struct EmbeddingModel {
    pub dimension: i32,
    pub model_id: String,
    pub platform: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, ToSchema, PartialEq)]
pub struct LlmModel {
    pub name: String,
    pub description: String,
    pub model_id: String,
    pub model_type: String,
    pub secret_name: Option<String>,
    pub secret_region: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, ToSchema, PartialEq)]
pub struct AppDataSource {
    pub filestore: HashMap<String, Vec<FileStore>>,
    pub datastore: HashMap<String, Vec<DataStore>>,
}

#[derive(Serialize, Deserialize, Debug, Clone, ToSchema, PartialEq)]
pub struct FileStore {
    pub url: String,
    pub hints: Vec<Hint>,
}

#[derive(Serialize, Deserialize, Debug, Clone, ToSchema, PartialEq)]
pub struct Hint {
    pub prefix: String,
    pub descriptions: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, ToSchema, PartialEq)]
pub struct DataStore {
    pub host: String,
    pub port: String,
    pub username: Option<String>,
    pub secret_name: Option<String>,
    pub aws_service_name: Option<String>,
    pub database: String,
    pub db_type: String,
    pub descriptions: Option<String>,
    pub tables: Vec<Table>,
    pub region: Option<String>,
    pub fact_phrases: Option<Vec<String>>,
    pub fact_words: Option<Vec<String>>,
    pub search_keywords: Option<Vec<String>>,
    pub summary: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, ToSchema, PartialEq)]
pub struct Table {
    pub name: String,
    pub descriptions: String,
    pub schema: Option<String>,
    pub schema_json: Option<serde_json::Value>,
    pub columns: Option<Vec<Column>>,
    pub sample_rows: Option<SampleRows>,
    pub fact_phrases: Option<Vec<String>>,
    pub fact_words: Option<Vec<String>>,
    pub search_keywords: Option<Vec<String>>,
    pub summary: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, ToSchema, PartialEq)]
pub struct Column {
    pub name: String,
    pub descriptions: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, ToSchema, PartialEq)]
pub struct SampleRows {
    pub top_rows: Vec<String>,
    pub random_rows: Vec<String>,
    pub bottom_rows: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json;

    #[test]
    fn test_success_onboarding_request_serialization_and_deserialization() {
        let onboarding_request = OnboardingRequest {
            app_name: "Test App".to_string(),
            app_description: "This is a test app".to_string(),
            text_embedding_model: EmbeddingModel {
                dimension: 100,
                model_id: "model1".to_string(),
                platform: "platform1".to_string(),
            },
            multimodal_embedding_model: EmbeddingModel {
                dimension: 200,
                model_id: "model2".to_string(),
                platform: "platform2".to_string(),
            },
            csv_append_same_schema: false,
            allowed_models: vec![],
            app_datasource: AppDataSource {
                filestore: HashMap::new(),
                datastore: HashMap::new(),
            },
        };

        let serialized = serde_json::to_string(&onboarding_request).unwrap();
        let deserialized: OnboardingRequest = serde_json::from_str(&serialized).unwrap();

        assert_eq!(onboarding_request, deserialized);
    }

    #[test]
    fn test_success_llm_model_serialization_and_deserialization() {
        let llm_model = LlmModel {
            name: "Test Model".to_string(),
            description: "This is a test model".to_string(),
            model_id: "model1".to_string(),
            model_type: "type1".to_string(),
            secret_name: None,
            secret_region: None,
        };

        let serialized = serde_json::to_string(&llm_model).unwrap();
        let deserialized: LlmModel = serde_json::from_str(&serialized).unwrap();

        assert_eq!(llm_model, deserialized);
    }

    #[test]
    fn test_success_app_datasource_serialization_and_deserialization() {
        let mut filestore = HashMap::new();
        filestore.insert("key1".to_string(), vec![]);
        filestore.insert("key2".to_string(), vec![]);

        let mut datastore = HashMap::new();
        datastore.insert("key1".to_string(), vec![]);
        datastore.insert("key2".to_string(), vec![]);

        let app_datasource = AppDataSource {
            filestore,
            datastore,
        };

        let serialized = serde_json::to_string(&app_datasource).unwrap();
        let deserialized = serde_json::from_str(&serialized).unwrap();

        assert_eq!(app_datasource, deserialized);
    }

    #[test]
    fn test_success_filestore_serialization_and_deserialization() {
        let filestore = FileStore {
            url: "https://example.com".to_string(),
            hints: vec![],
        };

        let serialized = serde_json::to_string(&filestore).unwrap();
        let deserialized: FileStore = serde_json::from_str(&serialized).unwrap();

        assert_eq!(filestore, deserialized);
    }

    #[test]
    fn test_success_hint_serialization_and_deserialization() {
        let hint = Hint {
            prefix: "prefix".to_string(),
            descriptions: "This is a test hint".to_string(),
        };

        let serialized = serde_json::to_string(&hint).unwrap();
        let deserialized: Hint = serde_json::from_str(&serialized).unwrap();

        assert_eq!(hint, deserialized);
    }

    #[test]
    fn test_success_datastore_serialization_and_deserialization() {
        let datastore = DataStore {
            host: "localhost".to_string(),
            port: "5432".to_string(),
            username: None,
            secret_name: None,
            aws_service_name: None,
            database: "test_db".to_string(),
            db_type: "postgres".to_string(),
            descriptions: None,
            tables: vec![],
            region: None,
            fact_phrases: None,
            fact_words: None,
            search_keywords: None,
            summary: None,
        };

        let serialized = serde_json::to_string(&datastore).unwrap();
        let deserialized: DataStore = serde_json::from_str(&serialized).unwrap();

        assert_eq!(datastore, deserialized);
    }

    #[test]
    fn test_success_table_serialization_and_deserialization() {
        let table = Table {
            name: "test_table".to_string(),
            descriptions: "This is a test table".to_string(),
            schema: Some("public".to_string()),
            schema_json: None,
            columns: Some(vec![]),
            sample_rows: None,
            fact_phrases: None,
            fact_words: None,
            search_keywords: None,
            summary: None,
        };

        let serialized = serde_json::to_string(&table).unwrap();
        let deserialized: Table = serde_json::from_str(&serialized).unwrap();

        assert_eq!(table, deserialized);
    }

    #[test]
    fn test_success_column_serialization_and_deserialization() {
        let column = Column {
            name: "test_column".to_string(),
            descriptions: "This is a test column".to_string(),
        };

        let serialized = serde_json::to_string(&column).unwrap();
        let deserialized: Column = serde_json::from_str(&serialized).unwrap();

        assert_eq!(column, deserialized);
    }

    #[test]
    fn test_success_sample_rows_serialization_and_deserialization() {
        let sample_rows = SampleRows {
            top_rows: vec![],
            random_rows: vec![],
            bottom_rows: vec![],
        };

        let serialized = serde_json::to_string(&sample_rows).unwrap();
        let deserialized: SampleRows = serde_json::from_str(&serialized).unwrap();

        assert_eq!(sample_rows, deserialized);
    }
}
