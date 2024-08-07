/*
*  Created Date:  Mar 17, 2024
 * -----
 * Copyright (c) 2024 Tresle.ai or its affiliates. All Rights Reserved.
 */
//! This module contains the schema for the history document.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Serialize, Deserialize, Debug, Clone, ToSchema)]
pub struct HistoryDocument {
    pub reference_id: String,
    pub task_id: String,
    pub query: String,
    pub response: String,
    pub timestamp: String,
    disclaimer_text: String,
}

impl HistoryDocument {
    pub fn new(
        reference_id: String,
        task_id: String,
        query: String,
        response: String,
        timestamp: String,
        disclaimer_text: String,
    ) -> Self {
        Self {
            reference_id,
            task_id,
            query,
            response,
            timestamp,
            disclaimer_text,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;
    use std::str::FromStr;

    #[test]
    fn test_history_document_traits() {
        let doc = HistoryDocument {
            reference_id: "123".to_string(),
            task_id: "456".to_string(),
            query: "query".to_string(),
            response: "response".to_string(),
            timestamp: "timestamp".to_string(),
            disclaimer_text: "disclaimer_text".to_string(),
        };

        // Test Clone
        let cloned_doc = doc.clone();
        assert_eq!(doc.timestamp, cloned_doc.timestamp);

        // Test Debug
        println!("{:?}", doc); // This should not panic

        // Test Serialize
        let serialized_doc = serde_json::to_string(&doc).unwrap();
        assert!(Value::from_str(&serialized_doc).is_ok());

        // Test Deserialize
        let deserialized_doc: HistoryDocument = serde_json::from_str(&serialized_doc).unwrap();
        assert_eq!(doc.timestamp, deserialized_doc.timestamp);
    }
}
