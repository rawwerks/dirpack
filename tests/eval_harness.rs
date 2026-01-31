use std::path::PathBuf;

use dirpack::eval::evaluate;

#[test]
fn eval_metrics_on_fixture() {
    let repo = PathBuf::from("tests/fixtures/small_project");
    let report = evaluate(&repo, &[500, 1000]);

    assert_eq!(report.budgets.len(), 2);

    for metrics in report.budgets {
        // Overshoot should be within 2%
        assert!(metrics.overshoot_ratio <= 0.02, "overshoot {}", metrics.overshoot_ratio);

        // Entry points should be fully covered at budgets >= 500
        assert!(
            (metrics.entry_point_coverage - 1.0).abs() < f64::EPSILON,
            "entry point coverage {}",
            metrics.entry_point_coverage
        );

        // Tree ratio should be <= 40% of budget
        assert!(
            metrics.tree_ratio <= 0.4 + 1e-6,
            "tree ratio {}",
            metrics.tree_ratio
        );
    }
}
