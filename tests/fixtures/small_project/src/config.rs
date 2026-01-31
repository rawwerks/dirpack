//! Configuration module.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub name: String,
    pub verbose: bool,
    pub max_items: usize,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            name: "default".to_string(),
            verbose: false,
            max_items: 100,
        }
    }
}

impl Config {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            ..Default::default()
        }
    }

    pub fn with_verbose(mut self, verbose: bool) -> Self {
        self.verbose = verbose;
        self
    }
}
