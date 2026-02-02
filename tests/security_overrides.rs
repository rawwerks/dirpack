use dirpack::budget::BudgetTarget;
use dirpack::config::{
    apply_security_overrides, clamp_budget_target, Config, SAFE_MAX_BUDGET_BYTES,
    SAFE_MAX_BUDGET_TOKENS, SAFE_MAX_SCAN_DEPTH,
};

#[test]
fn test_clamp_budget_target_tokens() {
    let target = BudgetTarget::Tokens(SAFE_MAX_BUDGET_TOKENS + 500);
    match clamp_budget_target(target) {
        BudgetTarget::Tokens(tokens) => assert_eq!(tokens, SAFE_MAX_BUDGET_TOKENS),
        BudgetTarget::Bytes(_) => panic!("expected tokens budget"),
    }
}

#[test]
fn test_clamp_budget_target_bytes() {
    let target = BudgetTarget::Bytes(SAFE_MAX_BUDGET_BYTES + 2048);
    match clamp_budget_target(target) {
        BudgetTarget::Bytes(bytes) => assert_eq!(bytes, SAFE_MAX_BUDGET_BYTES),
        BudgetTarget::Tokens(_) => panic!("expected bytes budget"),
    }
}

#[test]
fn test_apply_security_overrides() {
    let mut config = Config::default();
    config.scanning.follow_symlinks = true;
    config.scanning.include_hidden = true;
    config.scanning.max_depth = 0;

    apply_security_overrides(&mut config);

    assert!(!config.scanning.follow_symlinks);
    assert!(!config.scanning.include_hidden);
    assert_eq!(config.scanning.max_depth, SAFE_MAX_SCAN_DEPTH);
}
