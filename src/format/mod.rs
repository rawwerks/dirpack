pub mod full;
pub mod json;
pub mod pipe;

pub use pipe::PipeFormatter;

use serde::Serialize;

#[derive(Debug, Clone, Default, Serialize)]
pub struct PackOutput {
    pub title: String,
    pub root: String,
    pub notes: Option<String>,
    pub tree: Option<String>,
    pub directories: Vec<DirectoryListing>,
    pub files: Vec<FileOutput>,
    pub budget: Option<BudgetSummary>,
}

impl PackOutput {
    pub fn new(title: impl Into<String>, root: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            root: root.into(),
            ..Self::default()
        }
    }
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct DirectoryListing {
    pub name: String,
    pub files: Vec<String>,
}

impl DirectoryListing {
    pub fn new(name: impl Into<String>, files: Vec<String>) -> Self {
        Self {
            name: name.into(),
            files,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct FileOutput {
    pub path: String,
    pub category: Option<String>,
    pub priority: Option<i32>,
    pub signatures: Vec<String>,
    pub content: Option<String>,
}

impl FileOutput {
    pub fn new(path: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            ..Self::default()
        }
    }
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct BudgetSummary {
    pub target_tokens: Option<usize>,
    pub used_tokens: Option<usize>,
    pub target_bytes: Option<usize>,
    pub used_bytes: Option<usize>,
}

pub trait Formatter {
    fn format(&self) -> String;
}
