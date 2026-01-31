//! Small project library.

pub mod config;
pub mod utils;

pub use config::Config;

/// Initialize the library.
pub fn init() {
    println!("Initialized");
}

/// Process input data.
pub fn process(data: &str) -> String {
    data.to_uppercase()
}

/// Validate configuration.
pub fn validate(config: &Config) -> bool {
    config.name.len() > 0
}
