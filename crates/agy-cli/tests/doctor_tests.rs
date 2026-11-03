use agy_cli::doctor::DoctorEngine;

#[tokio::test]
async fn test_doctor_diagnostics_report_structure() {
    let engine = DoctorEngine {
        skills_library_path: Some("data/skills-library".to_string()),
        presets_yaml_path: Some("data/skills-library/presets.yaml".to_string()),
        caddyfile_path: Some("deploy/caddy/Caddyfile".to_string()),
        ..Default::default()
    };

    let report = engine.run_diagnostics().await;
    assert_eq!(report.total_checks, 9);
    assert_eq!(report.checks.len(), 9);

    let categories: Vec<String> = report.checks.iter().map(|c| c.category.clone()).collect();
    assert!(categories.iter().any(|c| c.contains("1. Kernel")));
    assert!(categories.iter().any(|c| c.contains("2. POSIX Users")));
    assert!(categories.iter().any(|c| c.contains("3. Systemd")));
    assert!(categories.iter().any(|c| c.contains("4. Caddy Ingress")));
    assert!(categories.iter().any(|c| c.contains("5. Linux PAM")));
    assert!(categories.iter().any(|c| c.contains("6. Central Hub")));
    assert!(categories.iter().any(|c| c.contains("7. Quota Failover")));
    assert!(categories.iter().any(|c| c.contains("8. Skills Library")));
    assert!(categories.iter().any(|c| c.contains("9. Network Sockets")));
}

#[tokio::test]
async fn test_doctor_json_serialization() {
    let engine = DoctorEngine::default();
    let report = engine.run_diagnostics().await;
    let json_str = serde_json::to_string(&report).expect("DoctorReport must serialize to JSON");
    assert!(json_str.contains("total_checks"));
    assert!(json_str.contains("checks"));
}

#[tokio::test]
async fn test_doctor_identifies_missing_skills_library() {
    let engine = DoctorEngine {
        skills_library_path: Some("/nonexistent/skills/path".to_string()),
        presets_yaml_path: Some("/nonexistent/presets.yaml".to_string()),
        ..Default::default()
    };

    let report = engine.run_diagnostics().await;
    let skill_check = report
        .checks
        .iter()
        .find(|c| c.category.contains("8. Skills Library"))
        .expect("Skills check must exist");

    assert!(!skill_check.passed);
    assert!(skill_check.recommendation.is_some());
}
