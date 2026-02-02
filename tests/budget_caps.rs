use dirpack::budget::{Budget, BudgetTarget};
use dirpack::limits;

#[test]
fn test_budget_tokens_clamped_to_max() {
    let budget = Budget::new(BudgetTarget::Tokens(limits::MAX_BUDGET_TOKENS + 1));
    assert_eq!(budget.limit(), limits::MAX_BUDGET_TOKENS);
}

#[test]
fn test_budget_bytes_clamped_to_max() {
    let budget = Budget::new(BudgetTarget::Bytes(limits::MAX_BUDGET_BYTES + 1));
    assert_eq!(budget.limit(), limits::MAX_BUDGET_BYTES);
}

#[test]
fn test_scan_depth_clamp_behaviour() {
    assert_eq!(limits::clamp_scan_depth(0), limits::MAX_SCAN_DEPTH);
    assert_eq!(
        limits::clamp_scan_depth(limits::MAX_SCAN_DEPTH + 10),
        limits::MAX_SCAN_DEPTH
    );
    assert_eq!(limits::clamp_scan_depth(3), 3);
}
