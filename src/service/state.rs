/*
 * Created Date:  Mar 17, 2024
 * -----
 * Copyright (c) 2024 Tresle.ai or its affiliates. All Rights Reserved.
 */
//! state.rs
//! The `AppState` struct represents the state of the application.
//! It contains a MongoDB client and the name of the application's collection.
//!
//! `db`: A MongoDB client that implements the `DBTrait` trait, and is thread-safe (implements `Sync` and `Send`).
//! `app_collection`: The name of the application's collection in the MongoDB database.

use crate::configuration::settings::TresleFacadeServiceSettings;
use mongodb_utils::mongodb_client::DBTrait;
use std::fmt;

#[derive(Debug, thiserror::Error)]
pub enum AppStateError {
    #[error("App settings not provided")]
    AppSettingsNotProvided,
    #[error("DB not set")]
    DbNotSet,
}

pub struct AppState {
    pub db: Box<dyn DBTrait + Sync + Send>,
    pub app_settings: TresleFacadeServiceSettings,
}

impl fmt::Debug for AppState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AppState")
            .field("db", &"db")
            .field("app_settings", &self.app_settings)
            .finish()
    }
}

impl AppState {
    pub fn new(
        db: Box<dyn DBTrait + Sync + Send>,
        app_settings: TresleFacadeServiceSettings,
    ) -> Result<Self, AppStateError> {
        Ok(AppState { db, app_settings })
    }

    /// Returns a new `Builder` for `AppState`.
    pub fn builder() -> AppStateBuilder {
        AppStateBuilder {
            db: None,
            app_settings: None,
        }
    }
}

/// The `AppStateBuilder` struct provides a way to construct an `AppState`.
///
/// `db`: An optional MongoDB client that implements the `DBTrait` trait, and is thread-safe (implements `Sync` and `Send`).
/// `app_collection`: An optional string representing the name of the application's collection in the MongoDB database.
pub struct AppStateBuilder {
    db: Option<Box<dyn DBTrait + Sync + Send>>,
    app_settings: Option<TresleFacadeServiceSettings>,
}

impl AppStateBuilder {
    /// Sets the MongoDB client for the `Builder`.
    ///
    /// This method consumes the `Builder`, takes as input a MongoDB client that implements the `DBTrait` trait,
    /// and returns the `Builder`.
    pub fn mongodb_client(mut self, db_client: impl DBTrait + Sync + Send + 'static) -> Self {
        self.set_mongodb_client(db_client);
        self
    }

    /// Sets the MongoDB client for the `Builder`.
    ///
    /// This method takes as input a MongoDB client that implements the `DBTrait` trait,
    /// and returns a mutable reference to the `Builder`.
    pub fn set_mongodb_client(
        &mut self,
        db_client: impl DBTrait + Sync + Send + 'static,
    ) -> &mut Self {
        self.db = Some(Box::new(db_client));
        self
    }

    pub fn set_application_settings(mut self, app_settings: TresleFacadeServiceSettings) -> Self {
        self.app_settings = Some(app_settings);
        self
    }

    /// Builds the `AppState` from the `Builder`.
    ///
    /// This method consumes the `Builder` and returns an `AppState`.
    /// It will panic if the `db` or `app_collection` fields of the `Builder` are `None`.

    pub fn build(self) -> Result<AppState, AppStateError> {
        let app_state: AppState = AppState::new(
            self.db.ok_or(AppStateError::DbNotSet)?,
            self.app_settings
                .ok_or(AppStateError::AppSettingsNotProvided)?,
        )?;
        Ok(app_state)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::runtime::Runtime;

    #[test]
    fn test_success_app_state_error() {
        let app_state_error = AppStateError::AppSettingsNotProvided;
        assert_eq!(
            app_state_error.to_string(),
            "App settings not provided".to_string()
        );
        let app_state_error = AppStateError::DbNotSet;
        assert_eq!(app_state_error.to_string(), "DB not set".to_string());
        println!("Now {:?} will print!", app_state_error);
    }

    #[test]
    fn test_success_app_state() {
        let rt = Runtime::new().unwrap();

        rt.block_on(async {
            // Create a dev AppState and app_name
            let app_state = crate::tests::test_get_appstate().await.unwrap();
            println!("Now {:?} will print!", app_state);
        });
        assert!(true);
    }
}
