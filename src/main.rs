/*
 * Created Date:  Mar 17, 2024
 * -----
 * Copyright (c) 2024 Tresle.ai or its affiliates. All Rights Reserved.
 */
//! main.rs
//!
//! This is the main entry point for the Tresleai Facade Service APIs.
//! The microservice is built using the Axum web framework.
//! The microservice provides APIs for onboarding applications, retrieving data, and other administrative tasks.
//! The microservice uses DocumentDB as the database.
//! The microservice uses the Tresleai Logging Layer for logging.
//! The microservice uses the Tresleai OpenAPI for API documentation.
//!
//! The `main` function does the following:
#![allow(rustdoc::invalid_rust_codeblocks)]
//#![doc = include_str!("../README.md")]
pub mod admin_ui_api;
mod configuration;
mod onboarding;
mod retrieval;
mod service;

use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use crate::admin_ui_api::app_delete_handler::*;
use crate::admin_ui_api::app_get_handler::*;
use crate::admin_ui_api::app_get_logs_handler::*;
use crate::admin_ui_api::app_knowledge_nodes_and_errors_count::*;
use crate::admin_ui_api::app_knowledge_nodes_chart_handler::*;
use crate::admin_ui_api::app_knowledge_nodes_errors_handler::*;
use crate::admin_ui_api::app_knowledge_nodes_handler::*;
use crate::admin_ui_api::app_list_handler::*;
use crate::admin_ui_api::app_search_enabled_handler::*;
use crate::admin_ui_api::apps_and_calls_overview_handler::*;
use crate::admin_ui_api::capture_tc_handler::*;
use crate::admin_ui_api::kub_generate_token_handler::*;
use crate::admin_ui_api::metric_calls_handler::*;
use crate::admin_ui_api::metric_error_handler::*;
use crate::onboarding::handler::*;
use crate::retrieval::handler::*;
use crate::retrieval::history_handler::*;

use crate::service::state::AppState;
use axum::http::{HeaderName, HeaderValue, Method};
use axum::Router;
use dotenv::dotenv;
use logging_utils::layer::TresleaiLoggingLayer;
use logging_utils::worker::TresleaiBackgroundWorker;
use mongodb_utils::mongodb_client::DBTrait;
use mongodb_utils::mongodb_client::DB;
use service::route::create_router;
use std::sync::Arc;
use tower_http::cors::CorsLayer;
use tracing::{debug, instrument};
use tracing_subscriber::Layer;
use tracing_subscriber::{fmt, layer::*, EnvFilter};

//OpenApi generation
#[derive(OpenApi)]
#[openapi(
    paths(
        post_app_onboarding_handler,
        post_retrieval_handler,
        get_history_handler,
        delete_app,
        get_app,
        get_kubernetes_token,
        get_app_list,
        get_metric_calls,
        get_metric_errors,
        get_logs,
        get_apps_and_calls_overview_handler,
        update_search_enabled_handler,
        get_knowledge_nodes_handler,
        get_knowledge_nodes_chart_handler,
        get_knowledge_nodes_errors_handler,
        get_knowledge_nodes_and_errors_count,
        post_capture_tc_handler
    ),
    components(schemas(
        crate::onboarding::schema::app_onboarding_request::OnboardingRequest,
        crate::onboarding::schema::app_onboarding_request::AppDataSource,
        crate::onboarding::schema::app_onboarding_request::LlmModel,
        crate::onboarding::schema::app_onboarding_request::FileStore,
        crate::onboarding::schema::app_onboarding_request::DataStore,
        crate::onboarding::schema::app_onboarding_request::Hint,
        crate::onboarding::schema::app_onboarding_request::Table,
        crate::onboarding::schema::app_onboarding_request::SampleRows,
        crate::onboarding::schema::app_onboarding_request::Column,
        crate::onboarding::schema::response::AppCreateResponse,
        crate::onboarding::schema::response::ErrorResponse,
        crate::retrieval::schema::history_document::HistoryDocument,
        crate::admin_ui_api::schema::CaptureUserSchema,
        api_utils::retrieval_model::RetrievalRequest,
        api_utils::retrieval_model::UserDetails,
        api_utils::retrieval_model::AccessDetails,
        api_utils::retrieval_model::IAMPolicyDetails,
        api_utils::retrieval_model::DbPolicyDetails,
    )),
    info(
        title = "Tresleai Rest API ",
        version = "1.0.0",
        description = "Tresleai Facade Microservice API"
    )
)]
struct ApiDoc;

#[instrument]
#[tokio::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();
    // env_logger::init();
    debug!("Starting Facade Microservice.");

    // Initialize environment and get settings
    let settings = match configuration::environment::init_environment_and_get_settings() {
        Ok(settings) => settings,
        Err(e) => {
            eprintln!("Failed to initialize environment and get settings: {}", e);
            std::process::exit(1);
        }
    };

    // Initialize a connection to the database
    let mongodb = match DB::init(
        settings.mongo_db.mongo_db_url.clone(),
        settings.mongo_db.mongo_db_database_name.clone(),
    )
    .await
    {
        Ok(mongodb) => mongodb,
        Err(e) => {
            eprintln!("Failed to initialize database connection: {}", e);
            std::process::exit(1);
        }
    };

    // Set up AppState struct instance
    let app_state = match AppState::builder()
        .mongodb_client(mongodb)
        .set_application_settings(settings)
        .build()
    {
        Ok(app_state) => app_state,
        Err(e) => {
            eprintln!("Failed to build AppState: {}", e);
            std::process::exit(1);
        }
    };

    let app_state_arc = Arc::new(app_state);

    // Initialize tracing subscriber
    let tresleai_background_worker = match tracing_initialization(app_state_arc.clone()).await {
        Ok(worker) => worker,
        Err(e) => {
            eprintln!("Failed to initialize tracing: {}", e);
            std::process::exit(1);
        }
    };

    // Set up CORS (Cross-Origin Resource Sharing) settings
    let origins: Vec<HeaderValue> = app_state_arc
        .app_settings
        .cors_allowed_origins
        .iter()
        .filter_map(|origin| origin.parse().ok())
        .collect();

    let methods: Vec<Method> = app_state_arc
        .app_settings
        .application
        .cors
        .allowed_methods
        .iter()
        .filter_map(|method| method.parse().ok())
        .collect();

    let headers: Vec<HeaderName> = app_state_arc
        .app_settings
        .application
        .cors
        .allowed_headers
        .iter()
        .filter_map(|header| header.parse().ok())
        .collect();

    let credentials = app_state_arc
        .app_settings
        .application
        .cors
        .allow_credentials;

    let cors = CorsLayer::new()
        .allow_origin(origins)
        .allow_methods(methods)
        .allow_credentials(credentials)
        .allow_headers(headers);

    // Create a router with the AppState instance and apply the CORS settings to it
    let app = Router::new()
        .merge(create_router(app_state_arc.clone())) // Application routes
        .merge(SwaggerUi::new("/swagger-ui").url("/api-doc/openapi.json", ApiDoc::openapi())) // Swagger UI
        .layer(cors);

    debug!("🚀 Server started successfully.");

    // Start a server that listens on 0.0.0.0:8000 and serves the application
    let listener =
        match tokio::net::TcpListener::bind(app_state_arc.app_settings.application.address()).await
        {
            Ok(listener) => listener,
            Err(e) => {
                debug!("Failed to bind to address: {}", e);
                std::process::exit(1);
            }
        };

    match axum::serve(listener, app).await {
        Ok(_) => {
            debug!("Server started successfully.");
        }
        Err(e) => {
            debug!("Failed to start server: {}", e);
            std::process::exit(1);
        }
    }
    if let Some(worker) = tresleai_background_worker {
        worker.shutdown().await;
    }
    Ok(())
}

pub async fn tracing_initialization(
    app_state_arc: Arc<AppState>,
) -> Result<Option<TresleaiBackgroundWorker>, Box<dyn std::error::Error>> {
    let fmt_layer = fmt::Layer::default();

    let crate_name = env!("CARGO_PKG_NAME").replace('-', "_");
    let fmt_filter = EnvFilter::builder()
        .with_default_directive(
            app_state_arc
                .app_settings
                .tracing_layer_levels
                .fmt_layer_level
                .parse()?,
        )
        .from_env()?
        .add_directive(
            format!(
                "{}={}",
                crate_name,
                app_state_arc
                    .app_settings
                    .tracing_layer_levels
                    .fmt_layer_service_exception_level
            )
            .parse()?,
        );

    if app_state_arc.app_settings.tracing_layer_debug_mode {
        let subscriber = tracing_subscriber::registry().with(fmt_layer.with_filter(fmt_filter));

        // Set the global tracing subscriber
        tracing::subscriber::set_global_default(subscriber)?;

        Ok(None)
    } else {
        // init trealeai subscriber layer and support background worker
        let logging_api_url = app_state_arc
            .app_settings
            .tresleai_urls
            .logging_service_url
            .clone();
        let audit_api_url = app_state_arc
            .app_settings
            .tresleai_urls
            .audit_service_url
            .clone();
        let metrics_api_url = app_state_arc
            .app_settings
            .tresleai_urls
            .metric_service_url
            .clone();
        let system_app_name = app_state_arc
            .app_settings
            .tracing_layer_system_app_name
            .clone();

        let (tresleai_layer, tresleai_background_worker) = TresleaiLoggingLayer::builder()
            .with_logging_api_url(logging_api_url)
            .with_audit_api_url(audit_api_url)
            .with_metrics_api_url(metrics_api_url)
            .with_system_app_name(system_app_name)
            .build();
        let subscriber = tracing_subscriber::registry()
            .with(fmt_layer.with_filter(fmt_filter))
            .with(
                tresleai_layer.with_filter(EnvFilter::try_new(
                    app_state_arc
                        .app_settings
                        .tracing_layer_levels
                        .peripheral_services_layer_level
                        .clone(),
                )?),
            );

        // Set the global tracing subscriber
        tracing::subscriber::set_global_default(subscriber)?;

        Ok(Some(tresleai_background_worker))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::configuration;
    use dotenv::dotenv;
    // use log::error as log_error;
    use mockito::{Server, ServerOpts};
    use mongodb_utils::mongodb_client::DBTrait;
    use mongodb_utils::mongodb_client::DB;
    use once_cell::sync::Lazy;
    use std::error::Error;
    use std::sync::Mutex;
    use tracing::error as log_error;

    pub static TEST_ENV_MUTEX: Mutex<i32> = Mutex::new(1);
    pub static TEST_DB_MUTEX: Mutex<i32> = Mutex::new(1);

    pub static MOCK_SERVER: Lazy<Mutex<Server>> = Lazy::new(|| {
        Mutex::new(Server::new_with_opts(ServerOpts {
            port: 8003,
            ..Default::default()
        }))
    });

    /// test function to get an appstate instance just like dev environment debugging
    pub async fn test_get_appstate() -> Result<Arc<AppState>, Box<dyn Error>> {
        let _guard = TEST_ENV_MUTEX.lock().unwrap();
        dotenv().ok();

        // Initialize environment and get settings
        let settings = match configuration::environment::init_environment_and_get_settings() {
            Ok(settings) => settings,
            Err(e) => {
                log_error!("Failed to initialize environment and get settings: {}", e);
                std::process::exit(1);
            }
        };

        // Initialize a connection to the database
        let mongodb = DB::init(
            settings.mongo_db.mongo_db_url.clone(),
            settings.mongo_db.mongo_db_database_name.clone(),
        )
        .await?;

        // Set up AppState struct instance
        let app_state = AppState::builder()
            .mongodb_client(mongodb)
            .set_application_settings(settings)
            .build()
            .unwrap();

        return Ok(Arc::new(app_state));
    }

    #[test]
    fn test_pub_test_get_appstate() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            println!("{}", ApiDoc::openapi().to_pretty_json().unwrap());
            let app_state = test_get_appstate().await.unwrap();
            assert_eq!(
                app_state.app_settings.application.name,
                "tresle-facade-service"
            );
        });
    }

    #[test]
    #[ignore = "Full service test - don't run this test every time"] // cargo test test_service -- --ignored
    fn test_service() {
        // Call the function you're testing
        crate::onboarding::handler::tests::test_success_post_app_onboarding_handler();
        crate::retrieval::handler::tests::test_success_post_retrieval_handler();

        // Check that the function's output is as expected
        assert!(true);
    }
}
