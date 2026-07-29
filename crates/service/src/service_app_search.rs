use super::*;

impl ServiceHost {
    pub fn search_app(&self, params: AppSearchParams) -> Result<AppSearchResult, ServiceError> {
        let query = params.query.trim();
        let limit_per_kind = params.limit_per_kind.unwrap_or(6).clamp(1, 20);
        if query.is_empty() {
            return Ok(AppSearchResult {
                generated_by: "local-v2.99",
                query: String::new(),
                count: 0,
                total_matched_count: 0,
                limit_per_kind,
                items: Vec::new(),
                read_only: true,
                provider_request_sent: false,
                skill_files_mutated: false,
                agent_config_mutated: false,
                raw_prompt_persisted: false,
                raw_response_persisted: false,
            });
        }

        let normalized_query = query.to_ascii_lowercase();
        let requested_agent = params
            .agent
            .as_deref()
            .map(str::trim)
            .filter(|agent| !agent.is_empty() && *agent != "all");
        let mut items = Vec::new();
        let mut total_matched_count = 0usize;

        if let Some(catalog) = self.open_existing_catalog_read_only()? {
            let adapter_ctx = self.effective_adapter_ctx()?;
            let mut matched_skills = self
                .list_visible_skill_records(&catalog)?
                .into_iter()
                .filter(|skill| requested_agent.is_none_or(|agent| skill.agent == agent))
                .filter(|skill| skill.state != "missing")
                .filter(|skill| {
                    app_search_matches(
                        &normalized_query,
                        [
                            skill.name.as_str(),
                            skill.definition_id.as_str(),
                            skill.agent.as_str(),
                            skill.scope.as_str(),
                            &skill.path.to_string_lossy(),
                            &skill.display_path.to_string_lossy(),
                        ],
                    )
                })
                .collect::<Vec<_>>();
            matched_skills.sort_by(|left, right| {
                left.name
                    .cmp(&right.name)
                    .then_with(|| left.agent.cmp(&right.agent))
                    .then_with(|| left.id.cmp(&right.id))
            });
            total_matched_count += matched_skills.len();
            for skill in matched_skills.into_iter().take(limit_per_kind) {
                let provenance = [
                    skill.publisher.as_deref(),
                    skill.package_name.as_deref(),
                    skill.package_version.as_deref(),
                    skill.source_kind.as_deref(),
                ]
                .into_iter()
                .flatten()
                .filter(|value| !value.is_empty())
                .collect::<Vec<_>>();
                let subtitle = std::iter::once(skill.agent.as_str())
                    .chain(std::iter::once(skill.scope.as_str()))
                    .chain(provenance)
                    .collect::<Vec<_>>()
                    .join(" · ");
                items.push(AppSearchItem {
                    id: format!("skill:{}", skill.id),
                    kind: "skill".to_string(),
                    target_id: skill.id.clone(),
                    title: skill.name.clone(),
                    subtitle,
                    agent: Some(skill.agent.clone()),
                    skill: Some(skill),
                    session: None,
                    config_snapshot: None,
                });
            }

            let mut snapshots = if let Some(agent) = requested_agent {
                list_agent_config_snapshots(&catalog, &adapter_ctx, agent, None)?
            } else {
                list_snapshots(&catalog, &adapter_ctx)?
            };
            snapshots.retain(|snapshot| {
                requested_agent.is_none_or(|agent| snapshot.agent == agent)
                    && app_search_matches(
                        &normalized_query,
                        [
                            snapshot.reason.as_str(),
                            snapshot.target.as_str(),
                            snapshot.agent.as_str(),
                            snapshot.scope.as_str(),
                        ],
                    )
            });
            snapshots.sort_by(|left, right| {
                right
                    .created_at
                    .cmp(&left.created_at)
                    .then_with(|| left.target.cmp(&right.target))
                    .then_with(|| left.id.cmp(&right.id))
            });
            total_matched_count += snapshots.len();
            for snapshot in snapshots.into_iter().take(limit_per_kind) {
                items.push(AppSearchItem {
                    id: format!("config_history:{}", snapshot.id),
                    kind: "config_history".to_string(),
                    target_id: snapshot.id.clone(),
                    title: snapshot.reason.clone(),
                    subtitle: format!(
                        "{} · {} · {} · {}",
                        snapshot.agent, snapshot.scope, snapshot.target, snapshot.created_at
                    ),
                    agent: Some(snapshot.agent.clone()),
                    skill: None,
                    session: None,
                    config_snapshot: Some(snapshot),
                });
            }
        }

        let sessions = self.preview_local_sessions(LocalSessionPreviewParams {
            authorized_roots: params.authorized_roots,
            auto_discover: params.auto_discover,
            agent: requested_agent.map(str::to_string),
            scope: Some("all".to_string()),
            search: Some(query.to_string()),
            project_root: params.project_root,
            current_cwd: params.current_cwd,
            include_content_items: None,
            session_id: None,
            limit: Some(limit_per_kind),
            offset: Some(0),
            paging_mode: None,
            cursor: None,
            source_revision: None,
            sort: Some("recent".to_string()),
            direction: Some("desc".to_string()),
            max_files: Some(1_000),
            max_excerpt_chars: Some(1_000),
        })?;
        total_matched_count += sessions.total_matched_count;
        for session in sessions.session_rows {
            items.push(AppSearchItem {
                id: format!("session:{}", session.id),
                kind: "session".to_string(),
                target_id: session.id.clone(),
                title: session.title.clone(),
                subtitle: [
                    session.agent.clone().unwrap_or_default(),
                    session
                        .project_root
                        .clone()
                        .unwrap_or_else(|| session.scope.clone()),
                    session
                        .ended_at
                        .or(session.started_at)
                        .or(session.modified_at)
                        .map(|value| value.to_string())
                        .unwrap_or_default(),
                ]
                .into_iter()
                .filter(|value| !value.is_empty())
                .collect::<Vec<_>>()
                .join(" · "),
                agent: session.agent.clone(),
                skill: None,
                session: Some(session),
                config_snapshot: None,
            });
        }

        Ok(AppSearchResult {
            generated_by: "local-v2.99",
            query: query.to_string(),
            count: items.len(),
            total_matched_count,
            limit_per_kind,
            items,
            read_only: true,
            provider_request_sent: false,
            skill_files_mutated: false,
            agent_config_mutated: false,
            raw_prompt_persisted: false,
            raw_response_persisted: false,
        })
    }
}

fn app_search_matches<'a>(
    normalized_query: &str,
    values: impl IntoIterator<Item = &'a str>,
) -> bool {
    values
        .into_iter()
        .any(|value| !value.is_empty() && value.to_ascii_lowercase().contains(normalized_query))
}
