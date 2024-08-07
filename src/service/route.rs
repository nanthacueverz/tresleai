/*
 * Created Date:  Mar 17, 2024
 * -----
 * Copyright (c) 2024 Tresle.ai or its affiliates. All Rights Reserved.
 */
//! This module contains the routes/endpoints for the different handlers/APIs.

use crate::service::error::TresleFacadeCommonError;
use axum::http::StatusCode;
use error_utils::ApiErrorResponse;
use error_utils::AxumApiError;
use std::sync::Arc;
use uuid::Uuid;

use crate::AppState;
use axum::{
    http::Uri,
    routing::{delete, get, patch, post, Router},
};
use tracing::debug;

use crate::admin_ui_api::app_delete_handler::delete_app;
use crate::admin_ui_api::app_get_handler::get_app;
use crate::admin_ui_api::app_get_logs_handler::get_logs;
use crate::admin_ui_api::app_knowledge_nodes_and_errors_count::get_knowledge_nodes_and_errors_count;
use crate::admin_ui_api::app_knowledge_nodes_chart_handler::get_knowledge_nodes_chart_handler;
use crate::admin_ui_api::app_knowledge_nodes_errors_handler::get_knowledge_nodes_errors_handler;
use crate::admin_ui_api::app_knowledge_nodes_handler::get_knowledge_nodes_handler;
use crate::admin_ui_api::app_list_handler::get_app_list;
use crate::admin_ui_api::app_search_enabled_handler::update_search_enabled_handler;
use crate::admin_ui_api::apps_and_calls_overview_handler::get_apps_and_calls_overview_handler;
use crate::admin_ui_api::capture_tc_handler::post_capture_tc_handler;
use crate::admin_ui_api::kub_generate_token_handler::get_kubernetes_token;
use crate::admin_ui_api::metric_calls_handler::get_metric_calls;
use crate::admin_ui_api::metric_error_handler::get_metric_errors;
use crate::onboarding::handler::post_app_onboarding_handler;
use crate::retrieval::handler::post_retrieval_handler;
use crate::retrieval::history_handler::get_history_handler;

pub fn create_router(app_state: Arc<AppState>) -> Router {
    Router::new()
        .route("/api/v1.0/retrieval", post(post_retrieval_handler))
        .route("/api/v1.0/history/retrieval", get(get_history_handler))
        .route("/api/v1.1/admin/token", get(get_kubernetes_token))
        .route("/api/v1.1/admin/apps", get(get_app_list))
        .route("/api/v1.1/admin/apps/:app_name", get(get_app))
        .route("/api/v1.1/admin/apps/:app_name", delete(delete_app))
        .route(
            "/api/v1.1/admin/search/apps/:app_name",
            patch(update_search_enabled_handler),
        )
        .route(
            "/api/v1.1/admin/apps/onboard",
            post(post_app_onboarding_handler),
        )
        .route("/api/v1.1/admin/capture_tc", post(post_capture_tc_handler))
        .route(
            "/api/v1.1/admin/overview",
            get(get_apps_and_calls_overview_handler),
        )
        .route(
            "/api/v1.1/admin/nodes/:app_name",
            get(get_knowledge_nodes_handler),
        )
        .route(
            "/api/v1.1/admin/nodes/errors/:app_name",
            get(get_knowledge_nodes_errors_handler),
        )
        .route(
            "/api/v1.1/admin/nodes/count/:app_name",
            get(get_knowledge_nodes_and_errors_count),
        )
        .route(
            "/api/v1.1/admin/nodes/chart/:app_name",
            get(get_knowledge_nodes_chart_handler),
        )
        .route("/api/v1.1/admin/logs", get(get_logs))
        .route("/api/v1.1/admin/metric/calls", get(get_metric_calls))
        .route("/api/v1.1/admin/metric/logs", get(get_metric_errors))
        .with_state(app_state)
        .fallback(fallback)
}

pub async fn fallback(uri: Uri) -> AxumApiError<TresleFacadeCommonError> {
    debug!("->> {:<12} - fallback - ", "HANDLER");
    let reference_id = Uuid::new_v4().to_string();
    let task_id = Uuid::new_v4().to_string();
    TresleFacadeCommonError::RouteNotFound {
        task_id,
        time_stamp: ApiErrorResponse::time_stamp(),
        error_code: StatusCode::BAD_REQUEST,
        source: Box::new(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("Route not found: {}", uri),
        )),
        reference_id,
        ext_message: "Internal Error. Please contact tresleai support team.".to_string(),
    }
    .into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::runtime::Runtime;

    #[test]
    fn test_success_create_router() {
        let rt = Runtime::new().unwrap();

        rt.block_on(async {
            let app_state = crate::tests::test_get_appstate().await.unwrap();
            let _router = create_router(app_state);
        });
    }
}
