use std::{collections::BTreeSet, sync::OnceLock};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use skills_copilot_core::{
    canonical_project_id as project_id_from_root, ActionDescriptor, ActionImpact, ActionIntent,
    ActionKind, ActionNetworkPosture, ActionReadbackDomain, ActionTargetRef, AgentId, Scope,
};

use crate::CommandError;

const ACTION_SOURCE_REVISION_DOMAIN: &str = "agent-copilot/action-source/v1";
const ACTION_PREVIEW_TOKEN_DOMAIN: &str = "agent-copilot/action-preview/v1";
const ACTION_ID_DOMAIN: &str = "agent-copilot/action-id/v1";
pub(crate) const ACTION_PREVIEW_SECRET_ENV: &str = "SKILLS_COPILOT_ACTION_PREVIEW_SECRET";
const HMAC_BLOCK_SIZE: usize = 64;
const ACTION_PREVIEW_SECRET_SIZE: usize = 32;

static ACTION_PREVIEW_SECRET: OnceLock<Result<[u8; ACTION_PREVIEW_SECRET_SIZE], String>> =
    OnceLock::new();

pub fn initialize_action_preview_secret_from_environment() {
    let _ = ACTION_PREVIEW_SECRET.get_or_init(action_preview_secret_from_environment);
}

#[doc(hidden)]
pub fn initialize_action_preview_secret_for_test(
    secret: [u8; ACTION_PREVIEW_SECRET_SIZE],
) -> Result<(), CommandError> {
    let initialized = ACTION_PREVIEW_SECRET.get_or_init(|| Ok(secret));
    match initialized {
        Ok(actual) if actual == &secret => Ok(()),
        Ok(_) => Err(CommandError::ActionTokenUnavailable(
            "action preview test secret was already initialized differently".to_string(),
        )),
        Err(error) => Err(CommandError::ActionTokenUnavailable(error.clone())),
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct ActionReference {
    pub action_id: String,
    pub source_revision: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_id: Option<String>,
    pub target: ActionTargetRef,
}

impl From<&ActionDescriptor> for ActionReference {
    fn from(action: &ActionDescriptor) -> Self {
        Self {
            action_id: action.id.clone(),
            source_revision: action.source_revision.clone(),
            project_id: action.project_id.clone(),
            target: action.target.clone(),
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct ActionConfirmation {
    pub reference: ActionReference,
    pub preview_token: String,
    pub confirmed: bool,
}

impl ActionConfirmation {
    pub fn confirmed(action: &ActionDescriptor, preview_token: impl Into<String>) -> Self {
        Self {
            reference: ActionReference::from(action),
            preview_token: preview_token.into(),
            confirmed: true,
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize)]
pub struct ActionPreviewBinding {
    pub action: ActionDescriptor,
    pub preconditions: Vec<ActionPrecondition>,
    pub preview_token: String,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum ActionPreconditionKind {
    CatalogRecord,
    AgentConfig,
    SourceFile,
    TargetFile,
    ManagerInventory,
    Archive,
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub struct ActionPrecondition {
    pub kind: ActionPreconditionKind,
    pub target_id: String,
    pub expected_revision: String,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct ActionReadbackObservation {
    pub domain: ActionReadbackDomain,
    pub target_id: String,
    pub revision: String,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct ActionReadbackRecord {
    pub action_id: String,
    pub source_revision: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_id: Option<String>,
    pub domains: Vec<ActionReadbackDomain>,
    pub target_ids: Vec<String>,
    pub observations: Vec<ActionReadbackObservation>,
    pub verified: bool,
}

impl ActionReadbackRecord {
    pub(crate) fn verified(
        action: &ActionDescriptor,
        mut observations: Vec<ActionReadbackObservation>,
    ) -> Result<Self, CommandError> {
        observations.sort_by(|left, right| {
            left.domain
                .cmp(&right.domain)
                .then_with(|| left.target_id.cmp(&right.target_id))
        });
        if observations.is_empty()
            || observations
                .iter()
                .any(|observation| !action.readback.contains(&observation.domain))
            || action.readback.iter().any(|domain| {
                !observations
                    .iter()
                    .any(|observation| observation.domain == *domain)
            })
        {
            return Err(CommandError::MismatchedActionReference(
                "readback domains do not exactly match the action contract".to_string(),
            ));
        }
        for observation in &observations {
            ensure_nonempty(&observation.target_id, "readback target")?;
            ensure_nonempty(&observation.revision, "readback revision")?;
        }
        let serialized = serde_json::to_string(&observations)?;
        let source_revision =
            action_source_revision("action.readback", &[("observations", &serialized)])?;
        let target_ids = observations
            .iter()
            .map(|observation| observation.target_id.clone())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        Ok(Self {
            action_id: action.id.clone(),
            source_revision,
            project_id: action.project_id.clone(),
            domains: action.readback.clone(),
            target_ids,
            observations,
            verified: true,
        })
    }
}

pub fn action_source_revision(
    operation: &str,
    fields: &[(&str, &str)],
) -> Result<String, CommandError> {
    ensure_nonempty(operation, "action operation")?;
    let mut hasher = Sha256::new();
    hash_field(&mut hasher, "domain", ACTION_SOURCE_REVISION_DOMAIN);
    hash_field(&mut hasher, "operation", operation);
    for (label, value) in fields {
        ensure_nonempty(label, "action revision field label")?;
        hash_field(&mut hasher, label, value);
    }
    Ok(format!("sha256:{:x}", hasher.finalize()))
}

pub fn deterministic_action_id(
    kind: ActionKind,
    intent: ActionIntent,
    target: &ActionTargetRef,
    project_id: Option<&str>,
) -> Result<String, CommandError> {
    let mut hasher = Sha256::new();
    hash_field(&mut hasher, "domain", ACTION_ID_DOMAIN);
    hash_field(&mut hasher, "kind", action_kind_wire_value(kind));
    hash_field(&mut hasher, "intent", action_intent_wire_value(intent));
    hash_target(&mut hasher, target);
    hash_optional_field(&mut hasher, "project_id", project_id);
    Ok(format!(
        "action:{}:{:x}",
        action_intent_wire_value(intent),
        hasher.finalize()
    ))
}

pub fn action_preview_binding(
    action: ActionDescriptor,
    mut preconditions: Vec<ActionPrecondition>,
) -> Result<ActionPreviewBinding, CommandError> {
    action
        .validate()
        .map_err(|error| CommandError::MismatchedActionReference(error.to_string()))?;
    validate_action_intent(action.kind, action.intent)?;
    validate_action_method_ownership(
        action.kind,
        &action.preview_method,
        action.apply_method.as_deref(),
    )?;
    preconditions.sort();
    for pair in preconditions.windows(2) {
        if pair[0].kind == pair[1].kind && pair[0].target_id == pair[1].target_id {
            return Err(CommandError::MismatchedActionReference(
                "action preconditions contain a duplicate target".to_string(),
            ));
        }
    }
    let mut payload = Sha256::new();
    hash_field(&mut payload, "domain", ACTION_PREVIEW_TOKEN_DOMAIN);
    hash_action_descriptor(&mut payload, &action);
    for precondition in &preconditions {
        ensure_nonempty(&precondition.target_id, "action precondition target")?;
        ensure_nonempty(
            &precondition.expected_revision,
            "action precondition revision",
        )?;
        hash_field(
            &mut payload,
            "precondition_kind",
            precondition_kind_wire_value(precondition.kind),
        );
        hash_field(&mut payload, "precondition_target", &precondition.target_id);
        hash_field(
            &mut payload,
            "precondition_revision",
            &precondition.expected_revision,
        );
    }
    let payload = payload.finalize();
    let signature = action_preview_hmac(payload.as_slice())?;
    Ok(ActionPreviewBinding {
        action,
        preconditions,
        preview_token: format!("action-preview:v1:hmac-sha256:{signature}"),
    })
}

pub fn resolve_action_reference<'a>(
    actions: &'a [ActionDescriptor],
    reference: &ActionReference,
) -> Result<&'a ActionDescriptor, CommandError> {
    let action = actions
        .iter()
        .find(|action| action.id == reference.action_id)
        .ok_or_else(|| CommandError::UnknownActionReference(reference.action_id.clone()))?;
    if action.source_revision != reference.source_revision {
        return Err(CommandError::StaleActionReference);
    }
    if action.project_id != reference.project_id || action.target != reference.target {
        return Err(CommandError::MismatchedActionReference(
            reference.action_id.clone(),
        ));
    }
    Ok(action)
}

pub fn ensure_action_confirmed(
    preview: &ActionPreviewBinding,
    confirmation: Option<&ActionConfirmation>,
) -> Result<(), CommandError> {
    let confirmation = confirmation.ok_or_else(|| {
        CommandError::ActionConfirmationRequired(format!(
            "{} requires its preview confirmation",
            preview.action.id
        ))
    })?;
    if !confirmation.confirmed {
        return Err(CommandError::ActionConfirmationRequired(
            preview.action.id.clone(),
        ));
    }
    let action = resolve_action_reference(
        std::slice::from_ref(&preview.action),
        &confirmation.reference,
    )?;
    if !constant_time_eq(
        confirmation.preview_token.as_bytes(),
        preview.preview_token.as_bytes(),
    ) {
        return Err(CommandError::StaleActionReference);
    }
    if !action.confirmation_required || action.apply_method.is_none() {
        return Err(CommandError::MismatchedActionReference(
            "apply confirmation referenced a read-only action".to_string(),
        ));
    }
    Ok(())
}

pub fn action_descriptor(
    kind: ActionKind,
    intent: ActionIntent,
    target: ActionTargetRef,
    project_id: Option<String>,
    impacts: Vec<ActionImpact>,
    preview_method: &str,
    apply_method: Option<&str>,
    source_revision: String,
    confirmation_required: bool,
    network: ActionNetworkPosture,
    readback: Vec<ActionReadbackDomain>,
    evidence_refs: Vec<String>,
) -> Result<ActionDescriptor, CommandError> {
    validate_action_intent(kind, intent)?;
    validate_action_method_ownership(kind, preview_method, apply_method)?;
    let id = deterministic_action_id(kind, intent, &target, project_id.as_deref())?;
    let descriptor = ActionDescriptor {
        id,
        kind,
        intent,
        target,
        project_id,
        impacts,
        preview_method: preview_method.to_string(),
        apply_method: apply_method.map(str::to_string),
        source_revision,
        confirmation_required,
        network,
        readback,
        evidence_refs,
    };
    descriptor
        .validate()
        .map_err(|error| CommandError::MismatchedActionReference(error.to_string()))?;
    Ok(descriptor)
}

pub fn canonical_project_id(project_root: Option<&std::path::Path>) -> Option<String> {
    project_root.map(|root| project_id_from_root(&root.to_string_lossy()))
}

pub fn canonical_readback_domains(
    domains: impl IntoIterator<Item = ActionReadbackDomain>,
) -> Vec<ActionReadbackDomain> {
    let domains = domains.into_iter().collect::<BTreeSet<_>>();
    domains.into_iter().collect()
}

fn hash_action_descriptor(hasher: &mut Sha256, action: &ActionDescriptor) {
    hash_field(hasher, "action_id", &action.id);
    hash_field(hasher, "action_kind", action_kind_wire_value(action.kind));
    hash_field(
        hasher,
        "action_intent",
        action_intent_wire_value(action.intent),
    );
    hash_target(hasher, &action.target);
    hash_optional_field(hasher, "project_id", action.project_id.as_deref());
    hash_field(hasher, "preview_method", &action.preview_method);
    hash_optional_field(hasher, "apply_method", action.apply_method.as_deref());
    hash_field(hasher, "source_revision", &action.source_revision);
    hash_field(
        hasher,
        "confirmation_required",
        if action.confirmation_required {
            "true"
        } else {
            "false"
        },
    );
    hash_field(hasher, "network", action_network_wire_value(action.network));
    let impacts = action
        .impacts
        .iter()
        .map(|impact| action_impact_wire_value(*impact))
        .collect::<BTreeSet<_>>();
    for impact in impacts {
        hash_field(hasher, "impact", impact);
    }
    let readback = action
        .readback
        .iter()
        .map(|domain| action_readback_wire_value(*domain))
        .collect::<BTreeSet<_>>();
    for domain in readback {
        hash_field(hasher, "readback", domain);
    }
    for evidence_ref in action.evidence_refs.iter().collect::<BTreeSet<_>>() {
        hash_field(hasher, "evidence_ref", evidence_ref);
    }
}

fn hash_target(hasher: &mut Sha256, target: &ActionTargetRef) {
    hash_field(
        hasher,
        "target_kind",
        &serde_json::to_string(&target.kind).expect("action target kind serializes"),
    );
    hash_field(hasher, "target_id", &target.id);
    hash_optional_field(hasher, "target_agent", target.agent.map(AgentId::as_str));
    hash_optional_field(hasher, "target_scope", target.scope.map(Scope::as_str));
}

fn hash_optional_field(hasher: &mut Sha256, label: &str, value: Option<&str>) {
    match value {
        Some(value) => {
            hash_field(hasher, &format!("{label}:presence"), "some");
            hash_field(hasher, label, value);
        }
        None => hash_field(hasher, &format!("{label}:presence"), "none"),
    }
}

fn hash_field(hasher: &mut Sha256, label: &str, value: &str) {
    hasher.update((label.len() as u64).to_be_bytes());
    hasher.update(label.as_bytes());
    hasher.update((value.len() as u64).to_be_bytes());
    hasher.update(value.as_bytes());
}

fn action_preview_hmac(payload: &[u8]) -> Result<String, CommandError> {
    let secret = ACTION_PREVIEW_SECRET.get_or_init(action_preview_secret_from_environment);
    let secret = secret
        .as_ref()
        .map_err(|error| CommandError::ActionTokenUnavailable(error.clone()))?;
    let mut key = [0_u8; HMAC_BLOCK_SIZE];
    key[..secret.len()].copy_from_slice(secret);
    let mut inner_pad = [0x36_u8; HMAC_BLOCK_SIZE];
    let mut outer_pad = [0x5c_u8; HMAC_BLOCK_SIZE];
    for index in 0..HMAC_BLOCK_SIZE {
        inner_pad[index] ^= key[index];
        outer_pad[index] ^= key[index];
    }
    let mut inner = Sha256::new();
    inner.update(inner_pad);
    inner.update(payload);
    let inner_digest = inner.finalize();
    let mut outer = Sha256::new();
    outer.update(outer_pad);
    outer.update(inner_digest);
    Ok(format!("{:x}", outer.finalize()))
}

fn action_preview_secret_from_environment() -> Result<[u8; ACTION_PREVIEW_SECRET_SIZE], String> {
    let value = std::env::var(ACTION_PREVIEW_SECRET_ENV);
    // The sidecar may subsequently invoke an external manager. Never allow the
    // signing secret to remain in its inherited process environment.
    std::env::remove_var(ACTION_PREVIEW_SECRET_ENV);
    match value {
        Ok(value) => decode_action_preview_secret(&value),
        Err(std::env::VarError::NotUnicode(_)) => {
            Err("action preview secret environment value is not valid Unicode".to_string())
        }
        Err(std::env::VarError::NotPresent) => Err(
            "action preview secret is unavailable; the native client must inject its in-memory session secret"
                .to_string(),
        ),
    }
}

fn decode_action_preview_secret(value: &str) -> Result<[u8; ACTION_PREVIEW_SECRET_SIZE], String> {
    if value.len() != ACTION_PREVIEW_SECRET_SIZE * 2
        || !value.bytes().all(|byte| byte.is_ascii_hexdigit())
    {
        return Err("action preview secret must be exactly 64 hexadecimal characters".to_string());
    }
    let mut secret = [0_u8; ACTION_PREVIEW_SECRET_SIZE];
    for (index, pair) in value.as_bytes().chunks_exact(2).enumerate() {
        let high = decode_hex_nibble(pair[0])
            .ok_or_else(|| "action preview secret contains invalid hexadecimal".to_string())?;
        let low = decode_hex_nibble(pair[1])
            .ok_or_else(|| "action preview secret contains invalid hexadecimal".to_string())?;
        secret[index] = (high << 4) | low;
    }
    Ok(secret)
}

fn decode_hex_nibble(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

fn constant_time_eq(left: &[u8], right: &[u8]) -> bool {
    let mut difference = left.len() ^ right.len();
    for index in 0..left.len().max(right.len()) {
        let left_byte = left.get(index).copied().unwrap_or_default();
        let right_byte = right.get(index).copied().unwrap_or_default();
        difference |= usize::from(left_byte ^ right_byte);
    }
    difference == 0
}

fn ensure_nonempty(value: &str, label: &str) -> Result<(), CommandError> {
    if value.trim().is_empty() {
        return Err(CommandError::MismatchedActionReference(format!(
            "{label} is empty"
        )));
    }
    Ok(())
}

fn action_impact_wire_value(impact: ActionImpact) -> &'static str {
    match impact {
        ActionImpact::ReadOnly => "read_only",
        ActionImpact::AppLocalData => "app_local_data",
        ActionImpact::AgentConfig => "agent_config",
        ActionImpact::SkillFiles => "skill_files",
        ActionImpact::ExternalManager => "external_manager",
        _ => "unknown",
    }
}

pub fn validate_action_method_ownership(
    kind: ActionKind,
    preview_method: &str,
    apply_method: Option<&str>,
) -> Result<(), CommandError> {
    let valid = match kind {
        ActionKind::RefreshEvidence => {
            matches!(
                (preview_method, apply_method),
                ("catalog.scanAll", Some("catalog.scanAll"))
                    | ("catalog.scanAgent", None)
                    | ("catalog.getSkill", None)
            )
        }
        ActionKind::ToggleSkill => {
            preview_method == "batch.previewSkillToggles"
                && apply_method == Some("batch.applySkillToggles")
        }
        ActionKind::InstallSkill => {
            preview_method == "skill.install" && apply_method == Some("skill.install")
        }
        ActionKind::ManagerInstall => {
            preview_method == "skillManager.previewInstall"
                && apply_method == Some("skillManager.applyInstall")
        }
        ActionKind::ManagerRemove => {
            preview_method == "skillManager.previewRemove"
                && apply_method == Some("skillManager.applyRemove")
        }
        ActionKind::ManagerUpdate => {
            preview_method == "skillManager.previewUpdate"
                && apply_method == Some("skillManager.applyUpdate")
        }
        ActionKind::ManagerLocalCreate => {
            preview_method == "skillManager.previewLocalCreate"
                && apply_method == Some("skillManager.applyLocalCreate")
        }
        ActionKind::ManagerLocalArchiveImport => {
            preview_method == "skillManager.previewLocalArchiveImport"
                && apply_method == Some("skillManager.applyLocalArchiveImport")
        }
        ActionKind::ManagerLocalArchiveUpdate => {
            preview_method == "skillManager.previewLocalArchiveUpdate"
                && apply_method == Some("skillManager.applyLocalArchiveUpdate")
        }
        ActionKind::ManagerLocalDelete => {
            preview_method == "skillManager.deleteLocal"
                && apply_method == Some("skillManager.deleteLocal")
        }
        ActionKind::RollbackConfig => {
            preview_method == "snapshot.previewRollback"
                && apply_method == Some("snapshot.rollback")
        }
        ActionKind::SaveConfig => {
            preview_method == "config.readAgentConfig"
                && matches!(
                    apply_method,
                    Some("config.saveClaudeSettings") | Some("config.toggleSkill")
                )
        }
        ActionKind::ResumeSession => {
            preview_method == "session.previewResume" && apply_method.is_none()
        }
        ActionKind::TriageFinding => {
            preview_method == "catalog.listFindingTriage"
                && matches!(
                    apply_method,
                    Some("catalog.setFindingTriage") | Some("catalog.clearFindingTriage")
                )
        }
        ActionKind::TuneRule => {
            preview_method == "rules.listTuning"
                && matches!(
                    apply_method,
                    Some("rules.setSeverityOverride")
                        | Some("rules.clearSeverityOverride")
                        | Some("rules.setSuppression")
                        | Some("rules.clearSuppression")
                )
        }
        ActionKind::ProjectContext => {
            preview_method == "project.validateContext"
                && matches!(
                    apply_method,
                    Some("project.setContext")
                        | Some("project.clearContext")
                        | Some("project.removeRecentContext")
                        | Some("project.clearRecentContexts")
                )
        }
        ActionKind::ProviderProfile => {
            preview_method == "llm.listProviderProfiles"
                && matches!(
                    apply_method,
                    Some("llm.saveProviderProfile") | Some("llm.deleteProviderProfile")
                )
        }
        ActionKind::ProviderConnectionTest => {
            preview_method == "llm.testProviderConnection"
                && apply_method == Some("llm.testProviderConnection")
        }
        _ => false,
    };
    if valid {
        Ok(())
    } else {
        Err(CommandError::MismatchedActionReference(format!(
            "action kind {kind:?} does not own {preview_method} -> {}",
            apply_method.unwrap_or("<none>")
        )))
    }
}

pub fn validate_action_intent(kind: ActionKind, intent: ActionIntent) -> Result<(), CommandError> {
    let valid = matches!(
        (kind, intent),
        (
            ActionKind::RefreshEvidence,
            ActionIntent::RefreshEvidence | ActionIntent::InspectEvidence
        ) | (
            ActionKind::ToggleSkill,
            ActionIntent::EnableSkill | ActionIntent::DisableSkill
        ) | (ActionKind::InstallSkill, ActionIntent::InstallSkill)
            | (ActionKind::ManagerInstall, ActionIntent::ManagerInstall)
            | (ActionKind::ManagerRemove, ActionIntent::ManagerRemove)
            | (ActionKind::ManagerUpdate, ActionIntent::ManagerUpdate)
            | (
                ActionKind::ManagerLocalCreate,
                ActionIntent::ManagerLocalCreate
            )
            | (
                ActionKind::ManagerLocalArchiveImport,
                ActionIntent::ManagerLocalArchiveImport
            )
            | (
                ActionKind::ManagerLocalArchiveUpdate,
                ActionIntent::ManagerLocalArchiveUpdate
            )
            | (
                ActionKind::ManagerLocalDelete,
                ActionIntent::ManagerLocalDelete
            )
            | (ActionKind::RollbackConfig, ActionIntent::RollbackConfig)
            | (ActionKind::SaveConfig, ActionIntent::SaveConfig)
            | (ActionKind::ResumeSession, ActionIntent::ResumeSession)
            | (ActionKind::TriageFinding, ActionIntent::TriageFinding)
            | (ActionKind::TuneRule, ActionIntent::TuneRule)
            | (ActionKind::ProjectContext, ActionIntent::SetProjectContext)
            | (
                ActionKind::ProviderProfile,
                ActionIntent::SaveProviderProfile
            )
            | (
                ActionKind::ProviderConnectionTest,
                ActionIntent::TestProviderConnection
            )
    );
    if valid {
        Ok(())
    } else {
        Err(CommandError::MismatchedActionReference(format!(
            "action intent {intent:?} does not belong to {kind:?}"
        )))
    }
}

fn action_kind_wire_value(kind: ActionKind) -> &'static str {
    match kind {
        ActionKind::RefreshEvidence => "refresh_evidence",
        ActionKind::ToggleSkill => "toggle_skill",
        ActionKind::InstallSkill => "install_skill",
        ActionKind::ManagerInstall => "manager_install",
        ActionKind::ManagerRemove => "manager_remove",
        ActionKind::ManagerUpdate => "manager_update",
        ActionKind::ManagerLocalCreate => "manager_local_create",
        ActionKind::ManagerLocalArchiveImport => "manager_local_archive_import",
        ActionKind::ManagerLocalArchiveUpdate => "manager_local_archive_update",
        ActionKind::ManagerLocalDelete => "manager_local_delete",
        ActionKind::RollbackConfig => "rollback_config",
        ActionKind::SaveConfig => "save_config",
        ActionKind::ResumeSession => "resume_session",
        ActionKind::TriageFinding => "triage_finding",
        ActionKind::TuneRule => "tune_rule",
        ActionKind::ProjectContext => "project_context",
        ActionKind::ProviderProfile => "provider_profile",
        ActionKind::ProviderConnectionTest => "provider_connection_test",
        _ => "unknown",
    }
}

fn action_intent_wire_value(intent: ActionIntent) -> &'static str {
    match intent {
        ActionIntent::RefreshEvidence => "refresh_evidence",
        ActionIntent::InspectEvidence => "inspect_evidence",
        ActionIntent::EnableSkill => "enable_skill",
        ActionIntent::DisableSkill => "disable_skill",
        ActionIntent::InstallSkill => "install_skill",
        ActionIntent::ManagerInstall => "manager_install",
        ActionIntent::ManagerRemove => "manager_remove",
        ActionIntent::ManagerUpdate => "manager_update",
        ActionIntent::ManagerLocalCreate => "manager_local_create",
        ActionIntent::ManagerLocalArchiveImport => "manager_local_archive_import",
        ActionIntent::ManagerLocalArchiveUpdate => "manager_local_archive_update",
        ActionIntent::ManagerLocalDelete => "manager_local_delete",
        ActionIntent::RollbackConfig => "rollback_config",
        ActionIntent::SaveConfig => "save_config",
        ActionIntent::ResumeSession => "resume_session",
        ActionIntent::TriageFinding => "triage_finding",
        ActionIntent::TuneRule => "tune_rule",
        ActionIntent::SetProjectContext => "set_project_context",
        ActionIntent::SaveProviderProfile => "save_provider_profile",
        ActionIntent::TestProviderConnection => "test_provider_connection",
        _ => "unknown",
    }
}

fn action_network_wire_value(network: ActionNetworkPosture) -> &'static str {
    match network {
        ActionNetworkPosture::None => "none",
        ActionNetworkPosture::Conditional => "conditional",
        ActionNetworkPosture::Required => "required",
        _ => "unknown",
    }
}

fn action_readback_wire_value(domain: ActionReadbackDomain) -> &'static str {
    match domain {
        ActionReadbackDomain::ProjectContext => "project_context",
        ActionReadbackDomain::CatalogSkills => "catalog_skills",
        ActionReadbackDomain::SkillAggregates => "skill_aggregates",
        ActionReadbackDomain::SkillFiles => "skill_files",
        ActionReadbackDomain::AgentConfig => "agent_config",
        ActionReadbackDomain::ConfigSnapshots => "config_snapshots",
        ActionReadbackDomain::ManagerInventory => "manager_inventory",
        ActionReadbackDomain::SessionContinuation => "session_continuation",
        ActionReadbackDomain::BlockedAttemptAudit => "blocked_attempt_audit",
        _ => "unknown",
    }
}

fn precondition_kind_wire_value(kind: ActionPreconditionKind) -> &'static str {
    match kind {
        ActionPreconditionKind::CatalogRecord => "catalog_record",
        ActionPreconditionKind::AgentConfig => "agent_config",
        ActionPreconditionKind::SourceFile => "source_file",
        ActionPreconditionKind::TargetFile => "target_file",
        ActionPreconditionKind::ManagerInventory => "manager_inventory",
        ActionPreconditionKind::Archive => "archive",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use skills_copilot_core::{ActionTargetKind, AgentId};

    fn fixture_action(revision: &str, target_id: &str) -> ActionDescriptor {
        initialize_action_preview_secret_for_test([0xA5; ACTION_PREVIEW_SECRET_SIZE])
            .expect("initialize action preview test secret");
        action_descriptor(
            ActionKind::ToggleSkill,
            ActionIntent::DisableSkill,
            ActionTargetRef {
                kind: ActionTargetKind::Skill,
                id: target_id.to_string(),
                agent: Some(AgentId::Codex),
                scope: Some(Scope::AgentGlobal),
            },
            None,
            vec![ActionImpact::AgentConfig],
            "batch.previewSkillToggles",
            Some("batch.applySkillToggles"),
            revision.to_string(),
            true,
            ActionNetworkPosture::None,
            vec![
                ActionReadbackDomain::AgentConfig,
                ActionReadbackDomain::SkillAggregates,
            ],
            vec![format!("skill:{target_id}")],
        )
        .expect("fixture action")
    }

    fn config_precondition(revision: &str) -> ActionPrecondition {
        ActionPrecondition {
            kind: ActionPreconditionKind::AgentConfig,
            target_id: "$HOME/.config".to_string(),
            expected_revision: revision.to_string(),
        }
    }

    #[test]
    fn action_confirmation_rejects_unknown_stale_and_mismatched_references() {
        let action = fixture_action("sha256:current", "skill-1");
        let preview =
            action_preview_binding(action.clone(), vec![config_precondition("sha256:config")])
                .expect("preview");

        let unknown = ActionReference {
            action_id: "action:invented".to_string(),
            ..ActionReference::from(&action)
        };
        assert!(matches!(
            resolve_action_reference(std::slice::from_ref(&action), &unknown),
            Err(CommandError::UnknownActionReference(_))
        ));

        let stale = ActionReference {
            source_revision: "sha256:stale".to_string(),
            ..ActionReference::from(&action)
        };
        assert!(matches!(
            resolve_action_reference(std::slice::from_ref(&action), &stale),
            Err(CommandError::StaleActionReference)
        ));

        let other_target = fixture_action("sha256:current", "skill-2");
        let mismatch = ActionReference {
            action_id: action.id.clone(),
            ..ActionReference::from(&other_target)
        };
        assert!(matches!(
            resolve_action_reference(std::slice::from_ref(&action), &mismatch),
            Err(CommandError::MismatchedActionReference(_))
        ));

        let confirmation = ActionConfirmation {
            reference: ActionReference::from(&action),
            preview_token: preview.preview_token.clone(),
            confirmed: true,
        };
        assert!(ensure_action_confirmed(&preview, Some(&confirmation)).is_ok());
    }

    #[test]
    fn preview_tokens_bind_project_target_revision_impact_network_and_readback() {
        let action = fixture_action("sha256:current", "skill-1");
        let preview =
            action_preview_binding(action.clone(), vec![config_precondition("sha256:config")])
                .expect("preview");

        let changed_revision = fixture_action("sha256:changed", "skill-1");
        let changed_revision =
            action_preview_binding(changed_revision, vec![config_precondition("sha256:config")])
                .expect("changed revision");
        assert_ne!(preview.preview_token, changed_revision.preview_token);

        let changed_target = fixture_action("sha256:current", "skill-2");
        let changed_target =
            action_preview_binding(changed_target, vec![config_precondition("sha256:config")])
                .expect("changed target");
        assert_ne!(preview.preview_token, changed_target.preview_token);

        let changed_config =
            action_preview_binding(action, vec![config_precondition("sha256:changed")])
                .expect("changed config");
        assert_ne!(preview.preview_token, changed_config.preview_token);
    }

    #[test]
    fn preview_token_is_process_secret_signed_not_a_public_digest() {
        let action = fixture_action("sha256:current", "skill-1");
        let preconditions = vec![config_precondition("sha256:config")];
        let preview =
            action_preview_binding(action.clone(), preconditions.clone()).expect("preview");

        let mut public_digest = Sha256::new();
        hash_field(&mut public_digest, "domain", ACTION_PREVIEW_TOKEN_DOMAIN);
        hash_action_descriptor(&mut public_digest, &action);
        for precondition in preconditions {
            hash_field(
                &mut public_digest,
                "precondition_kind",
                precondition_kind_wire_value(precondition.kind),
            );
            hash_field(
                &mut public_digest,
                "precondition_target",
                &precondition.target_id,
            );
            hash_field(
                &mut public_digest,
                "precondition_revision",
                &precondition.expected_revision,
            );
        }
        let publicly_reproducible = format!(
            "action-preview:v1:hmac-sha256:{:x}",
            public_digest.finalize()
        );

        assert_ne!(preview.preview_token, publicly_reproducible);
        assert!(preview
            .preview_token
            .starts_with("action-preview:v1:hmac-sha256:"));
    }

    #[test]
    fn stable_action_id_binds_kind_but_not_source_revision() {
        let current = fixture_action("sha256:current", "skill-1");
        let changed_revision = fixture_action("sha256:changed", "skill-1");
        assert_eq!(current.id, changed_revision.id);

        let other_kind = deterministic_action_id(
            ActionKind::InstallSkill,
            ActionIntent::InstallSkill,
            &current.target,
            current.project_id.as_deref(),
        )
        .expect("other kind id");
        assert_ne!(current.id, other_kind);
    }

    #[test]
    fn readback_rejects_observations_outside_the_declared_domains() {
        let action = fixture_action("sha256:current", "skill-1");
        let result = ActionReadbackRecord::verified(
            &action,
            vec![
                ActionReadbackObservation {
                    domain: ActionReadbackDomain::AgentConfig,
                    target_id: "$HOME/.codex/config.toml".to_string(),
                    revision: "sha256:config".to_string(),
                },
                ActionReadbackObservation {
                    domain: ActionReadbackDomain::SkillAggregates,
                    target_id: "skill-1".to_string(),
                    revision: "sha256:skill".to_string(),
                },
                ActionReadbackObservation {
                    domain: ActionReadbackDomain::ManagerInventory,
                    target_id: "unexpected-ledger".to_string(),
                    revision: "sha256:ledger".to_string(),
                },
            ],
        );

        assert!(matches!(
            result,
            Err(CommandError::MismatchedActionReference(_))
        ));
    }
}
