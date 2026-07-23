use std::collections::{BTreeMap, BTreeSet, HashMap};

use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};
use skills_copilot_catalog::{
    CatalogScanCoverageRecord, CatalogScanRevisionRecord, CatalogSkillProjectionRecord,
    ConflictGroupRecord, RuleFindingRecord,
};
use skills_copilot_commands::{
    derive_product_projection, list_conflicts, list_findings,
    list_projected_skill_instances_with_config_revision, product_skill_config_revision,
    AgentProjectionInput, ConflictProjectionInput, FindingProjectionInput, ProductProjection,
    ProductProjectionInput, SkillProjectionInput,
};
use skills_copilot_core::{
    AgentId, ListIncompleteReason, ListPageMetadata, ProjectReadinessRecord, SkillAggregateRecord,
    SkillState, SourceCoverage,
};

use crate::service_keyset_cursor::{decode_cursor, encode_cursor, KeysetCursor};
use crate::{
    effective_project_context_revision, load_project_context_state, ServiceError, ServiceHost,
};

const MAX_PRODUCT_SKILLS: usize = 500;
const MAX_PRODUCT_FINDINGS: usize = 1_000;
const MAX_PRODUCT_CONFLICTS: usize = 500;
const DEFAULT_AGGREGATE_LIMIT: usize = 100;
const MAX_AGGREGATE_LIMIT: usize = 100;

const PRODUCT_AGENTS: [AgentId; 6] = [
    AgentId::ClaudeCode,
    AgentId::Codex,
    AgentId::Opencode,
    AgentId::Pi,
    AgentId::Hermes,
    AgentId::Openclaw,
];

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct ProductReadParams {
    pub project_id: String,
    pub expected_project_context_revision: String,
    #[serde(default)]
    pub source_revision: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct SkillAggregateListParams {
    pub project_id: String,
    pub expected_project_context_revision: String,
    #[serde(default)]
    pub agent: Option<String>,
    #[serde(default)]
    pub limit: Option<usize>,
    #[serde(default)]
    pub cursor: Option<String>,
    #[serde(default)]
    pub source_revision: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct SkillAggregateListResult {
    pub source_revision: String,
    pub coverage: SourceCoverage,
    pub page: ListPageMetadata,
    pub aggregates: Vec<SkillAggregateRecord>,
}

#[derive(Debug, Clone)]
struct AcceptedProject {
    id: String,
    display_name: String,
    context_revision: String,
}

#[derive(Debug, Clone)]
pub(crate) struct AcceptedProductSnapshot {
    pub project_id: String,
    pub context_revision: String,
    pub source_revision: String,
    pub projection: ProductProjection,
    accepted_project: AcceptedProject,
}

impl ServiceHost {
    pub(crate) fn accept_active_product_snapshot(
        &self,
        expected_product_source_revision: Option<&str>,
    ) -> Result<AcceptedProductSnapshot, ServiceError> {
        let env_context = self.env_project_context();
        let context_revision =
            effective_project_context_revision(&self.app_data_dir, env_context.as_ref())?;
        let active = match env_context {
            Some(active) => active,
            None => load_project_context_state(&self.app_data_dir)?
                .active
                .ok_or(ServiceError::ProjectContextRequired)?,
        };
        if active.validation_error.is_some() {
            return Err(ServiceError::ProjectContextRequired);
        }
        self.accept_current_product_snapshot(
            &active.id,
            &context_revision,
            expected_product_source_revision,
        )
    }

    /// Accepts one current project/catalog/config snapshot for additive product
    /// reads. Session resume integration uses this seam instead of duplicating
    /// active-project and revision checks. The optional revision is the product
    /// projection revision, never an adapter-native session inventory revision.
    pub(crate) fn accept_current_product_snapshot(
        &self,
        project_id: &str,
        expected_project_context_revision: &str,
        expected_product_source_revision: Option<&str>,
    ) -> Result<AcceptedProductSnapshot, ServiceError> {
        validate_optional_revision(expected_product_source_revision)?;
        let accepted =
            self.accept_product_project(project_id, expected_project_context_revision)?;
        let projection = self.product_projection_snapshot(&accepted)?;
        let source_revision = projection.readiness.source_revision.clone();
        if expected_product_source_revision.is_some_and(|expected| expected != source_revision) {
            return Err(ServiceError::SourceChanged);
        }
        Ok(AcceptedProductSnapshot {
            project_id: accepted.id.clone(),
            context_revision: accepted.context_revision.clone(),
            source_revision,
            projection,
            accepted_project: accepted,
        })
    }

    pub(crate) fn revalidate_current_product_snapshot(
        &self,
        snapshot: &AcceptedProductSnapshot,
    ) -> Result<(), ServiceError> {
        if snapshot.project_id != snapshot.accepted_project.id
            || snapshot.context_revision != snapshot.accepted_project.context_revision
            || snapshot.source_revision != snapshot.projection.readiness.source_revision
        {
            return Err(ServiceError::InvalidRequest(
                "accepted product snapshot binding is invalid".to_string(),
            ));
        }
        self.revalidate_product_project(&snapshot.accepted_project)?;
        let current = self.product_projection_snapshot(&snapshot.accepted_project)?;
        if current.readiness.source_revision != snapshot.source_revision {
            return Err(ServiceError::SourceChanged);
        }
        Ok(())
    }

    pub(crate) fn project_readiness(
        &self,
        params: ProductReadParams,
    ) -> Result<ProjectReadinessRecord, ServiceError> {
        let snapshot = self.accept_current_product_snapshot(
            &params.project_id,
            &params.expected_project_context_revision,
            params.source_revision.as_deref(),
        )?;
        self.revalidate_current_product_snapshot(&snapshot)?;
        Ok(snapshot.projection.readiness)
    }

    pub(crate) fn list_skill_aggregates(
        &self,
        params: SkillAggregateListParams,
    ) -> Result<SkillAggregateListResult, ServiceError> {
        let snapshot = self.accept_current_product_snapshot(
            &params.project_id,
            &params.expected_project_context_revision,
            params.source_revision.as_deref(),
        )?;
        let agent = params
            .agent
            .as_deref()
            .map(parse_product_agent)
            .transpose()?;
        let limit = params
            .limit
            .unwrap_or(DEFAULT_AGGREGATE_LIMIT)
            .clamp(1, MAX_AGGREGATE_LIMIT);
        let query_digest = product_query_digest(&snapshot.project_id, agent)?;
        let cursor = params
            .cursor
            .as_deref()
            .map(|cursor| decode_cursor(cursor, "catalog.listSkillAggregates", &query_digest))
            .transpose()?;
        let source_revision = snapshot.source_revision.clone();
        if cursor
            .as_ref()
            .is_some_and(|cursor| cursor.source_revision != source_revision)
        {
            return Err(ServiceError::SourceChanged);
        }

        let coverage = agent
            .and_then(|agent| {
                snapshot
                    .projection
                    .readiness
                    .agents
                    .iter()
                    .find(|record| record.agent == agent)
                    .map(|record| record.coverage.clone())
            })
            .unwrap_or_else(|| snapshot.projection.readiness.coverage.clone());
        let filtered = snapshot
            .projection
            .skill_aggregates
            .iter()
            .filter(|aggregate| agent.is_none_or(|agent| aggregate.agents.contains(&agent)))
            .cloned()
            .collect::<Vec<_>>();
        let start = match cursor.as_ref() {
            Some(cursor) => filtered
                .iter()
                .position(|aggregate| aggregate.id == cursor.stable_id)
                .map(|position| position.saturating_add(1))
                .ok_or(ServiceError::SourceChanged)?,
            None => 0,
        };
        let total_count = filtered.len();
        let end = start.saturating_add(limit).min(total_count);
        let aggregates = filtered[start..end].to_vec();
        let has_more = end < total_count;
        let next_cursor = if has_more {
            aggregates
                .last()
                .map(|aggregate| {
                    encode_cursor(&KeysetCursor {
                        version: 1,
                        method: "catalog.listSkillAggregates".to_string(),
                        query_digest: query_digest.clone(),
                        source_revision: source_revision.clone(),
                        sort_value: 0,
                        stable_id: aggregate.id.clone(),
                        tie_breaker_digest: None,
                        accepted_count: None,
                        processed_prefix_digest: None,
                        resolved_start_at: None,
                        resolved_end_at: None,
                    })
                })
                .transpose()?
        } else {
            None
        };
        let page = ListPageMetadata {
            returned_count: aggregates.len(),
            total_count: coverage.is_complete().then_some(total_count),
            has_more,
            next_cursor,
            source_completeness: coverage.completeness,
            incomplete_reason: coverage.incomplete_reason,
        };
        page.validate(aggregates.len())
            .map_err(|error| ServiceError::InvalidRequest(error.to_string()))?;
        self.revalidate_current_product_snapshot(&snapshot)?;
        Ok(SkillAggregateListResult {
            source_revision,
            coverage,
            page,
            aggregates,
        })
    }

    fn accept_product_project(
        &self,
        requested_project_id: &str,
        expected_context_revision: &str,
    ) -> Result<AcceptedProject, ServiceError> {
        if requested_project_id.trim().is_empty() || expected_context_revision.trim().is_empty() {
            return Err(ServiceError::InvalidRequest(
                "project_id and expected_project_context_revision are required".to_string(),
            ));
        }
        let env_context = self.env_project_context();
        let context_revision =
            effective_project_context_revision(&self.app_data_dir, env_context.as_ref())?;
        if expected_context_revision != context_revision {
            return Err(ServiceError::StaleProjectContext);
        }
        let active = match env_context {
            Some(active) => active,
            None => load_project_context_state(&self.app_data_dir)?
                .active
                .ok_or(ServiceError::ProjectContextRequired)?,
        };
        if active.validation_error.is_some() {
            return Err(ServiceError::ProjectContextRequired);
        }
        if requested_project_id != active.id {
            return Err(ServiceError::ProjectContextMismatch);
        }
        Ok(AcceptedProject {
            id: active.id,
            display_name: active.name,
            context_revision,
        })
    }

    fn revalidate_product_project(&self, accepted: &AcceptedProject) -> Result<(), ServiceError> {
        let env_context = self.env_project_context();
        let revision =
            effective_project_context_revision(&self.app_data_dir, env_context.as_ref())?;
        if revision != accepted.context_revision {
            return Err(ServiceError::StaleProjectContext);
        }
        let active_id = match env_context {
            Some(active) => Some(active.id),
            None => load_project_context_state(&self.app_data_dir)?
                .active
                .map(|active| active.id),
        };
        if active_id.as_deref() != Some(accepted.id.as_str()) {
            return Err(ServiceError::ProjectContextMismatch);
        }
        Ok(())
    }

    fn product_projection_snapshot(
        &self,
        accepted: &AcceptedProject,
    ) -> Result<ProductProjection, ServiceError> {
        let catalog = self.open_catalog_for_read()?;
        let adapter_ctx = self.effective_adapter_ctx()?;
        let (projection, accepted_config_revision) = catalog.with_read_snapshot(|catalog| {
            let scan_revision = catalog.catalog_scan_revision()?;
            let scan_coverages = catalog.list_catalog_scan_coverages(&accepted.context_revision)?;
            let skill_metadata =
                catalog.list_catalog_skill_projections(&accepted.context_revision)?;
            let accepted_skills =
                list_projected_skill_instances_with_config_revision(catalog, &adapter_ctx)?;
            let config_revision = accepted_skills.config_revision;
            let mut instances = accepted_skills.instances;
            let records = catalog
                .list_skill_records_for_project_context(adapter_ctx.project_root.as_deref())?;
            let mut findings = list_findings(catalog)?;
            let mut conflicts = list_conflicts(catalog)?;
            let config_revision_after = product_skill_config_revision(&adapter_ctx)?;
            if config_revision != config_revision_after {
                return Err(ServiceError::SourceChanged);
            }

            instances.sort_by(|left, right| left.id.cmp(&right.id));
            findings.sort_by(|left, right| left.id.cmp(&right.id));
            conflicts.sort_by(|left, right| left.id.cmp(&right.id));
            let budget_limited = instances.len() > MAX_PRODUCT_SKILLS
                || findings.len() > MAX_PRODUCT_FINDINGS
                || conflicts.len() > MAX_PRODUCT_CONFLICTS;
            instances.truncate(MAX_PRODUCT_SKILLS);
            findings.truncate(MAX_PRODUCT_FINDINGS);
            conflicts.truncate(MAX_PRODUCT_CONFLICTS);

            let accepted_instance_ids = instances
                .iter()
                .map(|instance| instance.id.as_str())
                .collect::<BTreeSet<_>>();
            findings.retain(|finding| {
                finding
                    .instance_id
                    .as_deref()
                    .is_some_and(|id| accepted_instance_ids.contains(id))
            });
            conflicts.retain(|conflict| {
                conflict
                    .instance_ids
                    .iter()
                    .all(|id| accepted_instance_ids.contains(id.as_str()))
            });

            let source_revision = product_source_revision(
                accepted,
                &scan_revision,
                &scan_coverages,
                &skill_metadata,
                &instances,
                &findings,
                &conflicts,
                &config_revision,
                budget_limited,
            )?;
            let agent_sources = product_agent_sources(
                &source_revision,
                &scan_revision,
                scan_coverages,
                budget_limited,
            );
            let coverage_by_agent = agent_sources
                .iter()
                .map(|source| (source.agent, source.coverage.clone()))
                .collect::<HashMap<_, _>>();
            let metadata_by_id = skill_metadata
                .into_iter()
                .map(|metadata| (metadata.instance_id.clone(), metadata))
                .collect::<BTreeMap<_, _>>();
            let records_by_id = records
                .into_iter()
                .map(|record| (record.id.clone(), record))
                .collect::<BTreeMap<_, _>>();
            let skills = instances
                .into_iter()
                .map(|instance| {
                    let metadata = metadata_by_id.get(&instance.id).filter(|metadata| {
                        metadata.agent == instance.agent
                            && metadata.context_revision == accepted.context_revision
                            && metadata.catalog_scan_generation > 0
                            && metadata.catalog_scan_generation <= scan_revision.generation
                            && !metadata.catalog_scan_revision.is_empty()
                    });
                    let persisted_coverage = metadata.map(|metadata| metadata.coverage.clone());
                    let coverage = if budget_limited {
                        SourceCoverage::incomplete(
                            persisted_coverage
                                .as_ref()
                                .map_or(0, |coverage| coverage.inspected_sources),
                            persisted_coverage
                                .as_ref()
                                .and_then(|coverage| coverage.expected_sources),
                            ListIncompleteReason::SafetyBudget,
                        )
                    } else {
                        persisted_coverage.unwrap_or_else(|| {
                            SourceCoverage::unknown(ListIncompleteReason::NotInspected)
                        })
                    };
                    let record = records_by_id.get(&instance.id);
                    let unavailable_identity = format!("unavailable:not_inspected:{}", instance.id);
                    SkillProjectionInput {
                        instance_id: instance.id,
                        agent: (instance.agent != AgentId::ToolGlobal).then_some(instance.agent),
                        scope: instance.scope,
                        definition_id: instance.definition_id,
                        definition_fingerprint: Some(instance.fingerprint),
                        canonical_name: instance.name,
                        display_name: instance.display_name,
                        description: instance.description,
                        publisher: record.and_then(|record| record.publisher.clone()),
                        package_name: record.and_then(|record| record.package_name.clone()),
                        package_version: record
                            .and_then(|record| record.package_version.clone())
                            .or(instance.version),
                        source_kind: metadata
                            .map(|metadata| metadata.source_kind.clone())
                            .unwrap_or_else(|| "unavailable".to_string()),
                        source_identity: metadata
                            .map(|metadata| metadata.source_identity.clone())
                            .unwrap_or_else(|| unavailable_identity.clone()),
                        runtime_identity: metadata
                            .map(|metadata| metadata.runtime_identity.clone())
                            .unwrap_or(unavailable_identity),
                        source_revision: source_revision.clone(),
                        read_only_reason: record.and_then(|record| record.read_only_reason.clone()),
                        installed: metadata.is_some() && instance.state != SkillState::Missing,
                        linked: metadata.is_some_and(|metadata| metadata.linked),
                        enabled: instance.enabled,
                        precedence_proven: metadata
                            .is_some_and(|metadata| metadata.precedence_proven),
                        adapter_state: instance.state,
                        coverage: coverage_by_agent
                            .get(&instance.agent)
                            .map(|agent_coverage| {
                                merge_instance_coverage(&coverage, agent_coverage)
                            })
                            .unwrap_or(coverage),
                        evidence_summary: format!(
                            "{} {} skill evidence was accepted from the local catalog cache",
                            product_agent_label(instance.agent),
                            instance.scope.as_str()
                        ),
                        action_ids: Vec::new(),
                    }
                })
                .collect::<Vec<_>>();
            let findings = findings
                .into_iter()
                .map(|record| FindingProjectionInput {
                    source_revision: source_revision.clone(),
                    record,
                })
                .collect();
            let conflicts = conflicts
                .into_iter()
                .map(|record| ConflictProjectionInput {
                    source_revision: source_revision.clone(),
                    record,
                })
                .collect();
            let projection = derive_product_projection(ProductProjectionInput {
                project_id: accepted.id.clone(),
                project_display_name: accepted.display_name.clone(),
                source_revision,
                agent_sources,
                skills,
                findings,
                conflicts,
                sessions: Vec::new(),
                actions: Vec::new(),
            })
            .map_err(|error| ServiceError::InvalidRequest(error.to_string()))?;
            Ok((projection, config_revision))
        })?;
        if product_skill_config_revision(&adapter_ctx)? != accepted_config_revision {
            return Err(ServiceError::SourceChanged);
        }
        Ok(projection)
    }
}

fn product_agent_sources(
    source_revision: &str,
    scan_revision: &CatalogScanRevisionRecord,
    coverages: Vec<CatalogScanCoverageRecord>,
    budget_limited: bool,
) -> Vec<AgentProjectionInput> {
    let by_agent = coverages
        .into_iter()
        .filter(|record| {
            record.catalog_scan_generation > 0
                && record.catalog_scan_generation <= scan_revision.generation
                && !record.catalog_scan_revision.is_empty()
                && record.coverage.validate().is_ok()
        })
        .map(|record| (record.agent, record.coverage))
        .collect::<HashMap<_, _>>();
    PRODUCT_AGENTS
        .into_iter()
        .map(|agent| {
            let Some(coverage) = by_agent.get(&agent).cloned() else {
                return AgentProjectionInput::not_inspected(agent, source_revision);
            };
            AgentProjectionInput {
                agent,
                source_revision: source_revision.to_string(),
                coverage: if budget_limited {
                    SourceCoverage::incomplete(
                        coverage.inspected_sources,
                        coverage.expected_sources,
                        ListIncompleteReason::SafetyBudget,
                    )
                } else {
                    coverage
                },
                evidence_summary: format!(
                    "{} adapter scan coverage was accepted from the local catalog cache",
                    product_agent_label(agent)
                ),
                action_ids: Vec::new(),
            }
        })
        .collect()
}

#[allow(clippy::too_many_arguments)]
fn product_source_revision(
    accepted: &AcceptedProject,
    scan_revision: &CatalogScanRevisionRecord,
    coverages: &[CatalogScanCoverageRecord],
    skill_metadata: &[CatalogSkillProjectionRecord],
    instances: &[skills_copilot_core::SkillInstance],
    findings: &[RuleFindingRecord],
    conflicts: &[ConflictGroupRecord],
    config_revision: &str,
    budget_limited: bool,
) -> Result<String, ServiceError> {
    let value = json!({
        "project_id": accepted.id,
        "project_context_revision": accepted.context_revision,
        "catalog_scan_generation": scan_revision.generation,
        "catalog_scan_revision": scan_revision.revision,
        "skill_config_revision": config_revision,
        "budget_limited": budget_limited,
        "coverages": coverages.iter().map(|record| json!({
            "agent": record.agent.as_str(),
            "context_revision": record.context_revision,
            "catalog_scan_generation": record.catalog_scan_generation,
            "catalog_scan_revision": record.catalog_scan_revision,
            "coverage": record.coverage,
        })).collect::<Vec<_>>(),
        "skill_metadata": skill_metadata.iter().map(|record| json!({
            "instance_id": record.instance_id,
            "agent": record.agent.as_str(),
            "context_revision": record.context_revision,
            "catalog_scan_generation": record.catalog_scan_generation,
            "catalog_scan_revision": record.catalog_scan_revision,
            "source_kind": record.source_kind,
            "source_identity": record.source_identity,
            "runtime_identity": record.runtime_identity,
            "linked": record.linked,
            "precedence_proven": record.precedence_proven,
            "coverage": record.coverage,
        })).collect::<Vec<_>>(),
        "skills": instances.iter().map(|instance| json!({
            "id": instance.id,
            "agent": instance.agent.as_str(),
            "scope": instance.scope.as_str(),
            "definition_id": instance.definition_id,
            "name": instance.name,
            "display_name": instance.display_name,
            "description": instance.description,
            "version": instance.version,
            "state": instance.state.as_str(),
            "enabled": instance.enabled,
            "fingerprint": instance.fingerprint,
            "mtime": instance.mtime,
            "last_seen": instance.last_seen,
        })).collect::<Vec<_>>(),
        "findings": findings,
        "conflicts": conflicts.iter().map(|conflict| json!({
            "id": conflict.id,
            "definition_id": conflict.definition_id,
            "reason": conflict.reason,
            "winner_id": conflict.winner_id,
            "instance_ids": conflict.instance_ids,
        })).collect::<Vec<_>>(),
    });
    let mut hasher = Sha256::new();
    hasher.update(b"agent-copilot/product-projection-snapshot/v1");
    hasher.update([0]);
    hasher.update(serde_json::to_vec(&value)?);
    Ok(format!("sha256:{:x}", hasher.finalize()))
}

fn merge_instance_coverage(instance: &SourceCoverage, agent: &SourceCoverage) -> SourceCoverage {
    if !instance.is_complete() {
        return instance.clone();
    }
    if !agent.is_complete() {
        return agent.clone();
    }
    instance.clone()
}

fn parse_product_agent(value: &str) -> Result<AgentId, ServiceError> {
    match value {
        "claude-code" => Ok(AgentId::ClaudeCode),
        "codex" => Ok(AgentId::Codex),
        "opencode" => Ok(AgentId::Opencode),
        "pi" => Ok(AgentId::Pi),
        "hermes" => Ok(AgentId::Hermes),
        "openclaw" => Ok(AgentId::Openclaw),
        _ => Err(ServiceError::InvalidRequest(
            "agent must be a supported product agent".to_string(),
        )),
    }
}

fn product_query_digest(project_id: &str, agent: Option<AgentId>) -> Result<String, ServiceError> {
    let mut hasher = Sha256::new();
    hasher.update(b"catalog.listSkillAggregates/query/v1");
    hasher.update([0]);
    hasher.update(project_id.as_bytes());
    hasher.update([0]);
    hasher.update(agent.map_or("*", AgentId::as_str).as_bytes());
    Ok(format!("sha256:{:x}", hasher.finalize()))
}

fn validate_optional_revision(revision: Option<&str>) -> Result<(), ServiceError> {
    if revision.is_some_and(|revision| revision.trim().is_empty()) {
        return Err(ServiceError::InvalidRequest(
            "source_revision cannot be empty".to_string(),
        ));
    }
    Ok(())
}

fn product_agent_label(agent: AgentId) -> &'static str {
    match agent {
        AgentId::ToolGlobal => "Tool Global",
        AgentId::ClaudeCode => "Claude Code",
        AgentId::Codex => "Codex",
        AgentId::Opencode => "opencode",
        AgentId::Pi => "Pi",
        AgentId::Hermes => "Hermes",
        AgentId::Openclaw => "OpenClaw",
    }
}
