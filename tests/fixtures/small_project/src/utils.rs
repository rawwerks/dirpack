//! Utility functions.

/// Format a message with timestamp.
pub fn format_message(msg: &str) -> String {
    format!("[{}] {}", timestamp(), msg)
}

/// Get current timestamp string.
fn timestamp() -> String {
    "2024-01-01".to_string()
}
