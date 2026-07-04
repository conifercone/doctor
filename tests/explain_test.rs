#[test]
fn test_structured_summary_builder() {
    // Test that summary builder works with an empty report
    // (full integration test requires a real LLM API key)
    let overview = doctor::model::SystemOverview::new(
        doctor::model::system_overview::BuildTool::Maven,
        Some("3.2.0".into()),
        Some("17".into()),
        vec!["spring-boot-starter-web".into()],
        1,
    );
    let summary = doctor::model::summary::DiagnosisSummary::new(
        8,
        50,
        std::collections::HashMap::new(),
        true,
    );
    let report = doctor::model::DiagnosticReport::new(
        "test-project",
        1000,
        overview,
        vec![],
        summary,
    );
    let structured = doctor::ai::summary::build_summary(&report);
    assert_eq!(
        structured.project_context.spring_boot_version,
        Some("3.2.0".into())
    );
    assert!(structured.issues.is_empty());
}

#[test]
fn test_summary_strips_file_paths() {
    use doctor::evidence::{Evidence, EvidenceType, Reliability};
    use doctor::model::{Category, Confidence, Issue, Severity};

    let evidence = Evidence::new(
        EvidenceType::SourceCode,
        "src/main/java/com/example/App.java:25",
        "Found @Autowired com.example.UserService",
        Reliability::Confirmed,
    );
    let issue = Issue::new(
        "BEAN-001",
        "Test issue",
        Severity::Warning,
        Category::Bean,
        "Test description",
        vec![evidence],
        "Fix it",
        Confidence::High,
    )
    .unwrap();

    let overview = doctor::model::SystemOverview::new(
        doctor::model::system_overview::BuildTool::Maven,
        None,
        None,
        vec![],
        1,
    );
    let summary = doctor::model::summary::DiagnosisSummary::new(
        1,
        1,
        std::collections::HashMap::new(),
        false,
    );
    let report = doctor::model::DiagnosticReport::new(
        "test",
        0,
        overview,
        vec![issue],
        summary,
    );

    let structured = doctor::ai::summary::build_summary(&report);
    // key_classes should NOT contain the file path
    for issue_summary in &structured.issues {
        for cls in &issue_summary.key_classes {
            assert!(
                !cls.contains('/'),
                "key_classes must not contain file paths: {cls}"
            );
            assert!(
                !cls.contains(".java"),
                "key_classes must not contain .java files: {cls}"
            );
        }
    }
}
