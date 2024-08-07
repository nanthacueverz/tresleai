/*
 * Created Date:  Mar 17, 2024
 * -----
 * Copyright (c) 2024 Tresle.ai or its affiliates. All Rights Reserved.
 */
//! This module contains the environment
//!
//!
use crate::configuration::settings;
use crate::configuration::settings::SettingsError;
// use log::error;
use tracing::error;

pub type EnvParsingResult<T> = core::result::Result<T, SettingsError>;

fn get_env(name: &'static str) -> EnvParsingResult<String> {
    std::env::var(name).map_err(|_| {
        SettingsError::Config(config::ConfigError::NotFound(format!(
            "Failed to parse environment variable {}",
            name
        )))
    })
}

/// Preset any environment variables.
pub fn init_environment_and_get_settings(
) -> Result<settings::TresleFacadeServiceSettings, SettingsError> {
    let local_yaml = get_env("LOCAL_YAML")?;
    let global_yaml = get_env("GLOBAL_YAML")?;
    let config_dir = get_env("CONFIG_DIR")?;

    let base_dir = match std::env::current_dir() {
        Ok(dir) => dir,
        Err(e) => {
            error!("Failed to determine cwd: {}", e);
            return Err(SettingsError::Config(config::ConfigError::NotFound(
                "Failed to determine current working directory".to_string(),
            )));
        }
    };
    let config_dir = base_dir.join(config_dir);

    let settings_loader = config::Config::builder()
        .add_source(config::File::from(config_dir.join(global_yaml)))
        .add_source(config::File::from(config_dir.join(local_yaml)))
        .build()
        .map_err(SettingsError::Config)?;

    settings_loader
        .try_deserialize::<settings::TresleFacadeServiceSettings>()
        .map_err(SettingsError::Config)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use tempfile::tempdir;

    #[test]
    fn test_success_init_environment_and_get_settings() {
        dotenv::dotenv().ok();
        let _guard = crate::tests::TEST_ENV_MUTEX.lock().unwrap();
        // Setup mock environment variables
        // env::set_var("CONFIG_DIR", "configurations");
        // env::set_var("GLOBAL_YAML", ".global.yaml");
        // env::set_var("LOCAL_YAML", "local.yaml");

        let result = init_environment_and_get_settings();
        println!("{:?}", result);
        assert!(result.is_ok());

        // Clean up
        env::remove_var("CONFIG_DIR");
        env::remove_var("GLOBAL_YAML");
        env::remove_var("LOCAL_YAML");
    }

    #[test]
    fn test_fail_missing_environment_variables() {
        let _guard = crate::tests::TEST_ENV_MUTEX.lock().unwrap();
        // Ensure environment variables are not set
        env::remove_var("TRESLE_APP_ENVIRONMENT");
        env::remove_var("BASE_YAML");
        env::remove_var("CONFIG_DIR");

        let result = init_environment_and_get_settings();
        assert!(result.is_err());
    }

    #[test]
    fn test_fail_incorrect_environment_value() {
        let _guard = crate::tests::TEST_ENV_MUTEX.lock().unwrap();
        let temp_dir = tempdir().unwrap();
        env::set_var("TRESLE_APP_ENVIRONMENT", "invalid_environment");
        env::set_var("BASE_YAML", "base.yaml");
        env::set_var("CONFIG_DIR", temp_dir.path().to_str().unwrap());

        let result = init_environment_and_get_settings();
        assert!(result.is_err());

        // Cleanup
        env::remove_var("TRESLE_APP_ENVIRONMENT");
        env::remove_var("BASE_YAML");
        env::remove_var("CONFIG_DIR");
    }

    #[test]
    fn test_fail_missing_base_yaml() {
        let _guard = crate::tests::TEST_ENV_MUTEX.lock().unwrap();
        let temp_dir = tempdir().unwrap();
        env::set_var("TRESLE_APP_ENVIRONMENT", "test");
        env::set_var("BASE_YAML", "path/to/nonexistent/file");
        env::set_var("CONFIG_DIR", temp_dir.path().to_str().unwrap());

        let result = init_environment_and_get_settings();
        assert!(result.is_err());

        // Cleanup
        env::remove_var("TRESLE_APP_ENVIRONMENT");
        env::remove_var("BASE_YAML");
        env::remove_var("CONFIG_DIR");
    }
}
