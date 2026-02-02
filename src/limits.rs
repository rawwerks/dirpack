//! Safety limits for server usage.

pub const MAX_BUDGET_TOKENS: usize = 8_000;
pub const MAX_BUDGET_BYTES: usize = 32_000;
pub const MAX_SCAN_DEPTH: usize = 20;

pub fn clamp_budget_tokens(tokens: usize) -> usize {
    tokens.min(MAX_BUDGET_TOKENS)
}

pub fn clamp_budget_bytes(bytes: usize) -> usize {
    bytes.min(MAX_BUDGET_BYTES)
}

pub fn clamp_scan_depth(depth: usize) -> usize {
    if depth == 0 {
        MAX_SCAN_DEPTH
    } else {
        depth.min(MAX_SCAN_DEPTH)
    }
}
