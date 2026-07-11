use super::*;
use skills_copilot_core::{ListIncompleteReason, ListSourceCompleteness};

fn test_preview(operation: &str) -> SkillManagerCommandPreview {
    SkillManagerCommandPreview {
        tool_id: DEFAULT_MANAGER_TOOL.to_string(),
        operation: operation.to_string(),
        command: vec!["/usr/bin/false".to_string()],
        cwd: "/tmp".to_string(),
        env: Vec::new(),
        requires_confirmation: false,
        confirmed: false,
        network_required: operation == "search",
        network_allowed: true,
        will_run: false,
        preview_token: "test-token".to_string(),
        summary: "test".to_string(),
        risks: Vec::new(),
        source: None,
        skills: Vec::new(),
    }
}

#[test]
fn search_parser_extracts_ansi_find_results() {
    let stdout = "\n\u{1b}[38;5;102mInstall with\u{1b}[0m npx skills add <owner/repo@skill>\n\n\u{1b}[38;5;145mobra/superpowers@brainstorming\u{1b}[0m \u{1b}[36m245.4K installs\u{1b}[0m\n\u{1b}[38;5;102m└ https://skills.sh/obra/superpowers/brainstorming\u{1b}[0m\n\n\u{1b}[38;5;145mobra/superpowers@systematic-debugging\u{1b}[0m \u{1b}[36m161.5K installs\u{1b}[0m\n\u{1b}[38;5;102m└ https://skills.sh/obra/superpowers/systematic-debugging\u{1b}[0m\n";

    let results = parse_search_results(stdout);

    assert_eq!(results.len(), 2);
    assert_eq!(results[0].name, "brainstorming");
    assert_eq!(results[0].source.as_deref(), Some("obra/superpowers"));
    assert_eq!(results[0].description.as_deref(), Some("245.4K installs"));
    assert_eq!(results[1].name, "systematic-debugging");
    assert_eq!(results[1].source.as_deref(), Some("obra/superpowers"));
}

#[test]
fn search_parser_does_not_cap_returned_manager_rows() {
    let stdout = (0..55)
        .map(|index| format!("owner/repo@skill-{index} Result {index}"))
        .collect::<Vec<_>>()
        .join("\n");

    let results = parse_search_results(&stdout);

    assert_eq!(results.len(), 55);
}

#[test]
fn search_record_preserves_all_returned_rows_without_claiming_source_total() {
    let stdout = serde_json::to_string(
        &(0..35)
            .map(|index| {
                serde_json::json!({
                    "name": format!("skill-{index}"),
                    "source": "owner/repo",
                    "description": format!("Result {index}")
                })
            })
            .collect::<Vec<_>>(),
    )
    .expect("search JSON");
    let search =
        skill_manager_search_record(test_preview("search"), None, parse_search_results(&stdout));

    assert_eq!(search.results.len(), 35);
    assert_eq!(search.page.returned_count, 35);
    assert_eq!(search.page.total_count, None);
    assert_eq!(
        search.page.source_completeness,
        ListSourceCompleteness::Unknown
    );
    assert_eq!(
        search.page.incomplete_reason,
        Some(ListIncompleteReason::SourceLimited)
    );
}

#[test]
fn installed_record_reports_exact_enumerable_total() {
    let stdout = serde_json::to_string(&serde_json::json!({
        "skills": (0..27)
            .map(|index| serde_json::json!({
                "name": format!("installed-{index}"),
                "source": "owner/repo"
            }))
            .collect::<Vec<_>>()
    }))
    .expect("installed JSON");
    let installed = skill_manager_installed_record(
        test_preview("listInstalled"),
        SkillManagerCommandOutput {
            status: "completed".to_string(),
            exit_code: Some(0),
            stdout: stdout.clone(),
            stderr: String::new(),
        },
        parse_installed_records(&stdout).expect("valid installed JSON"),
    );

    assert_eq!(installed.installed.len(), 27);
    assert_eq!(installed.page.returned_count, 27);
    assert_eq!(installed.page.total_count, Some(27));
    assert_eq!(
        installed.page.source_completeness,
        ListSourceCompleteness::Enumerable
    );
    assert_eq!(installed.page.incomplete_reason, None);
}

#[test]
fn installed_parser_rejects_truncated_json_instead_of_claiming_exact_empty() {
    let stdout = serde_json::to_string(&serde_json::json!({
        "skills": (0..400)
            .map(|index| serde_json::json!({
                "name": format!("installed-{index}"),
                "source": format!("owner/repository-with-a-long-name-{index}"),
                "path": format!("/tmp/installed/{index}/SKILL.md")
            }))
            .collect::<Vec<_>>()
    }))
    .expect("large installed JSON");
    assert!(stdout.len() > MAX_CAPTURE_BYTES);
    let truncated = truncate_capture(&stdout);

    let result = parse_installed_records(&truncated);

    assert!(matches!(
        result,
        Err(CommandError::SkillManagerCommandFailed(detail))
            if detail == "listInstalled returned invalid or truncated JSON"
    ));
}

#[test]
fn installed_parser_rejects_malformed_or_unrecognized_json_instead_of_exact_empty() {
    for stdout in ["{not-json", r#"{"unexpected":[]}"#] {
        let result = parse_installed_records(stdout);

        assert!(matches!(
            result,
            Err(CommandError::SkillManagerCommandFailed(detail))
                if detail == "listInstalled returned invalid or truncated JSON"
        ));
    }
}

#[test]
fn installed_parser_accepts_a_recognized_exact_empty_list() {
    let installed =
        parse_installed_records(r#"{"skills":[]}"#).expect("recognized empty installed list");

    assert!(installed.is_empty());
}

#[test]
fn search_without_network_does_not_create_manager_cwd() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock")
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "skill-manager-search-no-network-{}-{unique}",
        std::process::id()
    ));
    let ctx = AdapterContext {
        user_home: root.join("home"),
        project_root: None,
        project_cwd: None,
        extra_roots: Vec::new(),
    };

    let record = search_skills_with_manager(
        &ctx,
        &SkillManagerSearchParams {
            query: "local-only".to_string(),
            owner: None,
            network_allowed: false,
        },
    )
    .expect("preview prohibited network search");

    assert!(record.output.is_none());
    assert!(record.results.is_empty());
    assert_eq!(record.page.returned_count, 0);
    assert_eq!(record.page.total_count, None);
    assert_eq!(
        record.page.source_completeness,
        ListSourceCompleteness::Unknown
    );
    assert_eq!(
        record.page.incomplete_reason,
        Some(ListIncompleteReason::SourceLimited)
    );
    assert!(!ctx.user_home.exists());
    let _ = fs::remove_dir_all(root);
}

#[test]
fn prohibited_previewed_command_does_not_create_cwd() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock")
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "skill-manager-prohibited-command-{}-{unique}",
        std::process::id()
    ));
    let cwd = root.join("missing-cwd");
    let ctx = AdapterContext {
        user_home: root.join("home"),
        project_root: None,
        project_cwd: None,
        extra_roots: Vec::new(),
    };
    let preview = SkillManagerCommandPreview {
        tool_id: DEFAULT_MANAGER_TOOL.to_string(),
        operation: "search".to_string(),
        command: vec!["/usr/bin/false".to_string()],
        cwd: cwd.to_string_lossy().to_string(),
        env: Vec::new(),
        requires_confirmation: false,
        confirmed: false,
        network_required: true,
        network_allowed: false,
        will_run: false,
        preview_token: "test-token".to_string(),
        summary: "test".to_string(),
        risks: Vec::new(),
        source: None,
        skills: Vec::new(),
    };

    let result = run_previewed_command(&ctx, &preview);

    assert!(matches!(
        result,
        Err(CommandError::InvalidSkillManagerRequest(_))
    ));
    assert!(!cwd.exists());
    let _ = fs::remove_dir_all(root);
}
