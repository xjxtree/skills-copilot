use super::*;

fn preview(operation: &str) -> SkillManagerCommandPreview {
    SkillManagerCommandPreview {
        action: None,
        preconditions: Vec::new(),
        tool_id: DEFAULT_MANAGER_TOOL.to_string(),
        operation: operation.to_string(),
        command: vec!["/usr/bin/true".to_string(), "--global".to_string()],
        cwd: "/tmp".to_string(),
        env: Vec::new(),
        requires_confirmation: true,
        confirmed: true,
        network_required: false,
        network_allowed: true,
        will_run: true,
        preview_token: "test".to_string(),
        summary: "test".to_string(),
        risks: Vec::new(),
        source: None,
        skills: vec!["selected".to_string()],
    }
}

fn context(label: &str) -> AdapterContext {
    let root = std::env::temp_dir().join(format!(
        "agent-copilot-manager-commit-{label}-{}-{}",
        std::process::id(),
        unix_timestamp_millis()
    ));
    AdapterContext {
        user_home: root.join("home"),
        project_root: None,
        project_cwd: None,
        extra_roots: Vec::new(),
    }
}

#[test]
fn outcome_unknown_is_nonretryable_partial_for_every_manager_mutation_path() {
    let ctx = context("unknown");
    for operation in ["install", "update", "localCreate"] {
        let catalog = Catalog::in_memory().expect("catalog");
        catalog.init().expect("catalog schema");
        catalog.inject_next_commit_outcome_unknown_for_test();
        let transaction = catalog
            .begin_immediate_transaction()
            .expect("catalog transaction");

        let error = commit_manager_catalog_transaction(&ctx, &preview(operation), transaction)
            .expect_err("unknown commit outcome must fail closed");

        assert!(matches!(
            error,
            CommandError::PartialEffect {
                state: "outcome_unknown",
                cleanup_required: true,
                ..
            }
        ));
    }
}

#[test]
fn rejected_commit_reports_preserved_external_effect_for_every_manager_mutation_path() {
    let ctx = context("rejected");
    for operation in ["install", "update", "localCreate"] {
        let catalog = Catalog::in_memory().expect("catalog");
        catalog.init().expect("catalog schema");
        catalog.inject_next_commit_failure_for_test();
        let transaction = catalog
            .begin_immediate_transaction()
            .expect("catalog transaction");

        let error = commit_manager_catalog_transaction(&ctx, &preview(operation), transaction)
            .expect_err("a rejected commit after manager execution is a partial effect");

        assert!(matches!(
            error,
            CommandError::PartialEffect {
                state: "applied_unverified",
                cleanup_required: true,
                ..
            }
        ));
    }
}

#[test]
fn rollback_failure_is_outcome_unknown_for_every_manager_mutation_path() {
    let ctx = context("rollback-unknown");
    for operation in ["install", "remove", "update", "localCreate"] {
        let catalog = Catalog::in_memory().expect("catalog");
        catalog.init().expect("catalog schema");
        catalog.inject_next_rollback_failure_for_test();
        let transaction = catalog
            .begin_immediate_transaction()
            .expect("catalog transaction");

        let error = rollback_manager_catalog_transaction(
            &ctx,
            &preview(operation),
            transaction,
            CommandError::VerificationFailed,
        );

        assert!(matches!(
            error,
            CommandError::PartialEffect {
                state: "outcome_unknown",
                cleanup_required: true,
                ..
            }
        ));
    }
}

#[test]
fn proven_manager_rollback_preserves_the_original_error_classification() {
    let ctx = context("rollback-proven");
    let catalog = Catalog::in_memory().expect("catalog");
    catalog.init().expect("catalog schema");
    let transaction = catalog
        .begin_immediate_transaction()
        .expect("catalog transaction");

    let error = rollback_manager_catalog_transaction(
        &ctx,
        &preview("install"),
        transaction,
        CommandError::SkillManagerCommandFailed("not started".to_string()),
    );

    assert!(matches!(error, CommandError::SkillManagerCommandFailed(_)));
}
