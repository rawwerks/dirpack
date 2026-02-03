use std::path::PathBuf;

use dirpack::eval::evaluate;
use dirpack::packer;

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

#[test]
fn eval_utilization_on_content_fixture() {
    let repo = PathBuf::from("tests/fixtures/content_budget");
    let report = evaluate(&repo, &[1000]);
    let metrics = &report.budgets[0];

    assert!(
        metrics.utilization_ratio >= 0.8,
        "utilization ratio {}",
        metrics.utilization_ratio
    );
}

#[test]
fn visual_inspection_500t() {
    let repo = PathBuf::from("tests/fixtures/small_project");
    let output = packer::pack_default(&repo, 500).output;
    println!("\n━━━ VISUAL INSPECTION (500t) ━━━");
    println!("{}", output);
    println!("━━━ END ━━━\n");
}

#[test]
fn visual_inspection_2000t() {
    let repo = PathBuf::from("tests/fixtures/small_project");
    let output = packer::pack_default(&repo, 2000).output;
    println!("\n━━━ VISUAL INSPECTION (2000t) ━━━");
    println!("{}", output);
    println!("━━━ END ━━━\n");
}

#[test]
fn eval_with_visual_inspection() {
    let status = std::process::Command::new("bash")
        .arg("./scripts/eval.sh")
        .arg("tests/fixtures/small_project")
        .arg("/dev/null")
        .env("DIRPACK", "./target/debug/dirpack")
        .status()
        .expect("eval.sh failed to run");

    assert!(status.success(), "eval.sh failed");
}
