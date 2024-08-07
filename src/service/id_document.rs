/*
*  Created Date:  Mar 17, 2024
 * -----
 * Copyright (c) 2024 Tresle.ai or its affiliates. All Rights Reserved.
 */
//! This module contains the schema for the ID document.

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct IdDocument {
    pub app_name: String,
    pub reference_id: String,
    pub task_id: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[allow(non_snake_case)]
    #[test]
    fn test_success_IdDocument() {
        let id_document = IdDocument {
            app_name: "app_name".to_string(),
            reference_id: "reference_id".to_string(),
            task_id: "task_id".to_string(),
        };
        assert_eq!(id_document.app_name, "app_name".to_string());
        assert_eq!(id_document.reference_id, "reference_id".to_string());
        assert_eq!(id_document.task_id, "task_id".to_string());

        let json_string = serde_json::to_string(&id_document).unwrap();
        let deserialized_id_document: IdDocument = serde_json::from_str(&json_string).unwrap();
        assert_eq!(deserialized_id_document.app_name, "app_name".to_string());
        let id = deserialized_id_document.clone();
        println!("Now {:?} will print!", id);
    }
}
