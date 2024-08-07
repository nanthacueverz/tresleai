/*
 * Created Date:  Mar 17, 2024
 * -----
 * Copyright (c) 2024 Tresle.ai or its affiliates. All Rights Reserved.
 */
//! This module contains the schema required for the different admin_ui_api handlers
//! The schema is used to define the request and response bodies for the different admin_ui_api handlers.
//!

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

//schema to capture user & t&c information
#[derive(Serialize, Deserialize, Debug, Default, ToSchema)]
pub struct CaptureUserSchema {
    pub user_name: String,
    pub ui_type: String,
}

//schema to capture user & t&c information
#[derive(Serialize, Deserialize, Debug, Default)]
pub struct CaptureTcSchema {
    pub is_tc: bool,
}
/// Optional query parameters
#[derive(Serialize, Deserialize, Debug, Default)]
pub struct QueryParams {
    pub page: Option<usize>,
    pub limit: Option<usize>,
    pub app_name: Option<String>,
    pub is_update: Option<bool>,
    pub search_enabled: Option<bool>,
    pub reference_id: Option<String>,
    pub knowledge_node_type: Option<String>,
    pub start_timestamp: Option<String>,
    pub end_timestamp: Option<String>,
    pub utc_start_timestamp: Option<DateTime<Utc>>,
    pub utc_end_timestamp: Option<DateTime<Utc>>,
}

/// Schema for the fetched apps
#[derive(Serialize, Deserialize, Debug, Default)]
pub struct AppListFetchSchema {
    pub app_name: String,
    pub app_description: String,
    pub api_key: String,
    pub onboarding_status: String,
    pub search_enabled: bool,
}

/// Schema for deletion response
#[allow(non_snake_case)]
#[derive(Serialize, Deserialize, Debug, Default)]
pub struct DeleteResponse {
    pub deletedCount: i64,
}

/// Schema for update response
#[allow(non_snake_case)]
#[derive(Serialize, Deserialize, Debug, Default)]
pub struct UpdateResponse {
    pub matchedCount: u64,
    pub modifiedCount: u64,
}

/// Schema for counts of knowledge nodes and errors while processing/extracting them
#[derive(Serialize, Deserialize, Debug, Default)]
pub struct Counts {
    pub knowledge_node_errors: u64,
    pub knowledge_node_file_store: u64,
    pub knowledge_node_data_store: u64,
}

/// Schema for the knowledge nodes chart count
#[allow(non_snake_case)]
#[derive(Serialize, Deserialize, Debug, Default)]
pub struct KnowledgeNodeChartCount {
    pub count: i32,
    pub indexed_at: String,
}

/// Schema for a graph item for knowledge node chart
#[derive(Serialize, Deserialize, Debug, Default)]
pub struct GraphItem {
    pub count: String,
    pub indexed_at: String,
}

/// Schema for the knowledge nodes chart API response
#[derive(Serialize, Deserialize, Debug, Default)]
pub struct NodesChartApiResponse {
    pub count: String,
    pub graph_items: Vec<GraphItem>,
    pub graph_interval: String,
}

impl From<KnowledgeNodeChartCount> for GraphItem {
    fn from(item: KnowledgeNodeChartCount) -> Self {
        GraphItem {
            count: item.count.to_string(),
            indexed_at: item.indexed_at,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[allow(non_snake_case)]
    fn test_success_QueryParams() {
        let qp = QueryParams {
            page: Some(1),
            limit: Some(10),
            app_name: Some("app_name".to_string()),
            is_update: Some(true),
            search_enabled: Some(true),
            reference_id: Some("reference_id".to_string()),
            knowledge_node_type: Some("knowledge_node_type".to_string()),
            start_timestamp: Some("start_timestamp".to_string()),
            end_timestamp: Some("end_timestamp".to_string()),
            utc_start_timestamp: Some(Utc::now()),
            utc_end_timestamp: Some(Utc::now()),
        };
        assert_eq!(qp.app_name, Some("app_name".to_string()));
        assert_eq!(qp.page, Some(1));

        let json_string = serde_json::to_string(&qp).unwrap();
        let deserialized: QueryParams = serde_json::from_str(&json_string).unwrap();
        assert_eq!(deserialized.app_name, Some("app_name".to_string()));
        println!("Now {:?} will print!", qp);

        let _qp2 = QueryParams::default();
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_success_QueryParams_all_none_fields_accepted() {
        let qp = QueryParams {
            page: None,
            limit: None,
            app_name: None,
            is_update: None,
            search_enabled: None,
            reference_id: None,
            knowledge_node_type: None,
            start_timestamp: None,
            end_timestamp: None,
            utc_start_timestamp: None,
            utc_end_timestamp: None,
        };
        assert_eq!(qp.app_name, None);
        assert_eq!(qp.page, None);

        let json_string = serde_json::to_string(&qp).unwrap();
        let deserialized: QueryParams = serde_json::from_str(&json_string).unwrap();
        assert_eq!(deserialized.app_name, None);
        println!("Now {:?} will print!", qp);

        let _qp2 = QueryParams::default();
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_success_AppListFetchSchema() {
        let appList = AppListFetchSchema {
            app_name: "app_name".to_string(),
            app_description: "app_description".to_string(),
            api_key: "api_key".to_string(),
            onboarding_status: "onboarding_status".to_string(),
            search_enabled: false,
        };
        assert_eq!(appList.app_name, "app_name".to_string());

        let json_string = serde_json::to_string(&appList).unwrap();
        let deserialized: AppListFetchSchema = serde_json::from_str(&json_string).unwrap();
        assert_eq!(deserialized.app_name, "app_name".to_string());
        println!("Now {:?} will print!", appList);

        let _appList2 = AppListFetchSchema::default();
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_success_DeleteResponse() {
        let resp = DeleteResponse { deletedCount: 1 };
        assert_eq!(resp.deletedCount, 1);

        let json_string = serde_json::to_string(&resp).unwrap();
        let deserialized: DeleteResponse = serde_json::from_str(&json_string).unwrap();
        assert_eq!(deserialized.deletedCount, 1);
        println!("Now {:?} will print!", resp);

        let _resp2 = DeleteResponse::default();
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_success_UpdateResponse() {
        let resp = UpdateResponse {
            matchedCount: 1,
            modifiedCount: 1,
        };
        assert_eq!(resp.matchedCount, 1);

        let json_string = serde_json::to_string(&resp).unwrap();
        let deserialized: UpdateResponse = serde_json::from_str(&json_string).unwrap();
        assert_eq!(deserialized.matchedCount, 1);
        println!("Now {:?} will print!", resp);

        let _resp2 = UpdateResponse::default();
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_success_Counts() {
        let counts = Counts {
            knowledge_node_errors: 1,
            knowledge_node_file_store: 1,
            knowledge_node_data_store: 1,
        };
        assert_eq!(counts.knowledge_node_errors, 1);

        let json_string = serde_json::to_string(&counts).unwrap();
        let deserialized: Counts = serde_json::from_str(&json_string).unwrap();
        assert_eq!(deserialized.knowledge_node_errors, 1);
        println!("Now {:?} will print!", counts);
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_success_KnowledgeNodeChartCount() {
        let knc = KnowledgeNodeChartCount {
            count: 1,
            indexed_at: "indexed_at".to_string(),
        };
        assert_eq!(knc.count, 1);

        let json_string = serde_json::to_string(&knc).unwrap();
        let deserialized: KnowledgeNodeChartCount = serde_json::from_str(&json_string).unwrap();
        assert_eq!(deserialized.count, 1);
        println!("Now {:?} will print!", knc);

        let _knc2 = KnowledgeNodeChartCount::default();
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_success_GraphItem() {
        let gi = GraphItem {
            count: "1".to_string(),
            indexed_at: "indexed_at".to_string(),
        };
        assert_eq!(gi.count, "1".to_string());

        let json_string = serde_json::to_string(&gi).unwrap();
        let deserialized: GraphItem = serde_json::from_str(&json_string).unwrap();
        assert_eq!(deserialized.count, "1".to_string());
        println!("Now {:?} will print!", gi);
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_success_NodesChartApiResponse() {
        let nca = NodesChartApiResponse {
            count: "1".to_string(),
            graph_items: vec![GraphItem {
                count: "1".to_string(),
                indexed_at: "indexed_at".to_string(),
            }],
            graph_interval: "graph_interval".to_string(),
        };
        assert_eq!(nca.count, "1".to_string());

        let json_string = serde_json::to_string(&nca).unwrap();
        let deserialized: NodesChartApiResponse = serde_json::from_str(&json_string).unwrap();
        assert_eq!(deserialized.count, "1".to_string());
        println!("Now {:?} will print!", nca);
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_success_From_KnowledgeNodeChartCount_for_GraphItem() {
        let knc = KnowledgeNodeChartCount {
            count: 1,
            indexed_at: "indexed_at".to_string(),
        };
        let gi: GraphItem = knc.into();
        assert_eq!(gi.count, "1".to_string());
        assert_eq!(gi.indexed_at, "indexed_at".to_string());
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_failure_From_KnowledgeNodeChartCount_for_GraphItem() {
        let knc = KnowledgeNodeChartCount {
            count: 1,
            indexed_at: "indexed_at".to_string(),
        };
        let gi: GraphItem = knc.into();
        assert_ne!(gi.count, "2".to_string());
        assert_ne!(gi.indexed_at, "indexed_at2".to_string());
    }
}
