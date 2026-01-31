//! Token counting utilities.
//!
//! Provides approximate token counting for budget management.
//! Uses a simple heuristic that works well for code and prose.

/// Count tokens in a string using a simple heuristic.
/// For code: ~4 characters per token
/// For prose: ~1.3 words per token
pub fn count_tokens(content: &str) -> usize {
    // Simple heuristic: average of char-based and word-based estimates
    let char_estimate = content.len() / 4;
    let word_count = content.split_whitespace().count();
    let word_estimate = (word_count as f64 * 1.3) as usize;

    // Use the higher estimate for safety
    char_estimate.max(word_estimate).max(1)
}

/// Estimate tokens for a given byte count.
pub fn estimate_tokens_from_bytes(bytes: usize) -> usize {
    bytes / 4
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_count_tokens() {
        // Short string
        assert!(count_tokens("hello") >= 1);

        // Code-like content
        let code = "fn main() { println!(\"Hello\"); }";
        let tokens = count_tokens(code);
        assert!(tokens > 0 && tokens < 20);

        // Prose content
        let prose = "This is a sentence with several words in it.";
        let tokens = count_tokens(prose);
        assert!(tokens > 0 && tokens < 20);
    }

    #[test]
    fn test_empty_string() {
        assert_eq!(count_tokens(""), 1); // Minimum of 1
    }
}
