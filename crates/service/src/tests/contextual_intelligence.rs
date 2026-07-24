use super::*;

#[test]
fn llm_project_health_and_semantic_search_are_bounded_to_current_product_evidence() {
    let app_data_dir = env::temp_dir().join(format!(
        "skills-copilot-llm-contextual-workspaces-{}-{}",
        std::process::id(),
        unique_suffix(),
    ));
    let host = test_host_with_project(app_data_dir.clone());
    seed_catalog_with_llm_skill(&host, &app_data_dir.join("fixture-skill").join("SKILL.md"));

    let project = host.handle(ServiceRequest {
        id: Some("project-health".to_string()),
        method: "llm.previewPrompt".to_string(),
        params: json!({"action": "project_health"}),
    });
    assert!(project.ok, "{:?}", project.error);
    let project = project.result.expect("project health preview");
    assert_eq!(
        project.pointer("/response_contract/result_schema"),
        Some(&json!("copy_only_markdown"))
    );
    assert!(project["prompt_preview"]
        .as_str()
        .is_some_and(|prompt| prompt.contains("accepted deterministic evidence")));

    let semantic_request = json!({
        "action": "semantic_search",
        "user_intent": "find the audit skill",
        "search_candidates": [
            {
                "id": "skill:llm-skill-id",
                "kind": "skill",
                "title": "Audit skill",
                "subtitle": "/private/project/SKILL.md"
            },
            {
                "id": "session:session-1",
                "kind": "session",
                "title": "Audit follow-up",
                "subtitle": "claude-code"
            }
        ]
    });
    let semantic = host.handle(ServiceRequest {
        id: Some("semantic-search".to_string()),
        method: "llm.previewPrompt".to_string(),
        params: semantic_request.clone(),
    });
    assert!(semantic.ok, "{:?}", semantic.error);
    let semantic = semantic.result.expect("semantic preview");
    assert_eq!(
        semantic.pointer("/response_contract/result_schema"),
        Some(&json!("semantic_rerank"))
    );
    let evidence = semantic["response_contract"]["evidence"]
        .as_array()
        .expect("semantic candidate evidence");
    assert_eq!(evidence.len(), 2);
    assert!(evidence.iter().all(|reference| reference["id"]
        .as_str()
        .is_some_and(|id| id.starts_with("search-candidate:"))));
    let target_ids = evidence
        .iter()
        .filter_map(|reference| reference["target_id"].as_str())
        .collect::<BTreeSet<_>>();
    assert_eq!(
        target_ids,
        BTreeSet::from(["skill:llm-skill-id", "session:session-1"])
    );
    let prompt = semantic["prompt_preview"]
        .as_str()
        .expect("semantic prompt");
    assert!(prompt.contains("Rerank only the candidates below"));
    assert!(!prompt.contains("/private/project"));
    assert!(!prompt.contains("\"target_id\""));

    let mut duplicate = semantic_request;
    duplicate["search_candidates"][1]["id"] = json!("skill:llm-skill-id");
    let rejected = host.handle(ServiceRequest {
        id: Some("semantic-search-duplicate".to_string()),
        method: "llm.previewPrompt".to_string(),
        params: duplicate,
    });
    assert!(!rejected.ok);
    assert_eq!(
        rejected.error.as_ref().map(|error| error.code.as_str()),
        Some("invalid_request")
    );

    let _ = fs::remove_dir_all(app_data_dir);
}
