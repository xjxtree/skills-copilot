use super::*;

fn exercise_composite_commit_failure(outcome_unknown: bool) {
    let label = if outcome_unknown {
        "unknown"
    } else {
        "rejected"
    };
    let root = std::env::temp_dir().join(format!(
        "skill-manager-composite-commit-{label}-{}-{}",
        std::process::id(),
        unix_timestamp_millis()
    ));
    let app_data = root.join("app-data");
    let skill_name = format!("composite-{label}");
    let skill_directory = tool_global_staging_skills_root(&app_data).join(&skill_name);
    let skill_path = skill_directory.join("SKILL.md");
    fs::create_dir_all(&skill_directory).expect("app-owned skill");
    fs::write(
        &skill_path,
        format!("---\nname: {skill_name}\ndescription: Fixture\n---\n# Fixture\n"),
    )
    .expect("skill file");
    let canonical_skill = skill_path.canonicalize().expect("canonical skill");
    let catalog = Catalog::open(&root.join("catalog.sqlite")).expect("catalog");
    catalog.init().expect("catalog schema");
    catalog
        .upsert_skill_instance(&tests::test_instance(
            "tool-composite",
            AgentId::ToolGlobal,
            Scope::ToolGlobal,
            &skill_name,
            canonical_skill.clone(),
            canonical_skill.clone(),
        ))
        .expect("catalog fixture");
    let plan = CompositeLocalDeletePlan {
        instance_id: "tool-composite".to_string(),
        skill_name,
        skill_path: canonical_skill.clone(),
        skill_directory: skill_directory.clone(),
        catalog_revision: "fixture".to_string(),
        tree_revision: local_delete_tree_revision(&canonical_skill).expect("tree revision"),
    };
    if outcome_unknown {
        catalog.inject_next_commit_outcome_unknown_for_test();
    } else {
        catalog.inject_next_commit_failure_for_test();
    }
    let transaction = catalog
        .begin_immediate_transaction()
        .expect("catalog transaction");
    catalog
        .delete_skill_instance("tool-composite")
        .expect("transactional catalog delete");
    let quarantine = plan
        .skill_directory
        .with_file_name(format!(".agent-copilot-delete-{label}"));
    fs::rename(&plan.skill_directory, &quarantine).expect("quarantine local source");
    let quarantine_tree_revision =
        local_delete_tree_revision(&quarantine.join("SKILL.md")).expect("quarantine revision");
    let mut cleanup = CompositeLocalDeleteMutation {
        quarantine,
        quarantine_tree_revision,
        original_directory: plan.skill_directory.clone(),
        original_skill_path: plan.skill_path.clone(),
        original_tree_revision: plan.tree_revision.clone(),
        observations: Vec::new(),
        active: true,
    };

    let disposition = commit_composite_local_delete(transaction, Some(&mut cleanup));

    if outcome_unknown {
        assert!(matches!(
            disposition,
            CompositeLocalDeleteCommit::OutcomeUnknown(CatalogError::InjectedCommitOutcomeUnknown)
        ));
        assert!(
            !skill_directory.exists(),
            "uncertain commit must not guess that restoration is safe"
        );
        assert!(
            fs::read_dir(tool_global_staging_skills_root(&app_data))
                .expect("staging root")
                .filter_map(Result::ok)
                .any(|entry| entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with(".agent-copilot-delete-")),
            "uncertain commit must retain private restoration material"
        );
    } else {
        assert!(
            matches!(
                &disposition,
                CompositeLocalDeleteCommit::NotCommitted(CatalogError::InjectedCommitFailure)
            ),
            "unexpected disposition: {disposition:?}"
        );
        assert!(
            skill_directory.join("SKILL.md").is_file(),
            "proven-uncommitted delete must restore the original tree"
        );
    }
    fs::remove_dir_all(root).ok();
}

#[test]
fn rejected_composite_commit_restores_local_source() {
    exercise_composite_commit_failure(false);
}

#[test]
fn uncertain_composite_commit_retains_quarantine() {
    exercise_composite_commit_failure(true);
}
