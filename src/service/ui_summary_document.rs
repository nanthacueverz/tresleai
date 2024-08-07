/*
*  Created Date:  Mar 17, 2024
 * -----
 * Copyright (c) 2024 Tresle.ai or its affiliates. All Rights Reserved.
 */
//! This module contains the schema for the UI Summary document.

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UiSummaryDocument {
    pub app_name: String,
    pub call_type: String,
    pub count: u64,
    pub timestamp: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_success_ui_summary_document() {
        let ui_summary_document = UiSummaryDocument {
            app_name: "app_name".to_string(),
            call_type: "call_type".to_string(),
            count: 1,
            timestamp: "timestamp".to_string(),
        };
        assert_eq!(ui_summary_document.app_name, "app_name".to_string());
        assert_eq!(ui_summary_document.call_type, "call_type".to_string());
        assert_eq!(ui_summary_document.count, 1);
        assert_eq!(ui_summary_document.timestamp, "timestamp".to_string());

        let json_string = serde_json::to_string(&ui_summary_document).unwrap();
        let deserialized_ui_summary_document: UiSummaryDocument =
            serde_json::from_str(&json_string).unwrap();
        assert_eq!(
            deserialized_ui_summary_document.app_name,
            "app_name".to_string()
        );
        let ui = deserialized_ui_summary_document.clone();
        println!("Now {:?} will print!", ui);
    }
}
