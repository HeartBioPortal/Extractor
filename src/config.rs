use config::{Config as ConfigFile, ConfigError, Environment, File};
use std::path::Path;
use crate::types::Config;

pub fn load_config() -> Result<Config, ConfigError> {
    let config_dir = Path::new("config");
    
    let builder = ConfigFile::builder()
        // Start with default config
        .add_source(File::from(config_dir.join("default.toml")))
        // Add local config if it exists
        .add_source(
            File::from(config_dir.join("local.toml"))
                .required(false)
        )
        // Add environment variables with prefix "EXTRACTOR_"
        .add_source(Environment::with_prefix("EXTRACTOR"));

    // Build and convert to our config structure
    builder.build()?.try_deserialize()
}