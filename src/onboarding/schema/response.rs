/*
 * Created Date:  Mar 17, 2024
 * -----
 * Copyright (c) 2024 Tresle.ai or its affiliates. All Rights Reserved.
 */
use serde::Serialize;
use utoipa::ToSchema;

#[allow(non_snake_case)]
#[derive(Serialize, Debug, ToSchema)]
pub struct AppCreateResponse {
    pub status: String,
    pub message: String,
    pub api_key: String,
    pub app_id: String,
    pub reference_id: String,
}

#[derive(Serialize, Debug, ToSchema)]
pub struct ErrorResponse {
    pub status: String,
    pub message: String,
    pub errors: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[allow(non_snake_case)]
    fn test_success_AppCreateResponse() {
        let app_create_response = AppCreateResponse {
            status: "status".to_string(),
            message: "message".to_string(),
            api_key: "api_key".to_string(),
            app_id: "app_id".to_string(),
            reference_id: "reference_id".to_string(),
        };
        assert_eq!(app_create_response.status, "status".to_string());
        assert_eq!(app_create_response.message, "message".to_string());

        let _json_string = serde_json::to_string(&app_create_response).unwrap();
        println!("Now {:?} will print!", app_create_response);
        let _schema = AppCreateResponse::schema();
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_success_ErrorResponse() {
        let error_response = ErrorResponse {
            status: "status".to_string(),
            message: "message".to_string(),
            errors: vec!["error1".to_string(), "error2".to_string()],
        };
        assert_eq!(error_response.status, "status".to_string());
        assert_eq!(error_response.message, "message".to_string());

        let _json_string = serde_json::to_string(&error_response).unwrap();
        println!("Now {:?} will print!", error_response);
        let _schema = ErrorResponse::schema();
    }
}
