//! Integration tests.

#[test]
fn test_init() {
    small_project::init();
}

#[test]
fn test_process() {
    let result = small_project::process("hello");
    assert_eq!(result, "HELLO");
}
