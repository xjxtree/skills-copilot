import Foundation

enum AppLanguage: String, CaseIterable, Identifiable {
    case english = "en"
    case simplifiedChinese = "zh-Hans"

    static let storageKey = "app.language"
    static let defaultLanguage = AppLanguage.english

    var id: String { rawValue }

    var localeIdentifier: String { rawValue }

    var title: String {
        switch self {
        case .english:
            return UIStrings.languageEnglish
        case .simplifiedChinese:
            return UIStrings.languageSimplifiedChinese
        }
    }

    static func fromStorage(_ rawValue: String?) -> AppLanguage {
        guard let rawValue, let language = AppLanguage(rawValue: rawValue) else {
            return defaultLanguage
        }
        return language
    }
}

enum UIStrings {
    static var appTitle: String { text("app.title", "Agent Copilot") }
    static var appWindowTitle: String { text("app.windowTitle", "Agent Copilot") }
    static var searchPrompt: String { text("search.prompt", "Search") }
    static var scan: String { text("action.scan", "Scan") }
    static var reload: String { text("action.reload", "Reload") }
    static var save: String { text("action.save", "Save") }
    static var done: String { text("action.done", "Done") }
    static var cancel: String { text("action.cancel", "Cancel") }
    static var enable: String { text("action.enable", "Enable") }
    static var disable: String { text("action.disable", "Disable") }
    static var preview: String { text("action.preview", "Preview") }
    static var previewGate: String { text("action.previewGate", "Preview Gate") }
    static var executionBlocked: String { text("action.executionBlocked", "Execution Blocked") }
    static var rollback: String { text("action.rollback", "Rollback") }
    static var installToAgent: String { text("action.installToAgent", "Install to Agent...") }
    static var confirmInstall: String { text("action.confirmInstall", "Confirm Install") }
    static var llmAnalyze: String { text("llm.action.analyze", "Analyze") }
    static var llmRecommend: String { text("llm.action.recommend", "Recommend") }
    static var llmExplainConflict: String { text("llm.action.explainConflict", "Explain Same-agent Conflict") }
    static var llmDraftFrontmatter: String { text("llm.action.draftFrontmatter", "Draft Frontmatter") }
    static var chooseProject: String { text("action.chooseProject", "Choose Project") }
    static var clearProject: String { text("action.clearProject", "Clear Project") }
    static var revealInFinder: String { text("action.revealInFinder", "Reveal in Finder") }
    static var openFile: String { text("action.openFile", "Open") }
    static var copyPath: String { text("action.copyPath", "Copy Path") }
    static var skills: String { text("nav.skills", "Skills") }
    static var project: String { text("nav.project", "Project") }
    static var view: String { text("nav.view", "View") }
    static var agent: String { text("filter.agent", "Agent") }
    static var state: String { text("filter.state", "State") }
    static var sort: String { text("filter.sort", "Sort") }
    static var claudeCode: String { text("agent.claudeCode", "Claude Code") }
    static var codex: String { text("agent.codex", "Codex") }
    static var opencode: String { text("agent.opencode", "opencode") }
    static var pi: String { text("agent.pi", "Pi") }
    static var hermes: String { text("agent.hermes", "Hermes") }
    static var openclaw: String { text("agent.openclaw", "OpenClaw") }
    static var detailSection: String { text("detail.section", "Detail Section") }
    static var overview: String { text("detail.overview", "Overview") }
    static var findings: String { text("detail.findings", "Issues") }
    static var batchToggleTitle: String { text("batchToggle.title", "Safe Batch") }
    static var batchToggleOpen: String { text("batchToggle.open", "Batch") }
    static var batchToggleOpenHelp: String { text("batchToggle.open.help", "Choose visible skills, preview the safe enable/disable plan, then apply confirmed writable changes.") }
    static var batchToggleSheetTitle: String { text("batchToggle.sheet.title", "Batch Skill Operations") }
    static var batchToggleSheetSubtitle: String { text("batchToggle.sheet.subtitle", "Select skills from the current sidebar result, then preview before applying enable or disable changes.") }
    static var batchToggleBoundary: String { text("batchToggle.boundary", "Preview-first enable/disable for visible skills only. Read-only adapters and unverified writable roots are skipped; no scripts, AI provider calls, credentials, skill-content writes, or public release actions are available.") }
    static var batchToggleTarget: String { text("batchToggle.target", "Batch target") }
    static var batchToggleSelectAll: String { text("batchToggle.selectAll", "Select All") }
    static var batchToggleClearSelection: String { text("batchToggle.clearSelection", "Clear") }
    static var batchToggleSelected: String { text("batchToggle.selected", "Selected") }
    static var batchToggleWritable: String { text("batchToggle.writable", "Writable") }
    static var batchToggleSkipped: String { text("batchToggle.skipped", "Skipped") }
    static var batchToggleApply: String { text("batchToggle.apply", "Apply") }
    static var batchTogglePreviewing: String { text("batchToggle.previewing", "Preparing batch preview...") }
    static var batchToggleSnapshotPlan: String { text("batchToggle.snapshotPlan", "Snapshot / rollback plan") }
    static var batchToggleSnapshotPlanDefault: String { text("batchToggle.snapshotPlan.default", "Service will create agent-config snapshots for writable adapter targets before applying, then use existing rollback support for those config files.") }
    static var batchToggleSnapshotPlanUnavailable: String { text("batchToggle.snapshotPlan.unavailable", "Service batch preview is unavailable, so apply is disabled. No files were written.") }
    static var batchToggleServicePreviewUnavailable: String { text("batchToggle.servicePreviewUnavailable", "Service batch preview method is unavailable. This is a local read-only eligibility estimate; apply is disabled until batch.applySkillToggles or batch.applyToggle is available.") }
    static var batchToggleApplyUnavailable: String { text("batchToggle.applyUnavailable", "Batch apply is unavailable until a service preview/apply pair confirms the snapshot plan.") }
    static var batchToggleNoWritableChanges: String { text("batchToggle.noWritableChanges", "No writable skill changes are available in this preview.") }
    static var batchToggleNoAffectedSkills: String { text("batchToggle.noAffectedSkills", "No writable affected skills in this preview.") }
    static var batchToggleNoSkippedSkills: String { text("batchToggle.noSkippedSkills", "No skipped skills in this preview.") }
    static var batchTogglePreviewChanged: String { text("batchToggle.previewChanged", "Batch preview changed before confirmation. Preview again before applying.") }
    static var batchToggleNoSelection: String { text("batchToggle.noSelection", "Select at least one visible skill to prepare a batch preview.") }
    static var noSkillsInCatalog: String { text("empty.noSkillsInCatalog", "No skills in catalog") }
    static var noSkillsMatchSearch: String { text("empty.noSkillsMatchSearch", "No skills match this search") }
    static var noProjectSelected: String { text("project.none", "No Project") }
    static var toolbarNoProjectSelected: String { text("toolbar.project.noneSelected", "No project selected") }
    static var projectChoosePrompt: String { text("project.choosePrompt", "Choose a project or OpenClaw workspace directory to scan project-scoped Claude, Codex, opencode, and workspace-scoped OpenClaw skills.") }
    static var projectSelectedSource: String { text("project.source.selected", "Selected project") }
    static var projectGlobalRootsOnly: String { text("project.source.globalOnly", "No project: global roots only") }
    static var recentProjects: String { text("project.recent", "Recent Projects") }
    static var noRecentProjects: String { text("project.noRecent", "No Recent Projects") }
    static var projectValidation: String { text("project.validation", "Project Validation") }
    static var noProjectSkillsMessage: String { text("empty.noProjectSkills.message", "No skills were found in global roots. Choose a project to include project-scoped skills, then scan.") }
    static func noAgentSkillsMessage(_ agent: String) -> String {
        format("empty.noAgentSkills.message", "No %@ skills found in the current roots. Switch agents or choose a project, then scan again.", agent)
    }
    static var noCodexProjectMessage: String { text("empty.noCodexProject.message", "No Codex skills match the current global roots. Choose a project to include project-scoped Codex skills.") }
    static var noCodexSkillsMessage: String { text("empty.noCodexSkills.message", "No Codex skills match the current search or filters.") }
    static var noOpenClawWorkspaceSkillsMessage: String { text("empty.noOpenClawWorkspace.message", "No OpenClaw workspace skills match this view. OpenClaw only scans confirmed workspace skills and workspace .agents/skills roots; generic repo roots are skipped rather than treated as missing skills.") }
    static var adapterCapabilities: String { text("sidebar.adapterCapabilities", "Adapter Capabilities") }
    static var adapterScan: String { text("adapter.capability.scan", "Scan") }
    static var adapterToggle: String { text("adapter.capability.toggle", "Toggle") }
    static var adapterInstall: String { text("adapter.capability.install", "Install") }
    static var loading: String { text("state.loading", "Loading...") }
    static var startupPreparingLoading: String { text("startup.preparing", "Preparing startup...") }
    static var startupCatalogLoading: String { text("startup.catalog", "Loading catalog data...") }
    static var startupAnalysisLoading: String { text("startup.analysis", "Loading analysis data...") }
    static var startupSessionsLoading: String { text("startup.sessions", "Loading session data...") }
    static var startupConfigLoading: String { text("startup.config", "Loading config data...") }
    static var startupDetailLoading: String { text("startup.detail", "Loading detail data...") }
    static var startupReadyLoading: String { text("startup.ready", "Loading app...") }
    static var stateEnabled: String { text("state.enabled", "Enabled") }
    static var stateDisabled: String { text("state.disabled", "Disabled") }
    static var stateBroken: String { text("state.broken", "Broken") }
    static var stateMissing: String { text("state.missing", "Missing") }
    static var stateShadowed: String { text("state.shadowed", "Shadowed") }
    static var stateUnknown: String { text("state.unknown", "Unknown") }
    static var retryRefresh: String { text("action.retryRefresh", "Retry Refresh") }
    static var refreshLog: String { text("refresh.log", "Refresh Log") }
    static var refreshIdle: String { text("refresh.idle", "Ready to refresh") }
    static var refreshReloading: String { text("refresh.reloading", "Reloading catalog collections...") }
    static var refreshScanning: String { text("refresh.scanning", "Scanning skills across supported adapters and refreshing catalog...") }
    static var refreshWatcherManual: String { text("refresh.watcherManual", "Automatic watcher events are not active in this native sidecar yet. Use Reload or Scan to refresh.") }
    static var safetyFlags: String { text("safety.flags", "Safety flags") }
    static var noSafetyFlags: String { text("safety.flags.empty", "No safety flags returned.") }
    static var safetyReadOnlyClear: String { text("safety.readOnly.clear", "Read-only flags clear") }
    static var safetyReadOnlyWarning: String { text("safety.readOnly.warning", "Safety flags need review") }
    static var safetyProviderNotSent: String { text("safety.providerNotSent", "Provider not sent") }
    static var safetyWritesBlocked: String { text("safety.writesBlocked", "Writes blocked") }
    static var safetyScriptsBlocked: String { text("safety.scriptsBlocked", "Scripts blocked") }
    static var safetyMutationsBlocked: String { text("safety.mutationsBlocked", "Config/triage mutations blocked") }
    static var safetyCredentialsBlocked: String { text("safety.credentialsBlocked", "Credentials blocked") }
    static var safetyRawTraceStored: String { text("safety.rawTraceStored", "Raw trace stored") }
    static var safetyCloudSync: String { text("safety.cloudSync", "Cloud sync") }
    static var safetyTelemetry: String { text("safety.telemetry", "Telemetry") }
    static var catalogNotLoaded: String { text("state.catalogNotLoaded", "Catalog not loaded") }
    static var noSkillSelected: String { text("empty.noSkillSelected", "No Skill Selected") }
    static var noSkillSelectedMessage: String { text("empty.noSkillSelected.message", "Reload the catalog or select a skill from the sidebar.") }
    static var noSessionSelected: String { text("empty.noSessionSelected", "No Session Selected") }
    static var noSessionSelectedMessage: String { text("empty.noSessionSelected.message", "Refresh sessions or choose a session from the Sessions list.") }
    static func localSessionNoMatchesMessage(totalCount: Int) -> String {
        format("sidebar.sessions.noMatchesWithCount", "No sessions match the current filters. %d local sessions are loaded; clear search or change scope to show them.", totalCount)
    }
    static var noConfigSelected: String { text("empty.noConfigSelected", "No Config History Selected") }
    static var noConfigSelectedMessage: String { text("empty.noConfigSelected.message", "Select Config in the primary sidebar, then choose a config history item to inspect.") }
    static var noFindings: String { text("empty.noFindings", "No Issues") }
    static var noFindingsMessage: String { text("empty.noFindings.message", "No rule issues are associated with this skill.") }
    static var noMatchingFindings: String { text("empty.noMatchingFindings", "No Matching Issues") }
    static var noMatchingFindingsMessage: String { text("empty.noMatchingFindings.message", "Adjust the severity or rule filter to show issues.") }
    static var noConflicts: String { text("empty.noConflicts", "No Conflicts") }
    static var noConflictsMessage: String { text("empty.noConflicts.message", "No same-agent conflict currently references this skill in the current agent. Cross-agent duplicates are not shown as conflicts.") }
    static var noSnapshots: String { text("empty.noSnapshots", "No Agent Config History") }
    static var noSnapshotsMessage: String { text("empty.noSnapshots.message", "No agent config snapshots have been recorded for this agent yet.") }
    static var snapshotPreview: String { text("snapshot.preview", "Agent Config Preview") }
    static var rollbackSnapshotQuestion: String { text("snapshot.rollback.question", "Rollback Agent Config?") }
    static var current: String { text("snapshot.current", "Current Agent Config") }
    static var snapshot: String { text("snapshot.snapshot", "Snapshot Agent Config") }
    static var agentConfigHistory: String { text("sidebar.agentConfigHistory", "Agent Config History") }
    static var agentConfigHistorySummary: String { text("sidebar.agentConfigHistory.summary", "Preview or roll back saved configuration snapshots for the selected agent.") }
    static var agentConfigTimeline: String { text("sidebar.agentConfigTimeline", "Agent Config Timeline") }
    static var agentConfigTimelineBoundary: String { text("sidebar.agentConfigTimeline.boundary", "Config-level only: these rollback points capture agent configuration files, not SKILL.md content, and they do not mean every skill has its own snapshot.") }
    static var agentConfigTimelineSelectAgent: String { text("sidebar.agentConfigTimeline.selectAgent", "Choose one agent to view its config timeline. All Agents never mixes rollback points.") }
    static var agentConfigTimelineDefaultAction: String { text("sidebar.agentConfigTimeline.defaultAction", "Config snapshot") }
    static var agentConfigTimelineStatus: String { text("sidebar.agentConfigTimeline.status", "Rollback point") }
    static var previewDiff: String { text("action.previewDiff", "Preview diff") }
    static var recentActivity: String { text("detail.recentActivity", "Recent Activity") }
    static var noRecentActivity: String { text("detail.recentActivity.empty", "No enable or disable activity has been recorded for this skill yet.") }
    static var loadingRecentActivity: String { text("detail.recentActivity.loading", "Loading activity...") }
    static var activityPayload: String { text("detail.activity.payload", "Payload") }
    static var emptyPlaceholder: String { text("value.empty", "<empty>") }
    static var definition: String { text("metadata.definition", "Definition") }
    static var catalogID: String { text("metadata.catalogId", "Catalog ID") }
    static var source: String { text("metadata.source", "Source") }
    static var provenanceRoot: String { text("metadata.provenanceRoot", "Root") }
    static var provenanceKind: String { text("metadata.provenanceKind", "Kind") }
    static var provenanceNativeKind: String { text("metadata.provenance.kind.native", "Native") }
    static var provenanceCompatibilityKind: String { text("metadata.provenance.kind.compatibility", "Compatibility") }
    static var provenanceConfiguredKind: String { text("metadata.provenance.kind.configured", "Configured") }
    static var provenanceInferredKind: String { text("metadata.provenance.kind.inferred", "Inferred") }
    static var provenanceToolGlobalKind: String { text("metadata.provenance.kind.toolGlobal", "Tool-global") }
    static var provenanceReadOnlyKind: String { text("metadata.provenance.kind.readOnly", "Read-only") }
    static var provenanceExternalKind: String { text("metadata.provenance.kind.external", "External") }
    static var provenanceNativeRoot: String { text("metadata.provenance.root.native", "native root") }
    static var provenanceNativeOpencodeRoot: String { text("metadata.provenance.root.nativeOpencode", "Native opencode root") }
    static var provenanceClaudeCompatibilityRoot: String { text("metadata.provenance.root.claudeCompatibility", "Claude compatibility root") }
    static var provenanceAgentsCompatibilityRoot: String { text("metadata.provenance.root.agentsCompatibility", "Agents compatibility root") }
    static var provenanceConfiguredRoot: String { text("metadata.provenance.root.configured", "configured root") }
    static var provenanceToolGlobalRoot: String { text("metadata.provenance.root.toolGlobal", "Tool-global staging") }
    static var provenanceReadOnlyRoot: String { text("metadata.provenance.root.readOnly", "read-only root") }
    static var provenanceExternalRoot: String { text("metadata.provenance.root.external", "External root") }
    static var provenanceHermesHomeProfileRoot: String { text("metadata.provenance.root.hermesHomeProfile", "Hermes home/profile root") }
    static var provenanceHermesExternalRoot: String { text("metadata.provenance.root.hermesExternal", "Hermes explicit external root") }
    static var provenanceOpenClawWorkspaceRoot: String { text("metadata.provenance.root.openClawWorkspace", "OpenClaw workspace root") }
    static var provenanceOpenClawReadOnlyRoot: String { text("metadata.provenance.root.openClawReadOnly", "OpenClaw read-only root") }
    static var provenanceUnclassifiedRoot: String { text("metadata.provenance.root.unclassified", "Unclassified root") }
    static var fingerprint: String { text("metadata.fingerprint", "Fingerprint") }
    static var description: String { text("metadata.description", "Description") }
    static var noDescription: String { text("metadata.noDescription", "No description") }
    static var frontmatter: String { text("metadata.frontmatter", "Frontmatter") }
    static var body: String { text("metadata.body", "Body") }
    static var permissions: String { text("metadata.permissions", "Permissions") }
    static var winner: String { text("metadata.winner", "Winner") }
    static var none: String { text("value.none", "None") }
    static var findingSeverityFilter: String { text("findings.filter.severity", "Severity") }
    static var findingRuleFilter: String { text("findings.filter.rule", "Rule ID") }
    static var findingTriageFilter: String { text("findings.filter.triage", "Triage") }
    static var allSeverities: String { text("findings.filter.allSeverities", "All Severities") }
    static var allRuleIDs: String { text("findings.filter.allRules", "All Rule IDs") }
    static var findingTriageOpen: String { text("findings.triage.open", "Unmarked") }
    static var findingTriageReviewed: String { text("findings.triage.reviewed", "Reviewed") }
    static var findingTriageIgnored: String { text("findings.triage.ignored", "Ignored") }
    static var findingTriageNeedsFollowUp: String { text("findings.triage.needsFollowUp", "Needs follow-up") }
    static var findingTriageFilterActive: String { text("findings.triage.filter.active", "Unmarked + follow-up") }
    static var findingTriageFilterAll: String { text("findings.triage.filter.all", "All triage") }
    static var findingTriageNoticeTitle: String { text("findings.triage.notice.title", "Local finding triage") }
    static var findingTriageNoticeBody: String { text("findings.triage.notice.body", "Issue labels are stored only in Agent Copilot app data. They do not write agent config, skill content, toggle snapshots, scripts, or AI output. If an issue changes after rescan, its local label is cleared and it returns to Unmarked.") }
    static var findingTriageActionReviewed: String { text("findings.triage.action.reviewed", "Mark reviewed") }
    static var findingTriageActionIgnored: String { text("findings.triage.action.ignored", "Ignore") }
    static var findingTriageActionFollowUp: String { text("findings.triage.action.followUp", "Needs follow-up") }
    static var findingTriageActionReopen: String { text("findings.triage.action.reopen", "Clear label") }
    static var ruleTuningTitle: String { text("rules.tuning.title", "Rule Tuning / Suppression") }
    static var ruleTuningBoundary: String { text("rules.tuning.boundary", "App-local review state only. These controls never edit skill files, write agent config, create snapshots, execute scripts, call an AI provider, or store credentials.") }
    static var ruleTuningEffectiveState: String { text("rules.tuning.effectiveState", "Effective rule state") }
    static var ruleTuningSeverityOverride: String { text("rules.tuning.severityOverride", "Severity override") }
    static var ruleTuningClearSeverity: String { text("rules.tuning.clearSeverity", "Clear override") }
    static var ruleTuningSuppressGroup: String { text("rules.tuning.suppressGroup", "Suppress group") }
    static var ruleTuningUnsuppressGroup: String { text("rules.tuning.unsuppressGroup", "Unsuppress group") }
    static var ruleTuningSuppressRule: String { text("rules.tuning.suppressRule", "Suppress rule") }
    static var ruleTuningUnsuppressRule: String { text("rules.tuning.unsuppressRule", "Unsuppress rule") }
    static var ruleTuningSuppressed: String { text("rules.tuning.suppressed", "Suppressed locally") }
    static var ruleTuningRuleWide: String { text("rules.tuning.ruleWide", "Rule-wide") }
    static var ruleTuningFindingGroup: String { text("rules.tuning.findingGroup", "Issue") }
    static var ruleTuningNoOverride: String { text("rules.tuning.noOverride", "No local override") }
    static var findingExplanation: String { text("findings.explanation", "Why this appears") }
    static var findingRuleID: String { text("findings.ruleId", "Rule ID") }
    static var findingRuleSource: String { text("findings.ruleSource", "Rule source") }
    static var findingCatalogTarget: String { text("findings.catalogTarget", "Catalog target") }
    static var findingTrigger: String { text("findings.trigger", "Trigger") }
    static var findingImpact: String { text("findings.impact", "Impact") }
    static var findingRiskRelated: String { text("findings.riskRelated", "Risk-related") }
    static var findingRiskRelatedHelp: String { text("findings.riskRelated.help", "This rule is part of the permission, script, dependency, or tool-risk subset.") }
    static var findingsCompactNotice: String { text("findings.notice.compact", "Local scan issues are read-only reminders. They do not write agent config or skill files.") }
    static var findingsSummaryOverview: String { text("findings.summary.overview", "Issue overview") }
    static func findingsIssueSummary(_ visible: Int, _ total: Int) -> String {
        format("findings.summary.issueValue", "%d / %d issues", visible, total)
    }
    static func findingsImpactedSummary(_ count: Int) -> String {
        format("findings.summary.impactedValue", "%d affected instances", count)
    }
    static func findingsScanEntrySummary(_ count: Int) -> String {
        format("findings.summary.scanEntryValue", "%d scan entries", count)
    }
    static var findingRemediation: String { text("findings.remediation", "Suggested remediation") }
    static var currentAgentConflictsOnly: String { text("conflicts.currentAgentOnly", "Current agent only. Cross-agent duplicates are omitted from conflicts.") }
    static var findingSourceFrontmatter: String { text("findings.source.frontmatter", "Frontmatter validation") }
    static var findingSourcePermission: String { text("findings.source.permission", "Permission analysis") }
    static var findingSourceScript: String { text("findings.source.script", "Script safety analysis") }
    static var findingSourceDependency: String { text("findings.source.dependency", "Dependency analysis") }
    static var findingSourcePath: String { text("findings.source.path", "Catalog path check") }
    static var findingSourceFingerprint: String { text("findings.source.fingerprint", "Catalog fingerprint check") }
    static var findingSourceCatalog: String { text("findings.source.catalog", "Catalog rule") }
    static var findingNoCatalogTarget: String { text("findings.catalogTarget.none", "No definition or instance ID reported") }
    static var remediationFrontmatterRequired: String { text("findings.remediation.frontmatterRequired", "Add the required frontmatter fields in SKILL.md, then rescan.") }
    static var remediationToolsNotEmpty: String { text("findings.remediation.toolsNotEmpty", "Declare the allowed tools the skill needs, or remove tool-dependent instructions.") }
    static var remediationPathExists: String { text("findings.remediation.pathExists", "Restore the source file or remove the stale catalog entry, then scan again.") }
    static var remediationFingerprintChanged: String { text("findings.remediation.fingerprintChanged", "Review the changed skill content and rescan once the catalog should trust the new fingerprint.") }
    static var remediationNetworkDeclared: String { text("findings.remediation.networkDeclared", "Declare the intended network access explicitly, or keep it undeclared only if the skill does not use network access.") }
    static var remediationExecNeedsHuman: String { text("findings.remediation.execNeedsHuman", "Require human confirmation for execution-capable behavior, or remove the execution request.") }
    static var remediationDependencyUnknown: String { text("findings.remediation.dependencyUnknown", "Replace or document the unknown dependency, then rescan.") }
    static var instances: String { text("metadata.instances", "Instances") }
    static var target: String { text("metadata.target", "Target") }
    static var scope: String { text("metadata.scope", "Scope") }
    static var access: String { text("metadata.access", "Access") }
    static var permissionTools: String { text("permissions.tools", "Tools") }
    static var permissionFiles: String { text("permissions.files", "Files") }
    static var permissionNetwork: String { text("permissions.network", "Network") }
    static var permissionExec: String { text("permissions.exec", "Execution") }
    static var permissionHumanReview: String { text("permissions.humanReview", "Human review") }
    static var permissionRaw: String { text("permissions.raw", "Raw permissions") }
    static var permissionUndeclared: String { text("permissions.undeclared", "Undeclared / unknown") }
    static var permissionNoneDeclared: String { text("permissions.noneDeclared", "None declared") }
    static var permissionUnknownPayload: String { text("permissions.unknownPayload", "Unknown payload") }
    static var permissionNetworkReadOnly: String { text("permissions.network.readOnly", "Read-only declared") }
    static var permissionNetworkFull: String { text("permissions.network.full", "Full declared") }
    static var permissionRequested: String { text("permissions.requested", "Requested") }
    static var permissionNotRequested: String { text("permissions.notRequested", "Not requested") }
    static var permissionRequired: String { text("permissions.required", "Required") }
    static var permissionNotDeclaredRequired: String { text("permissions.notDeclaredRequired", "Not declared as required") }
    static var permissionUndeclaredNote: String { text("permissions.undeclaredNote", "Permissions are undeclared or unavailable in the catalog payload; this is not a safe or unsafe verdict.") }
    static var permissionDeclarationNote: String { text("permissions.declarationNote", "These values are permission declarations from the catalog payload, not a safety verdict.") }
    static var service: String { text("settings.service", "Service") }
    static var settingsWindowTitle: String { text("settings.window.title", "Settings") }
    static var settingsSidebarSubtitle: String { text("settings.sidebar.subtitle", "Immediate app preferences") }
    static var settingsNavAppearanceSubtitle: String { text("settings.nav.appearance.subtitle", "Theme and privacy") }
    static var settingsNavLanguageSubtitle: String { text("settings.nav.language.subtitle", "Interface and privacy") }
    static var settingsNavProviderSubtitle: String { text("settings.nav.provider.subtitle", "Connection and Keychain") }
    static var settingsNavObservabilitySubtitle: String { text("settings.nav.observability.subtitle", "Usage and logs") }
    static var settingsNavServiceSubtitle: String { text("settings.nav.service.subtitle", "Local sidecar") }
    static var appearanceSettings: String { text("settings.appearance.title", "Appearance") }
    static var appearanceBoundary: String { text("settings.appearance.boundary", "Appearance preferences are stored locally in the app. They do not write agent config, skill files, provider settings, credentials, reports, or prompts.") }
    static var appearanceAppliesImmediately: String { text("settings.appearance.appliesImmediately", "The main window and Settings update immediately after selection.") }
    static var themeSettings: String { text("settings.theme.title", "Theme") }
    static var themeSelection: String { text("settings.theme.selection", "Theme") }
    static var themeFollowSystem: String { text("settings.theme.followSystem", "Follow System") }
    static var themeLight: String { text("settings.theme.light", "Light") }
    static var themeDark: String { text("settings.theme.dark", "Dark") }
    static var languageSettings: String { text("settings.language.title", "Language") }
    static var languageSelection: String { text("settings.language.selection", "App language") }
    static var languageEnglish: String { text("settings.language.english", "English") }
    static var languageSimplifiedChinese: String { text("settings.language.simplifiedChinese", "Simplified Chinese") }
    static var languageBoundary: String { text("settings.language.boundary", "Language is stored as an app-local preference. It does not write agent config, skill files, provider settings, credentials, reports, or prompts.") }
    static var languageAppliesImmediately: String { text("settings.language.appliesImmediately", "The main window and Settings update immediately after selection.") }
    static var privacyScreenshotMode: String { text("settings.privacy.screenshotMode", "Screenshot privacy mode") }
    static var privacyScreenshotBoundary: String { text("settings.privacy.screenshotBoundary", "When enabled, local paths shown in the native UI use screenshot-safe placeholders and long-path collapse by default. Reveal is explicit and local to the current view.") }
    static var privacyRevealPath: String { text("privacy.path.reveal", "Reveal") }
    static var privacyHidePath: String { text("privacy.path.hide", "Hide") }
    static var privacyScreenshotSafe: String { text("privacy.path.screenshotSafe", "Screenshot safe") }
    static var version: String { text("settings.version", "Version") }
    static var protocolLabel: String { text("settings.protocol", "Protocol") }
    static var catalog: String { text("settings.catalog", "Catalog") }
    static var userHome: String { text("settings.userHome", "User Home") }
    static var methods: String { text("settings.methods", "Methods") }
    static var unknown: String { text("value.unknown", "Unknown") }
    static var notLoaded: String { text("value.notLoaded", "Not loaded") }
    static var aiProviderSettings: String { text("settings.aiProvider.title", "AI Provider") }
    static var aiProviderBoundary: String { text("settings.aiProvider.boundary", "Configure a user-owned provider profile for explicit AI requests. No analysis runs in the background, Test Connection is manual, and provider output cannot write skills, agent config, snapshots, or scripts.") }
    static var aiProviderUnavailable: String { text("settings.aiProvider.unavailable", "AI provider settings are unavailable in this service build.") }
    static var aiProviderOpenAICompatible: String { text("settings.aiProvider.kind.openai", "OpenAI-compatible") }
    static var aiProviderClaudeCompatible: String { text("settings.aiProvider.kind.claude", "Claude-compatible") }
    static var aiProviderEndpoint: String { text("settings.aiProvider.endpoint", "Endpoint") }
    static var aiProviderEndpointPlaceholder: String { text("settings.aiProvider.endpoint.placeholder", "https://api.example.com/v1") }
    static var aiProviderModel: String { text("settings.aiProvider.model", "Model") }
    static var aiProviderModelPlaceholder: String { text("settings.aiProvider.model.placeholder", "model") }
    static var aiProviderAPIVersion: String { text("settings.aiProvider.apiVersion", "API version") }
    static var aiProviderOptionalPlaceholder: String { text("settings.aiProvider.optional.placeholder", "optional") }
    static var aiProviderAPIKey: String { text("settings.aiProvider.apiKey", "API key") }
    static var aiProviderAPIKeyPlaceholder: String { text("settings.aiProvider.apiKey.placeholder", "Leave blank to keep existing Keychain item") }
    static var aiProviderKeychainFirst: String { text("settings.aiProvider.keychainFirst", "API keys are sent only to the local service when the profile auto-saves or when Test Connection is confirmed. The service should store secrets in Keychain first; the native UI clears this field after each action and never displays saved keys.") }
    static var aiProviderBudget: String { text("settings.aiProvider.budget", "Budget") }
    static var aiProviderMonthlyBudget: String { text("settings.aiProvider.monthlyBudget", "Monthly budget") }
    static var aiProviderMonthlyBudgetPlaceholder: String { text("settings.aiProvider.monthlyBudget.placeholder", "5") }
    static var aiProviderMonthlyBudgetHelp: String { text("settings.aiProvider.monthlyBudget.help", "Maximum monthly provider spend in USD. Blank uses the service default; 0 disables provider requests.") }
    static var aiProviderTokenLimit: String { text("settings.aiProvider.tokenLimit", "Single-request token limit") }
    static var aiProviderTokenLimitPlaceholder: String { text("settings.aiProvider.tokenLimit.placeholder", "128000") }
    static var aiProviderTokenLimitHelp: String { text("settings.aiProvider.tokenLimit.help", "Maximum input and output tokens allowed for one provider request. Requests above this estimate are blocked before sending.") }
    static var aiProviderStorage: String { text("settings.aiProvider.storage", "Credential storage") }
    static var aiProviderConfigured: String { text("settings.aiProvider.configured", "Configured") }
    static var aiProviderUnconfigured: String { text("settings.aiProvider.unconfigured", "Unconfigured") }
    static var aiProviderDisabledReason: String { text("settings.aiProvider.disabledReason", "Disabled reason") }
    static var aiProviderSave: String { text("settings.aiProvider.save", "Save Provider") }
    static var aiProviderTest: String { text("settings.aiProvider.test", "Test Connection") }
    static var aiProviderSaveConfirmationTitle: String { text("settings.aiProvider.saveConfirmation.title", "Save provider settings?") }
    static var aiProviderSaveConfirmationMessage: String { text("settings.aiProvider.saveConfirmation.message", "This sends the provider profile and any API key draft to the local service so it can update the verified profile and Keychain-backed credential state.") }
    static var aiProviderTestConfirmationTitle: String { text("settings.aiProvider.testConfirmation.title", "Test provider connection?") }
    static var aiProviderTestConfirmationMessage: String { text("settings.aiProvider.testConfirmation.message", "This performs one manual provider connection test against the configured endpoint. The UI clears any API key draft after the request.") }
    static var aiProviderSaving: String { text("settings.aiProvider.saving", "Saving provider...") }
    static var aiProviderTesting: String { text("settings.aiProvider.testing", "Testing connection...") }
    static var aiProviderAutosavePending: String { text("settings.aiProvider.autosavePending", "Valid changes will be saved automatically.") }
    static var aiProviderSaved: String { text("settings.aiProvider.saved", "Provider settings saved. API key draft cleared.") }
    static var aiProviderTestResult: String { text("settings.aiProvider.testResult", "Test result") }
    static var aiProviderTestSucceeded: String { text("settings.aiProvider.testSucceeded", "Provider connection test succeeded.") }
    static var aiProviderTestFailed: String { text("settings.aiProvider.testFailed", "Provider connection test failed.") }
    static var aiProviderAuditMetadata: String { text("settings.aiProvider.audit", "Audit metadata") }
    static var aiProviderNoAudit: String { text("settings.aiProvider.noAudit", "No audit metadata returned.") }
    static var aiProviderAuditDuration: String { text("settings.aiProvider.audit.duration", "Duration") }
    static var aiProviderAuditRedaction: String { text("settings.aiProvider.audit.redaction", "Redaction") }
    static var aiProviderAuditPromptStored: String { text("settings.aiProvider.audit.promptStored", "Prompt stored") }
    static var aiProviderAuditResponseStored: String { text("settings.aiProvider.audit.responseStored", "Response stored") }
    static var aiProviderAuditApplied: String { text("settings.aiProvider.audit.applied", "Applied") }
    static var aiProviderAuditNotApplied: String { text("settings.aiProvider.audit.notApplied", "Not applied") }
    static var aiProviderAuditStored: String { text("settings.aiProvider.audit.stored", "Stored") }
    static var aiProviderAuditNotStored: String { text("settings.aiProvider.audit.notStored", "Not stored") }
    static var aiProviderAuditErrorCode: String { text("settings.aiProvider.audit.errorCode", "Error code") }
    static var aiProviderEndpointRequired: String { text("settings.aiProvider.validation.endpointRequired", "Endpoint is required.") }
    static var aiProviderEndpointInvalid: String { text("settings.aiProvider.validation.endpointInvalid", "Endpoint must include a URL scheme such as https://.") }
    static var aiProviderModelRequired: String { text("settings.aiProvider.validation.modelRequired", "Model is required.") }
    static var aiProviderBudgetInvalid: String { text("settings.aiProvider.validation.budgetInvalid", "Monthly budget must be a number.") }
    static var aiProviderTokenLimitInvalid: String { text("settings.aiProvider.validation.tokenLimitInvalid", "Single-request token limit must be a whole number.") }
    static var agentConfigSettings: String { text("settings.agentConfig.title", "Agent Config") }
    static var agentConfigSettingsSubtitle: String { text("settings.agentConfig.subtitle", "Review each agent's verified config write paths, current target, and rollback history.") }
    static var projectScan: String { text("settings.agentConfig.projectScan", "Project scan") }
    static var configToggle: String { text("settings.agentConfig.configToggle", "Skill toggles") }
    static var configSnapshot: String { text("settings.agentConfig.configSnapshot", "Snapshots") }
    static var writableConfig: String { text("settings.agentConfig.writableConfig", "Writable config") }
    static var agentConfigBlockedScope: String { text("settings.agentConfig.blockedScope", "Still blocked") }
    static var agentConfigSettingsHistory: String { text("settings.agentConfig.history", "Config History") }
    static var currentConfigFile: String { text("settings.agentConfig.currentFile", "Current Config File") }
    static var agentConfigSensitiveValuesHidden: String { text("settings.agentConfig.sensitiveValuesHidden", "Sensitive values hidden") }
    static var agentConfigSensitiveValuesVisible: String { text("settings.agentConfig.sensitiveValuesVisible", "Sensitive values visible") }
    static var agentConfigShowSensitive: String { text("settings.agentConfig.showSensitive", "Show & Edit") }
    static var agentConfigShowSensitiveValues: String { text("settings.agentConfig.showSensitiveValues", "Show Values") }
    static var agentConfigHideSensitive: String { text("settings.agentConfig.hideSensitive", "Hide") }
    static var configAutosavePending: String { text("settings.agentConfig.autosavePending", "Valid changes will be saved automatically.") }
    static var configAutosaveSaving: String { text("settings.agentConfig.autosaveSaving", "Saving settings...") }
    static var configConsistencyProtocolRequired: String { text("settings.agentConfig.protocolV2Required", "Config writes require service protocol v2. Reload after updating the service; settings and rollback remain read-only.") }
    static var configRevisionUnavailable: String { text("settings.agentConfig.revisionUnavailable", "This service did not provide a config revision. Reload with protocol v2; settings remain read-only.") }
    static var configConflict: String { text("settings.agentConfig.conflict", "Settings changed outside Agent Copilot. Your draft was kept; compare it with the latest config before saving again.") }
    static var rollbackPreviewAgain: String { text("snapshot.rollback.previewAgain", "The rollback preview is no longer current. Preview again before applying.") }
    static var rollbackBindingUnavailable: String { text("snapshot.rollback.bindingUnavailable", "This service did not provide a rollback preview token. Rollback remains read-only until you reconnect with protocol v2.") }
    static var agentConfigEditConfirmationTitle: String { text("settings.agentConfig.editConfirmation.title", "Show and edit raw config?") }
    static var agentConfigEditConfirmationMessage: String { text("settings.agentConfig.editConfirmation.message", "This reveals sensitive config values and enables raw editing. Valid changes auto-save through the verified snapshot flow.") }
    static var agentConfigSkillEnablement: String { text("settings.agentConfig.skillEnablement", "Skill enablement") }
    static var agentConfigDisabledSkillsTitle: String { text("settings.agentConfig.disabledSkills", "Disabled skills") }
    static var agentConfigDisabledSkillsEmpty: String { text("settings.agentConfig.disabledSkills.empty", "No config-disabled skills detected.") }
    static var agentConfigReadOnlyBoundary: String { text("settings.agentConfig.readOnlyBoundary", "Read-only preview only. This view does not write agent config, create snapshots, execute scripts, call providers, or save credentials.") }
    static var agentConfigNoReadableDocuments: String { text("settings.agentConfig.noReadableDocuments", "No readable config documents were reported for this agent.") }
    static var supported: String { text("value.supported", "Supported") }
    static var notSupported: String { text("value.notSupported", "Not supported") }
    static func agentConfigDisabledSkillsCount(_ count: Int) -> String {
        format("settings.agentConfig.disabledSkills.count", "%d disabled", count)
    }
    static func agentConfigDisabledSkillsMore(_ count: Int) -> String {
        format("settings.agentConfig.disabledSkills.more", "%d more", count)
    }
    static func agentConfigHistoryEmpty(_ agent: String) -> String {
        format("settings.agentConfig.historyEmpty", "No %@ config snapshots yet.", agent)
    }
    static func agentConfigRawEditorBoundary(_ agent: String) -> String {
        format("settings.agentConfig.rawEditorBoundary", "%@ config is managed only through verified skill toggle paths for now. Raw editing is intentionally limited to Claude settings.", agent)
    }
    static func agentConfigReadOnlyPreview(_ agent: String) -> String {
        format("settings.agentConfig.readOnlyPreview", "%@ current config is shown as a redacted, read-only preview.", agent)
    }
    static var claudeSettings: String { text("settings.claudeSettings", "Claude Settings") }
    static var existingFile: String { text("settings.existingFile", "Existing file") }
    static var willCreateFile: String { text("settings.willCreateFile", "Will create file") }
    static var settingsInvalidUTF8: String { text("settings.invalidUtf8", "Settings content is not valid UTF-8.") }
    static var formatJSON: String { text("action.formatJSON", "Format JSON") }
    static var jsonValidSettingsWrite: String { text("settings.jsonValid", "JSON is valid. Changes will auto-save through snapshot, atomic write, verification, and rescan.") }
    static var connectedProtocolNote: String { text("detail.protocolNote", "This native macOS shell is connected through the Rust service protocol. Scan, toggle, and agent config rollback actions use verified write paths with snapshots.") }
    static var loadingSkillDetail: String { text("detail.loading", "Loading skill detail...") }
    static var readOnlyPreview: String { text("detail.readOnlyPreview", "Read-only preview") }
    static var toolGlobalPreviewTitle: String { text("detail.toolGlobal.previewTitle", "Tool-global Preview") }
    static var toolGlobalPreviewNote: String { text("detail.toolGlobal.previewNote", "Tool-global skills are staged for review. They cannot be toggled here and must be copied into a specific agent after an explicit confirmation.") }
    static var toolGlobalTargetAgent: String { text("detail.toolGlobal.targetAgent", "Target Agent") }
    static var toolGlobalInstallPreviewTitle: String { text("detail.toolGlobal.installPreviewTitle", "Install Preview") }
    static var toolGlobalInstallReady: String { text("detail.toolGlobal.installReady", "Confirmed install writes through the target adapter verified path with snapshot and read-back verification.") }
    static var taskCockpitStageReadiness: String { text("taskCockpit.stage.readiness", "Readiness") }
    static var taskCockpitStageRouting: String { text("taskCockpit.stage.routing", "Routing") }
    static var taskCockpitStageAgentComparison: String { text("taskCockpit.stage.agentComparison", "Agent candidates") }
    static var taskCockpitAgentReadinessScore: String { text("taskCockpit.agent.readinessScore", "Readiness") }
    static var taskCockpitAgentRoutingScore: String { text("taskCockpit.agent.routingScore", "Routing") }
    static var taskCockpitAgentBestSkill: String { text("taskCockpit.agent.bestSkill", "Best skill") }
    static var taskCockpitAgentReasons: String { text("taskCockpit.agent.reasons", "Reasons") }
    static var taskCockpitAgentNoReasons: String { text("taskCockpit.agent.empty.reasons", "No reasons returned.") }
    static var taskCockpitEvidence: String { text("taskCockpit.evidence", "Evidence") }
    static var taskCockpitNoEvidence: String { text("taskCockpit.empty.evidence", "No evidence returned.") }
    static var taskCockpitSafetyFlags: String { text("taskCockpit.safetyFlags", "Safety flags") }
    static var providerObservabilityTitle: String { text("providerObservability.title", "Provider Observability") }
    static var providerObservabilityBoundary: String { text("providerObservability.boundary", "Loaded at startup from redacted app-local provider metadata; read-only and never sends provider requests.") }
    static var providerObservabilityAction: String { text("providerObservability.action.build", "Build Observability") }
    static var providerObservabilityUnavailable: String { text("providerObservability.unavailable", "Provider observability is unavailable in this service build.") }
    static var providerObservabilityNoResult: String { text("providerObservability.empty.result", "Provider observability will appear after startup loading completes.") }
    static var providerObservabilitySettingsMode: String { text("providerObservability.settings.mode", "Observability view") }
    static var providerObservabilityDashboard: String { text("providerObservability.settings.dashboard", "Dashboard") }
    static var providerObservabilityLogs: String { text("providerObservability.settings.logs", "Logs") }
    static var providerObservabilityIssuesOnly: String { text("providerObservability.settings.issuesOnly", "Issues only") }
    static var providerObservabilityNoFilteredCalls: String { text("providerObservability.empty.filteredCalls", "No provider logs match the current filters.") }
    static var providerObservabilityCalls: String { text("providerObservability.calls", "Calls") }
    static var providerObservabilitySuccessRate: String { text("providerObservability.successRate", "Success rate") }
    static var providerObservabilitySuccesses: String { text("providerObservability.successes", "Succeeded") }
    static var providerObservabilityFailures: String { text("providerObservability.failures", "Failed") }
    static var providerObservabilityBlocked: String { text("providerObservability.blocked", "Blocked") }
    static var providerObservabilityProviders: String { text("providerObservability.providers", "Providers") }
    static var providerObservabilityModels: String { text("providerObservability.models", "Models") }
    static var providerObservabilityDestinations: String { text("providerObservability.destinations", "Destinations") }
    static var providerObservabilityModelTaskHistory: String { text("providerObservability.modelTaskHistory", "Model-task history") }
    static var providerObservabilityNoModelTaskHistory: String { text("providerObservability.empty.modelTaskHistory", "No model-task history returned.") }
    static var providerObservabilityTaskKind: String { text("providerObservability.taskKind", "Task kind") }
    static var providerObservabilityMatchStatus: String { text("providerObservability.matchStatus", "Match status") }
    static var providerObservabilityConfidence: String { text("providerObservability.confidence", "Confidence") }
    static var providerObservabilitySourceKind: String { text("providerObservability.sourceKind", "Source kind") }
    static var providerObservabilityRedactionStatus: String { text("providerObservability.redactionStatus", "Redaction") }
    static var providerObservabilityRecentCalls: String { text("providerObservability.recentCalls", "Recent calls") }
    static var providerObservabilityStatusRows: String { text("providerObservability.statusRows", "Status rows") }
    static var providerObservabilityErrorRows: String { text("providerObservability.errorRows", "Errors") }
    static var providerObservabilityBudgetHints: String { text("providerObservability.budgetHints", "Budget hints") }
    static var providerObservabilityUsageHints: String { text("providerObservability.usageHints", "Usage hints") }
    static var providerObservabilityRetention: String { text("providerObservability.retention", "Retention / cleanup") }
    static var providerObservabilityNoCalls: String { text("providerObservability.empty.calls", "No recent redacted provider calls returned.") }
    static var providerObservabilityNoRows: String { text("providerObservability.empty.rows", "No rows returned.") }
    static var providerObservabilityMetadataRedacted: String { text("providerObservability.metadataRedacted", "Metadata redacted") }
    static var providerObservabilityAppLocalOnly: String { text("providerObservability.appLocalOnly", "App-local only") }
    static var providerObservabilityDuration: String { text("providerObservability.duration", "Duration") }
    static var providerObservabilityAverageDuration: String { text("providerObservability.averageDuration", "Average duration") }
    static var providerObservabilityEstimatedTokens: String { text("providerObservability.estimatedTokens", "Estimated tokens") }
    static var providerObservabilityEstimatedCost: String { text("providerObservability.estimatedCost", "Estimated cost") }
    static var providerObservabilityNotes: String { text("providerObservability.notes", "Notes") }
    static var providerObservabilityThreshold: String { text("providerObservability.threshold", "Threshold") }
    static var providerObservabilityChartsTitle: String { text("providerObservability.charts.title", "Charts") }
    static var providerObservabilityChartsMode: String { text("providerObservability.charts.mode", "Redacted metadata") }
    static var providerObservabilityChartsSummary: String { text("providerObservability.charts.summary", "Charts summarize redacted local metadata only; detailed rows below remain the evidence trail.") }
    static var providerObservabilityDateRange: String { text("providerObservability.dateRange", "Range") }
    static var providerObservabilityLast7Days: String { text("providerObservability.dateRange.last7Days", "7 days") }
    static var providerObservabilityLast30Days: String { text("providerObservability.dateRange.last30Days", "30 days") }
    static var providerObservabilityLast90Days: String { text("providerObservability.dateRange.last90Days", "90 days") }
    static var providerObservabilityAllTime: String { text("providerObservability.dateRange.allTime", "All") }
    static var providerObservabilityCustomRange: String { text("providerObservability.dateRange.custom", "Custom") }
    static var providerObservabilityStartDate: String { text("providerObservability.dateRange.start", "Start") }
    static var providerObservabilityEndDate: String { text("providerObservability.dateRange.end", "End") }
    static var providerObservabilityRefresh: String { text("providerObservability.refresh", "Refresh") }
    static var providerObservabilityChartStatus: String { text("providerObservability.chart.status", "Call status") }
    static var providerObservabilityChartModelTokens: String { text("providerObservability.chart.modelTokens", "Model tokens") }
    static var providerObservabilityChartDestinationCost: String { text("providerObservability.chart.destinationCost", "Destination cost") }
    static var providerObservabilityChartModelLatency: String { text("providerObservability.chart.modelLatency", "Model latency") }
    static var providerObservabilityChartModelTaskConfidence: String { text("providerObservability.chart.modelTaskConfidence", "Model-task fit") }
    static var providerObservabilityChartEmpty: String { text("providerObservability.chart.empty", "No chart data") }
    static var taskCockpitTitle: String { text("taskCockpit.title", "Task Preflight") }
    static var taskCockpitBoundary: String { text("taskCockpit.boundary", "Read-only local preflight: decide whether the task is ready to hand off, which agent/skill fits, and what must be clarified first.") }
    static var taskCockpitReadOnlyFootnote: String { text("taskCockpit.readOnlyFootnote", "Read-only preflight: provider prompt is previewed and confirmation-gated; no config write or script execution.") }
    static var taskCockpitAction: String { text("taskCockpit.action.build", "Build Preflight") }
    static var taskCockpitRetry: String { text("taskCockpit.action.retry", "Retry") }
    static var taskCockpitPromptReady: String { text("taskCockpit.prompt.ready", "Redacted provider prompt is ready. Confirm before sending.") }
    static var taskCockpitPromptPreviewTitle: String { text("taskCockpit.promptPreview.title", "Provider Prompt Preview") }
    static var taskCockpitPromptPreviewSummary: String { text("taskCockpit.promptPreview.summary", "Review the redacted prompt metadata and destination before sending this Task Preflight request.") }
    static var taskCockpitPromptConfirmSend: String { text("taskCockpit.promptPreview.confirmSend", "Confirm Send and Generate") }
    static var taskCockpitUnavailable: String { text("taskCockpit.unavailable", "Task preflight is unavailable in this service build.") }
    static var taskCockpitTaskRequired: String { text("taskCockpit.taskRequired", "Enter a task.") }
    static var taskCockpitTaskPlaceholder: String { text("taskCockpit.task.placeholder", "Describe the task to hand off to an agent") }
    static var taskCockpitInputReady: String { text("taskCockpit.input.ready", "Ready to build preflight.") }
    static var taskCockpitNoResult: String { text("taskCockpit.empty.result", "Ready. Enter a task, then build Preflight.") }
    static var taskCockpitLoaded: String { text("taskCockpit.loaded", "Task preflight loaded from local evidence.") }
    static var taskCockpitCancelled: String { text("taskCockpit.cancelled", "Task preflight build was cancelled. No provider or write action was started.") }
    static var taskCockpitCatalogUnavailableDiagnostic: String { text("taskCockpit.diagnostic.catalogUnavailable", "The service returned preflight metadata without an available catalog.") }
    static var taskCockpitPartialNoRows: String { text("taskCockpit.diagnostic.partialNoRows", "The service returned preflight metadata, but no candidate, context, gap, blocker, or evidence rows.") }
    static var taskCockpitSections: String { text("taskCockpit.sections", "Preflight sections") }
    static var taskCockpitTasks: String { text("taskCockpit.tasks", "Task rows") }
    static var taskCockpitRoutes: String { text("taskCockpit.routes", "Route candidates") }
    static var taskCockpitAgents: String { text("taskCockpit.agents", "Agent candidates") }
    static var taskCockpitSkills: String { text("taskCockpit.skills", "Skill candidates") }
    static var taskCockpitReadinessSignals: String { text("taskCockpit.readinessSignals", "Readiness signals") }
    static var taskCockpitProviderContext: String { text("taskCockpit.providerContext", "Provider-observability context") }
    static var taskCockpitNoRows: String { text("taskCockpit.empty.rows", "No rows returned.") }
    static var taskCockpitRecommendedAgent: String { text("taskCockpit.recommendedAgent", "Recommended agent") }
    static var taskCockpitRecommendedSkill: String { text("taskCockpit.recommendedSkill", "Recommended skill") }
    static var taskCockpitNoReliableRecommendation: String { text("taskCockpit.recommendation.none", "No clear candidate path yet") }
    static func taskCockpitAgentOnlyRecommendation(_ agent: String) -> String {
        format("taskCockpit.recommendation.agentOnly", "%@ · Agent candidate, confirm the skill", agent)
    }
    static var taskCockpitPartialNotice: String { text("taskCockpit.partialNotice", "Some diagnostics did not return; the candidate path is still usable.") }
    static var taskCockpitVerdictReady: String { text("taskCockpit.verdict.ready", "Recommend agent handoff") }
    static var taskCockpitVerdictNeedsReview: String { text("taskCockpit.verdict.needsReview", "Recommend with confirmation") }
    static var taskCockpitVerdictBlocked: String { text("taskCockpit.verdict.blocked", "Do not hand off yet") }
    static var taskCockpitVerdictUnavailable: String { text("taskCockpit.verdict.unavailable", "Preflight unavailable") }
    static var taskCockpitVerdictReadyMessage: String { text("taskCockpit.verdict.ready.message", "A matching local route was found. Confirm once, then hand off.") }
    static var taskCockpitVerdictNeedsReviewMessage: String { text("taskCockpit.verdict.needsReview.message", "A candidate route exists; confirm command, network, or permission boundaries before handoff.") }
    static var taskCockpitVerdictBlockedMessage: String { text("taskCockpit.verdict.blocked.message", "No clear local candidate path was found. Add product, resource, or action details.") }
    static var taskCockpitVerdictUnavailableMessage: String { text("taskCockpit.verdict.unavailable.message", "Not enough local data was returned to make a recommendation.") }
    static var taskCockpitReadinessShort: String { text("taskCockpit.score.readiness", "Readiness") }
    static var taskCockpitRoutingShort: String { text("taskCockpit.score.routing", "Routing") }
    static var taskCockpitRecommendationTitle: String { text("taskCockpit.recommendation.title", "Candidate path") }
    static var taskCockpitCandidateAlternativesTitle: String { text("taskCockpit.candidates.title", "Closest skill candidates") }
    static var taskCockpitReasonsTitle: String { text("taskCockpit.reasons.title", "Key reasons") }
    static var taskCockpitNoReasons: String { text("taskCockpit.reasons.empty", "No readable route reasons were returned.") }
    static var taskCockpitReasonReadinessBlocked: String { text("taskCockpit.reason.readinessBlocked", "There is not enough local fit evidence for a confident handoff.") }
    static var taskCockpitReasonRoutingBlocked: String { text("taskCockpit.reason.routingBlocked", "No stable skill route matched the task.") }
    static var taskCockpitReasonTaskWordingWeak: String { text("taskCockpit.reason.taskWordingWeak", "The task is too broad; add product, system, or action details.") }
    static var taskCockpitReasonExecNeedsHuman: String { text("taskCockpit.reason.execNeedsHuman", "The candidate skill may execute commands; confirm the action first.") }
    static var taskCockpitReasonNetworkDeclared: String { text("taskCockpit.reason.networkDeclared", "The candidate skill declares network access; confirm destination and permissions.") }
    static var taskCockpitReasonRouteAmbiguous: String { text("taskCockpit.reason.routeAmbiguous", "Nearby routes exist; make the task more specific.") }
    static var taskCockpitReasonCrossAgentDuplicate: String { text("taskCockpit.reason.crossAgentDuplicate", "Cross-agent duplicate or overlap signals may affect routing.") }
    static var taskCockpitReasonTaskFitWeak: String { text("taskCockpit.reason.taskFitWeak", "Task fit is weak, so choosing this skill may be inaccurate.") }
    static var taskCockpitReasonProductMatched: String { text("taskCockpit.reason.productMatched", "The task product/resource matches the candidate skill scope.") }
    static var taskCockpitReasonProductMismatch: String { text("taskCockpit.reason.productMismatch", "The task product/resource does not match this skill scope.") }
    static var taskCockpitProviderPartialSummary: String { text("taskCockpit.provider.partialSummary", "The model returned candidate information in an incomplete format; showing the recovered candidate summary.") }
    static var taskCockpitProviderUnparsed: String { text("taskCockpit.provider.unparsed", "The model response format was incomplete, so candidate details could not be parsed reliably.") }
    static var taskCockpitAttentionTitle: String { text("taskCockpit.attention.title", "Needs attention") }
    static var taskCockpitNoAttentionItems: String { text("taskCockpit.attention.empty", "No issue needs attention.") }
    static var taskCockpitNextStepReady: String { text("taskCockpit.next.ready", "Next: after human confirmation, hand off to the recommended agent.") }
    static var taskCockpitNextStepNeedsReview: String { text("taskCockpit.next.needsReview", "Next: confirm the boundary, or add details and regenerate.") }
    static var taskCockpitNextStepBlocked: String { text("taskCockpit.next.blocked", "Next: add product, resource, or action details, then regenerate.") }
    static var taskCockpitNextStepUnavailable: String { text("taskCockpit.next.unavailable", "Next: refresh the catalog, or choose a project/agent and retry.") }
    static var taskCockpitDiagnosticsTitle: String { text("taskCockpit.diagnostics.title", "Technical diagnostics") }
    static var taskCockpitDiagnosticsSummary: String { text("taskCockpit.diagnostics.summary", "For troubleshooting only: key matching steps and compact candidate evidence.") }
    static var taskCockpitDiagnosticsProcess: String { text("taskCockpit.diagnostics.process", "Matching process") }
    static var taskCockpitDiagnosticsTopRoute: String { text("taskCockpit.diagnostics.topRoute", "Top route") }
    static var taskCockpitDiagnosticsScanned: String { text("taskCockpit.diagnostics.scanned", "Scanned") }
    static var taskCockpitHistorySummary: String { text("taskCockpit.history.summary", "Completed Preflights stay in memory for this app session. Task text and provider results are not saved to disk and disappear when the app quits.") }
    static var taskCockpitHistoryClear: String { text("taskCockpit.history.clear", "Clear session history") }
    static var taskCockpitHistoryClearConfirmationTitle: String { text("taskCockpit.history.clearConfirmation.title", "Clear session history?") }
    static var taskCockpitHistoryClearConfirmationMessage: String { text("taskCockpit.history.clearConfirmation.message", "This clears completed Preflights from this app session and retries removal of prior local history. This cannot be undone.") }
    static var taskCockpitHistoryCleanupFailed: String { text("taskCockpit.history.cleanupFailed", "Prior local Task Preflight history could not be removed. Clear session history to retry.") }
    static var taskCockpitProgressTitle: String { text("taskCockpit.progress.title", "Progressive feedback") }
    static var taskCockpitProgressActionReview: String { text("taskCockpit.progress.actionReview", "Action review") }
    static var taskCockpitProgressBatchChecks: String { text("taskCockpit.progress.batchChecks", "Batch checks") }
    static var taskCockpitProgressPending: String { text("taskCockpit.progress.pending", "Pending") }
    static var taskCockpitProgressChecking: String { text("taskCockpit.progress.checking", "Checking") }
    static var taskCockpitProgressReady: String { text("taskCockpit.progress.ready", "Ready") }
    static var taskCockpitProgressNoRows: String { text("taskCockpit.progress.noRows", "No rows") }
    static var taskCockpitProgressPartial: String { text("taskCockpit.progress.partial", "Partial") }
    static var taskCockpitProgressSkipped: String { text("taskCockpit.progress.skipped", "Skipped") }
    static var taskCockpitProgressUnavailable: String { text("taskCockpit.progress.unavailable", "Unavailable") }
    static var taskCockpitProgressFallback: String { text("taskCockpit.progress.fallback", "Fallback / partial") }
    static var taskCockpitProgressCancelled: String { text("taskCockpit.progress.cancelled", "Cancelled") }
    static var taskCockpitProgressTimedOut: String { text("taskCockpit.progress.timedOut", "Timed out") }
    static var taskCockpitProgressFailed: String { text("taskCockpit.progress.failed", "Stopped") }

    static func taskCockpitPreparingStatus(elapsedSeconds: Int, timeoutSeconds: Int) -> String {
        format(
            "taskCockpit.preparingStatus",
            "Building preflight... %@ / %@.",
            taskCockpitDuration(elapsedSeconds),
            taskCockpitDuration(timeoutSeconds)
        )
    }

    static func taskCockpitTimedOut(_ timeoutSeconds: Int) -> String {
        format("taskCockpit.timedOut", "Preflight did not finish within %@; retry later.", taskCockpitDuration(timeoutSeconds))
    }

    static func taskCockpitFailed(_ reason: String) -> String {
        format("taskCockpit.failed", "Task preflight build stopped: %@.", reason)
    }

    static func taskCockpitLoadedWithFallback(_ _: String) -> String {
        text("taskCockpit.loadedWithFallback", "Core recommendation is ready; some diagnostics did not return.")
    }

    static func taskCockpitElapsedSeconds(_ elapsedSeconds: Int) -> String {
        let safeElapsedSeconds = max(0, elapsedSeconds)
        if safeElapsedSeconds == 1 {
            return format("taskCockpit.elapsedSecond", "Elapsed: %d second.", safeElapsedSeconds)
        }
        return format("taskCockpit.elapsedSeconds", "Elapsed: %d seconds.", safeElapsedSeconds)
    }

    static func taskCockpitProgressBlocked(_ blockerCount: Int) -> String {
        format("taskCockpit.progress.blocked", "%d blockers", blockerCount)
    }

    static func taskCockpitProgressRows(_ rowCount: Int) -> String {
        format("taskCockpit.progress.rows", "%d rows", rowCount)
    }
    static var llmAssist: String { text("llm.assist", "LLM Assist") }
    static var llmEnabled: String { text("llm.enabled", "Enabled") }
    static var llmDisabled: String { text("llm.disabled", "Disabled") }
    static var llmPreparing: String { text("llm.preparing", "Preparing...") }
    static var llmPreparePrompt: String { text("llm.preparePrompt", "Choose an action to preview tokens and cost.") }
    static var llmDisabledFallback: String { text("llm.disabledFallback", "LLM assist is unavailable in this build.") }
    static var llmPromptUnavailable: String { text("llm.promptPreview.unavailable", "LLM prompt preview is unavailable in this build.") }
    static var llmProvider: String { text("llm.provider", "Provider") }
    static var llmModel: String { text("llm.model", "Model") }
    static var llmTokens: String { text("llm.tokens", "Tokens") }
    static var llmCost: String { text("llm.cost", "Cost") }
    static var llmConfirmationRequired: String { text("llm.confirmationRequired", "Confirmation required before any LLM call.") }
    static var llmDraftCopyRequired: String { text("llm.draftCopyRequired", "Draft output requires user confirmation and copy.") }
    static var llmPromptPreviewTitle: String { text("llm.promptPreview.title", "Prompt Preview") }
    static var llmPromptPreviewAction: String { text("llm.promptPreview.action", "Preview Prompt") }
    static var llmPromptConfirmSend: String { text("llm.promptPreview.confirmSend", "Confirm & Send") }
    static var llmPromptSending: String { text("llm.promptPreview.sending", "Waiting for provider response; long-running models may take up to 10 minutes.") }
    static var llmPromptProviderRequired: String { text("llm.promptPreview.providerRequired", "Configure and save an AI provider before sending.") }
    static var llmPromptPreviewRequired: String { text("llm.promptPreview.previewRequired", "Preview the current prompt before sending.") }
    static var llmPromptSendSucceeded: String { text("llm.promptPreview.sendSucceeded", "Provider response received.") }
    static var llmPromptSendFailed: String { text("llm.promptPreview.sendFailed", "Provider request failed.") }
    static var llmPromptScope: String { text("llm.promptPreview.scope", "Prompt scope") }
    static var llmPromptDestination: String { text("llm.promptPreview.destination", "Destination") }
    static var llmPromptIncludedFields: String { text("llm.promptPreview.includedFields", "Included fields") }
    static var llmPromptExcludedFields: String { text("llm.promptPreview.excludedFields", "Excluded fields") }
    static var llmPromptRedactedPrompt: String { text("llm.promptPreview.redactedPrompt", "Redacted prompt") }
    static var llmPromptNoFields: String { text("llm.promptPreview.noFields", "No fields reported.") }
    static var llmPromptRawPromptStored: String { text("llm.promptPreview.rawPromptStored", "Raw prompt stored") }
    static var llmPromptRawResponseStored: String { text("llm.promptPreview.rawResponseStored", "Raw response stored") }
    static var llmPromptCopyOnly: String { text("llm.promptPreview.copyOnly", "Copy-only output") }
    static var llmPromptOutput: String { text("llm.promptPreview.output", "Provider output") }
    static var llmPromptCopyOutput: String { text("llm.promptPreview.copyOutput", "Copy Output") }
    static var llmPromptViewDetails: String { text("llm.promptPreview.viewDetails", "View Details") }
    static var llmPromptCopyFullText: String { text("llm.promptPreview.copyFullText", "Copy Full Text") }
    static var llmPromptCloseDetails: String { text("llm.promptPreview.closeDetails", "Close") }
    static var llmPromptHistoryNote: String { text("llm.promptPreview.historyNote", "Latest provider output is shown here and saved in local prompt run history.") }
    static var llmPromptHistoricalResponse: String { text("llm.promptPreview.historicalResponse", "Previous provider response") }
    static var llmPromptNoOutput: String { text("llm.promptPreview.noOutput", "Provider response did not include copy-only output text.") }
    static func localizedServiceMessage(_ value: String) -> String {
        let trimmed = value.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmed.isEmpty else { return value }

        if let (code, rest) = splitErrorCodePrefix(trimmed) {
            let localizedRest = localizedServiceMessage(rest)
            if localizedRest != rest {
                return "\(code): \(localizedRest)"
            }
        }

        if let profileID = backtickValue(
            in: trimmed,
            prefix: "Provider profile ",
            suffix: " is configured; provider calls remain user-triggered and confirmation-gated."
        ) {
            return format("service.message.providerProfileConfigured", "Provider profile `%@` is configured; provider calls remain user-triggered and confirmation-gated.", profileID)
        }
        if let profileID = backtickValue(
            in: trimmed,
            prefix: "Provider profile ",
            suffix: " exists but is disabled."
        ) {
            return format("service.message.providerProfileExistsDisabled", "Provider profile `%@` exists but is disabled.", profileID)
        }
        if let profileID = backtickValue(
            in: trimmed,
            prefix: "Provider profile ",
            suffix: " exists but its API key is unavailable from the OS credential store."
        ) {
            return format("service.message.providerProfileMissingKey", "Provider profile `%@` exists but its API key is unavailable from the OS credential store.", profileID)
        }
        if let profileID = backtickValue(in: trimmed, prefix: "Provider profile ", suffix: " is disabled.") {
            return format("service.message.providerProfileDisabled", "Provider profile `%@` is disabled.", profileID)
        }

        switch trimmed {
        case "No enabled provider profile is configured; no provider request can be sent.":
            return text("service.message.noEnabledProviderProfile", "No enabled provider profile is configured; no provider request can be sent.")
        case "Provider profiles exist, but none is enabled as the default provider.":
            return text("service.message.providerProfilesNoDefault", "Provider profiles exist, but none is enabled as the default provider.")
        case "LLM actions are disabled by default; no local provider is configured.":
            return text("service.message.llmNoProviderConfigured", "LLM actions are disabled by default; no local provider is configured.")
        case "Monthly provider budget is 0; provider requests are disabled.":
            return text("service.message.monthlyBudgetZero", "Monthly provider budget is 0; provider requests are disabled.")
        case "Single request token limit is lower than the redacted prompt estimate.":
            return text("service.message.tokenLimitBelowRedactedEstimate", "Single request token limit is lower than the redacted prompt estimate.")
        case "Single request token limit is lower than the prompt estimate.":
            return text("service.message.tokenLimitBelowPromptEstimate", "Single request token limit is lower than the prompt estimate.")
        case "Single request token limit is lower than the connection test estimate.":
            return text("service.message.tokenLimitBelowConnectionEstimate", "Single request token limit is lower than the connection test estimate.")
        case "Redacted prompt preview is ready for explicit confirmation.":
            return text("service.message.redactedPromptReady", "Redacted prompt preview is ready for explicit confirmation.")
        case "Confirm to send only this redacted prompt to the displayed provider endpoint.":
            return text("service.message.confirmRedactedPrompt", "Confirm to send only this redacted prompt to the displayed provider endpoint.")
        case "No provider profile is available for the confirmed prompt.":
            return text("service.message.noProviderForConfirmedPrompt", "No provider profile is available for the confirmed prompt.")
        case "preview_id does not match the current redacted prompt preview":
            return text("service.message.previewIDMismatch", "preview_id does not match the current redacted prompt preview")
        case "Provider profile is disabled; no request was sent.":
            return text("service.message.providerProfileDisabledNoRequest", "Provider profile is disabled; no request was sent.")
        case "Explicit confirmation id is required before a provider test.":
            return text("service.message.providerTestConfirmationRequired", "Explicit confirmation id is required before a provider test.")
        case "Explicit confirmation id is required before a provider prompt request.":
            return text("service.message.providerPromptConfirmationRequired", "Explicit confirmation id is required before a provider prompt request.")
        case "Provider budget settings block the test request.":
            return text("service.message.providerBudgetBlocksTest", "Provider budget settings block the test request.")
        case "Redacted prompt is empty; no request was sent.":
            return text("service.message.redactedPromptEmpty", "Redacted prompt is empty; no request was sent.")
        case "API key stored in the OS credential store.":
            return text("service.message.apiKeyStored", "API key stored in the OS credential store.")
        case "API key is available from the OS credential store.":
            return text("service.message.apiKeyAvailable", "API key is available from the OS credential store.")
        case "No API key is stored for this profile.":
            return text("service.message.noAPIKeyStored", "No API key is stored for this profile.")
        case "Connection test is within configured local budget limits.":
            return text("service.message.connectionTestWithinBudget", "Connection test is within configured local budget limits.")
        case "Provider connection test succeeded.":
            return aiProviderTestSucceeded
        case "Provider connection test failed.":
            return aiProviderTestFailed
        case "Provider response received.":
            return llmPromptSendSucceeded
        case "Provider request failed.":
            return llmPromptSendFailed
        case "Service call timed out before the sidecar returned a complete response.":
            return text("service.error.sidecarTimedOut", "Service call timed out before the sidecar returned a complete response.")
        default:
            return trimmed
        }
    }
    static func markdownTableHiddenRows(_ count: Int) -> String {
        format("llm.markdown.table.hiddenRows", "%d more rows in full details", count)
    }
    static var markdownTablePreviewSummary: String { text("llm.markdown.table.previewSummary", "Table content is folded in this preview. Open details to inspect the full Markdown table.") }
    static var scriptExecutionSafety: String { text("scriptExecution.safety", "Script Execution Safety") }
    static var scriptExecutionPreviewOnly: String { text("scriptExecution.previewOnly", "Preview-only") }
    static var scriptExecutionUnavailable: String { text("scriptExecution.unavailable", "Script execution preflight is unavailable in this service build. Scripts remain non-executable from the native UI.") }
    static var scriptExecutionBlockedNote: String { text("scriptExecution.blockedNote", "The native UI does not execute scripts. Use this panel only to inspect the safety gate data returned by the service.") }
    static var scriptExecutionPreviewSummary: String { text("scriptExecution.previewSummary", "Script execution is blocked by default until a separate confirmed service path is available.") }
    static var scriptExecutionNoCommand: String { text("scriptExecution.noCommand", "No command preview is available.") }
    static var scriptExecutionNoRisks: String { text("scriptExecution.noRisks", "No service risks were reported.") }
    static var scriptExecutionNoAudit: String { text("scriptExecution.noAudit", "No audit identifier reported.") }
    static var scriptExecutionAuditStatus: String { text("scriptExecution.auditStatus", "Audit status") }
    static var scriptExecutionAuditID: String { text("scriptExecution.auditId", "Audit ID") }
    static var scriptExecutionCommand: String { text("scriptExecution.command", "Command preview") }
    static var scriptExecutionCWD: String { text("scriptExecution.cwd", "CWD") }
    static var scriptExecutionEnv: String { text("scriptExecution.env", "Environment") }
    static var scriptExecutionNetwork: String { text("scriptExecution.network", "Network") }
    static var scriptExecutionFiles: String { text("scriptExecution.files", "Files") }
    static var scriptExecutionRisks: String { text("scriptExecution.risks", "Risks") }
    static var scriptExecutionConfirmationRequired: String { text("scriptExecution.confirmationRequired", "Human confirmation is required before any future execution service path.") }
    static var scriptExecutionEnvEmpty: String { text("scriptExecution.envEmpty", "No environment overrides") }
    static var scriptExecutionFilesEmpty: String { text("scriptExecution.filesEmpty", "No file scope declared") }
    static var toggleUnavailableBusy: String { text("detail.toggleUnavailable.busy", "A write is already in progress.") }
    static var toggleUnavailableBroken: String { text("detail.toggleUnavailable.broken", "Broken skills cannot be toggled until their SKILL.md can be parsed.") }
    static var toggleUnavailableMissing: String { text("detail.toggleUnavailable.missing", "Missing skills cannot be toggled because the source file was not found during the last scan.") }
    static var toggleUnavailableShadowed: String { text("detail.toggleUnavailable.shadowed", "Shadowed skills are read-only here; resolve the active copy before toggling.") }
    static var toggleUnavailableUnknown: String { text("detail.toggleUnavailable.unknown", "This skill has an unknown catalog state and is read-only in this build.") }
    static var toggleUnavailableToolGlobal: String { text("detail.toggleUnavailable.toolGlobal", "Tool-global skills are read-only previews. Install or copy to an agent requires a separate confirmed action.") }
    static var guardedToggle: String { text("detail.guardedToggle", "Guarded toggle") }
    static var piGuardedToggle: String { text("detail.pi.guardedToggle", "Guarded toggle") }
    static var piGuardedToggleBoundary: String { text("detail.pi.guardedToggle.boundary", "Pi toggle is guarded by preview, trust checks, config snapshot, and rollback. Native installs use the separate confirmed install flow; package installs and compatibility-root file writes stay blocked.") }
    static var hermesGuardedToggleBoundary: String { text("detail.hermes.guardedToggle.boundary", "Hermes toggle is guarded by preview, config snapshot, read-back, and rollback. It only edits skills.disabled; platform_disabled and external_dirs writes stay blocked.") }
    static var openClawGuardedToggleBoundary: String { text("detail.openClaw.guardedToggle.boundary", "OpenClaw toggle is guarded by preview, config snapshot, read-back, and rollback. It only edits skills.entries.<key>.enabled; other config keys stay blocked.") }
    static func guardedToggleBoundary(_ agent: String) -> String {
        format("detail.guardedToggle.boundary", "%@ toggle is guarded by preview, config snapshot, read-back, and rollback.", agent)
    }
    static var operationUnavailableBusy: String { text("detail.operationUnavailable.busy", "Another catalog operation is already in progress.") }
    static var readOnly: String { text("detail.readOnly", "Read-only") }
    static var hermesHomeProfileAccess: String { text("detail.hermes.homeProfileAccess", "Hermes home/profile skills support guarded toggles through skills.disabled. Installs remain limited to the native Hermes skills root.") }
    static var hermesExternalAccess: String { text("detail.hermes.externalAccess", "Hermes external_dirs are explicit read-only roots, not project roots or install targets. Guarded toggles still write only global skills.disabled.") }
    static var openClawWorkspaceScope: String { text("scope.openClawWorkspace", "Workspace") }
    static var openClawWorkspaceBoundary: String { text("openClaw.workspace.boundary", "OpenClaw scans only workspace skill roots (<workspace>/skills and <workspace>/.agents/skills). Generic repository roots are skipped rather than shown as missing skills.") }
    static var openClawReadOnlyAccess: String { text("detail.openClaw.readOnlyAccess", "OpenClaw skills support guarded toggles through skills.entries.<key>.enabled. Workspace installs remain limited to confirmed workspace skills roots.") }
    static var openClawToggleBlocked: String { text("detail.openClaw.toggleBlocked", "OpenClaw toggle needs the guarded config capability; unsupported OpenClaw config keys remain blocked.") }
    static var currentMatchesSnapshot: String { text("snapshot.matches", "Current agent config already matches this snapshot.") }
    static var currentDiffersFromSnapshot: String { text("snapshot.differs", "Current agent config differs from this snapshot.") }
    static var menuScanSkills: String { text("menu.scanSkills", "Scan Skills") }
    static var menuReloadSkills: String { text("menu.reloadSkills", "Reload Skills") }
    static var menuSkills: String { text("menu.skills", "Skills") }
    static var menuShowTaskCockpit: String { text("menu.showTaskCockpit", "Show Task Preflight") }
    static var menuShowOverview: String { text("menu.showOverview", "Show Overview") }
    static var menuShowFindings: String { text("menu.showFindings", "Show Issues") }
    static var menuClearSearch: String { text("menu.clearSearch", "Clear Search") }

    static func enabledSummary(enabled: Int, total: Int) -> String {
        format("sidebar.enabledSummary", "%d of %d enabled", enabled, total)
    }

    static func visibleSummary(_ count: Int) -> String {
        format("sidebar.visibleSummary", "%d visible", count)
    }

    static func localSessionContentCharacters(_ count: Int) -> String {
        format("localSessionContent.characters", "Chars: %d", count)
    }

    static func agentConfigTimelineSummary(_ agent: String, _ count: Int) -> String {
        format("sidebar.agentConfigTimeline.summary", "%@ config snapshots · %d rollback points", agent, count)
    }

    static func agentConfigTimelineEmptySummary(_ agent: String) -> String {
        format("sidebar.agentConfigTimeline.emptySummary", "No %@ config snapshots yet", agent)
    }

    static func agentConfigTimelineMore(_ count: Int) -> String {
        format("sidebar.agentConfigTimeline.more", "%d older rollback points hidden to keep the sidebar quiet.", count)
    }

    static func agentConfigTimelineRollbackConfirm(_ target: String) -> String {
        format("sidebar.agentConfigTimeline.rollbackConfirm", "Rollback restores this agent config file only after confirmation. Skill content snapshots are not included. Target: %@", target)
    }

    static func visibleFindingsSummary(_ visible: Int, _ total: Int) -> String {
        format("findings.visibleSummary", "%d of %d issues", visible, total)
    }

    static func visibleFindingGroupsSummary(_ visibleGroups: Int, _ totalGroups: Int, _ visibleEntries: Int) -> String {
        format("findings.visibleGroupSummary", "%d of %d issue groups · %d scan entries", visibleGroups, totalGroups, visibleEntries)
    }

    static func findingSeverityGroupCount(_ count: Int) -> String {
        format("findings.severityGroupCount", "%d issue groups", count)
    }

    static func findingIssueImpact(_ instances: Int, _ entries: Int) -> String {
        format("findings.issueImpact", "Impacted instances: %d · Scan entries: %d", instances, entries)
    }

    static func findingTriageUpdated(_ status: String) -> String {
        format("findings.triage.updated", "Set local finding triage to %@. No agent config or skill files were changed.", status)
    }

    static var findingTriageReopened: String { text("findings.triage.reopened", "Reopened finding locally. No agent config or skill files were changed.") }

    static func ruleTuningSetSeverity(_ severity: String) -> String {
        format("rules.tuning.setSeverity", "Set %@", severity)
    }

    static func ruleTuningSeverityUpdated(_ severity: String) -> String {
        format("rules.tuning.updated.severity", "Set app-local rule severity override to %@. No skill files, agent config, snapshots, scripts, AI provider calls, or credentials were touched.", severity)
    }

    static var ruleTuningSeverityCleared: String { text("rules.tuning.cleared.severity", "Cleared app-local rule severity override. No skill files or agent config were changed.") }
    static var ruleTuningSuppressionUpdated: String { text("rules.tuning.updated.suppression", "Updated app-local rule suppression. No skill files, agent config, snapshots, scripts, AI provider calls, or credentials were touched.") }
    static var ruleTuningSuppressionCleared: String { text("rules.tuning.cleared.suppression", "Cleared app-local rule suppression. No skill files or agent config were changed.") }

    static func noFindingsForSkillMessage(_ agent: String) -> String {
        format("empty.noFindingsForSkill.message", "No rule issues are associated with this %@ skill.", agent)
    }

    static func findingScopeSummary(_ skill: String, _ agent: String) -> String {
        format("findings.scopeSummary", "%@ · %@", skill, agent)
    }

    static func findingCatalogTarget(definition: String, instance: String) -> String {
        format("findings.catalogTarget.definitionInstance", "Definition %@ · Instance %@", definition, instance)
    }

    static func findingCatalogDefinition(_ definition: String) -> String {
        format("findings.catalogTarget.definition", "Definition %@", definition)
    }

    static func findingCatalogInstance(_ instance: String) -> String {
        format("findings.catalogTarget.instance", "Instance %@", instance)
    }

    static func findingRemediationFallback(_ ruleID: String) -> String {
        format("findings.remediation.fallback", "Review rule %@, update the skill source, then rescan to confirm the finding is resolved.", ruleID)
    }

    static func permissionUnknownValue(_ value: String) -> String {
        format("permissions.unknownValue", "Unknown (%@)", value)
    }

    static func scannedSkills(_ count: Int) -> String {
        format("message.scannedSkills", "Scanned %d skills across supported adapters.", count)
    }

    static func refreshReloaded(_ skills: Int, _ findings: Int, _ conflicts: Int) -> String {
        format("refresh.reloaded", "Reloaded %d skills, %d findings, and %d same-agent conflicts.", skills, findings, conflicts)
    }

    static func refreshScanComplete(_ scanned: Int, _ skills: Int, _ findings: Int, _ conflicts: Int) -> String {
        format("refresh.scanComplete", "Scan complete: %d scanned, %d in catalog, %d findings, %d same-agent conflicts.", scanned, skills, findings, conflicts)
    }

    static func refreshScanPartial(
        _ scanned: Int,
        _ skills: Int,
        _ findings: Int,
        _ conflicts: Int,
        issue: String,
        recovery: String
    ) -> String {
        format(
            "refresh.scanPartial",
            "Scan completed-partial: %d scanned, %d in catalog, %d findings, %d same-agent conflicts. First issue: %@. Recovery: %@",
            scanned,
            skills,
            findings,
            conflicts,
            issue,
            recovery
        )
    }

    static var refreshPartialIssueUnavailable: String {
        text("refresh.partialIssueUnavailable", "No typed issue detail was returned.")
    }

    static func refreshPartialIssue(kind: String, path: String, detail: String) -> String {
        format("refresh.partialIssue", "%@ at %@: %@", kind, path, detail)
    }

    static var refreshPartialRecoveryDefault: String {
        text("refresh.partialRecoveryDefault", "Review scan diagnostics, fix the affected root, then retry Scan.")
    }

    static func refreshFailed(_ reason: String) -> String {
        format("refresh.failed", "Refresh failed: %@. Retry when the issue is fixed.", reason)
    }

    static func stateUnknownValue(_ value: String) -> String {
        format("state.unknownValue", "Unknown (%@)", value)
    }

    static func toggleUnavailableReadOnlyAdapter(_ agent: String) -> String {
        format("detail.toggleUnavailable.readOnlyAdapter", "%@ skills are read-only in this build.", agent)
    }

    static func adapterNotImplementedMessage(_ agent: String) -> String {
        format("empty.adapterNotImplemented.message", "%@ adapter is not implemented yet. Check the capability status above for the current blocker.", agent)
    }

    static func readOnlyAdapterStatus(_ agent: String) -> String {
        format("detail.readOnlyAdapterStatus", "%@ adapter is read-only in this build.", agent)
    }

    static func toolGlobalAccessStatus(_ agent: String) -> String {
        format("detail.toolGlobal.accessStatus", "%@ tool-global staging is a read-only preview until installed into a specific agent.", agent)
    }

    static func toolGlobalInstallPreviewSummary(_ skill: String, _ agent: String) -> String {
        format("detail.toolGlobal.installPreviewSummary", "Preview copying %@ into %@. No files are written from this preview.", skill, agent)
    }

    static func toolGlobalInstallConfirmation(_ skill: String, _ agent: String) -> String {
        format("detail.toolGlobal.installConfirmation", "Installing %@ into %@ will require confirmation of the target path and adapter write semantics before any copy happens.", skill, agent)
    }

    static func toolGlobalInstalled(_ skill: String, _ agent: String) -> String {
        format("message.toolGlobalInstalled", "Installed %@ into %@.", skill, agent)
    }

    static func batchToggleSelectedCount(_ count: Int) -> String {
        format("batchToggle.selectedCount", "%d visible", count)
    }

    static func batchToggleScopeSummary(agent: String, visible: Int, selected: Int) -> String {
        format("batchToggle.scopeSummary", "%@ · %d visible · %d selected", agent, visible, selected)
    }

    static func batchToggleActionTarget(_ action: String) -> String {
        format("batchToggle.actionTarget", "Target: %@", action)
    }

    static func batchToggleAffectedSkills(_ count: Int) -> String {
        format("batchToggle.affectedSkills", "Affected skills (%d)", count)
    }

    static func batchToggleSkippedSkills(_ count: Int) -> String {
        format("batchToggle.skippedSkills", "Skipped read-only / ineligible (%d)", count)
    }

    static func batchToggleConfirmTitle(action: String, count: Int) -> String {
        format("batchToggle.confirm.title", "Apply %@ to %d writable skills?", action, count)
    }

    static func batchToggleConfirmApply(action: String, count: Int) -> String {
        format("batchToggle.confirm.apply", "Apply %@ to %d skills", action, count)
    }

    static func batchToggleConfirmMessage(action: String, affected: Int, skipped: Int, snapshot: String) -> String {
        format(
            "batchToggle.confirm.message",
            "This will %@ %d writable skills and skip %d read-only, ineligible, or no-op skills. %@",
            action,
            affected,
            skipped,
            snapshot
        )
    }

    static func batchToggleMoreItems(_ count: Int) -> String {
        format("batchToggle.moreItems", "%d more hidden to keep the sidebar compact.", count)
    }

    static func batchToggleAlreadyInTargetState(_ action: String) -> String {
        format("batchToggle.alreadyTarget", "Already %@", action)
    }

    static func batchToggleCapabilityMissing(_ agent: String) -> String {
        format("batchToggle.capabilityMissing", "%@ writable capability is not verified in this service response.", agent)
    }

    static func batchToggleWritableMissing(_ agent: String) -> String {
        format("batchToggle.writableMissing", "%@ root is not verified writable.", agent)
    }

    static func batchToggleApplied(action: String, count: Int) -> String {
        format("batchToggle.applied", "%@ batch applied to %d writable skills after preview confirmation.", action, count)
    }

    static func providerObservabilityLogCount(_ count: Int, total: Int) -> String {
        format("providerObservability.logs.count", "%d of %d calls", count, total)
    }

    static func providerObservabilityMoreRows(_ count: Int) -> String {
        format("providerObservability.logs.moreRows", "%d more rows available. Narrow filters or search to inspect them.", count)
    }

    static func toggledSkill(on: Bool, name: String) -> String {
        format(on ? "message.enabledSkill" : "message.disabledSkill", on ? "Enabled %@." : "Disabled %@.", name)
    }

    static func toggledSkill(on: Bool, name: String, agent: String) -> String {
        let message = toggledSkill(on: on, name: name)
        if agent == "codex" {
            return "\(message) \(codexRestartRequired)"
        }
        return message
    }

    static var codexRestartRequired: String { text("message.codexRestartRequired", "Codex runtime may need restart to read config.toml changes.") }

    static func rollbackRescanned(_ count: Int) -> String {
        format("message.rollbackRescanned", "Rolled back agent config snapshot and rescanned %d skills.", count)
    }

    static var refreshAfterWrite: String { text("refresh.afterWrite", "Catalog refreshed after the settings write.") }

    static func refreshAfterRollback(_ count: Int) -> String {
        format("refresh.afterRollback", "Catalog refreshed after agent config rollback with %d scanned skills.", count)
    }

    static var refreshAfterSettingsSave: String { text("refresh.afterSettingsSave", "Catalog refreshed after saving settings.") }

    static func charactersCaptured(_ count: Int) -> String {
        format("snapshot.charactersCaptured", "%d characters captured", count)
    }

    static func llmTokenSummary(input: Int, output: Int, total: Int) -> String {
        format("llm.tokenSummary", "%d in / %d out / %d total", input, output, total)
    }

    static func llmEstimatedCost(_ cost: Double) -> String {
        format("llm.estimatedCost", "$%.4f estimated", cost)
    }

    static func activityToggleState(enabled: Bool) -> String {
        format("detail.activity.toggleState", "Set to %@", enabled ? stateEnabled : stateDisabled)
    }

    static func scriptExecutionAuditStatusTitle(_ status: ScriptExecutionAuditStatus) -> String {
        switch status {
        case .unavailable:
            return text("scriptExecution.auditStatus.unavailable", "Unavailable")
        case .previewOnly:
            return text("scriptExecution.auditStatus.previewOnly", "Preview only")
        case .blocked:
            return text("scriptExecution.auditStatus.blocked", "Blocked")
        case .requiresConfirmation:
            return text("scriptExecution.auditStatus.requiresConfirmation", "Requires confirmation")
        case .audited:
            return text("scriptExecution.auditStatus.audited", "Audited")
        case .unknown:
            return text("scriptExecution.auditStatus.unknown", "Unknown")
        }
    }

    static var savedSettings: String { text("message.savedSettings", "Saved settings and refreshed catalog.") }

    static func projectSelectedAndScanned(_ name: String) -> String {
        format("message.projectSelectedAndScanned", "Selected %@ and refreshed catalog.", name)
    }

    static var projectClearedAndScanned: String { text("message.projectClearedAndScanned", "Cleared project context and refreshed catalog.") }
    static var projectScanSkippedValidation: String { text("refresh.projectValidationSkipped", "Project context needs attention before scanning.") }

    static func projectValidationFailed(_ reason: String) -> String {
        format("project.validationFailed", "Project validation failed: %@.", reason)
    }

    private static var activeLanguage = AppLanguage.fromStorage(UserDefaults.standard.string(forKey: AppLanguage.storageKey))
    private static var cachedLocalizedStrings: (language: AppLanguage, strings: [String: String])?

    @discardableResult
    static func use(_ language: AppLanguage) -> AppLanguage {
        activeLanguage = language
        if cachedLocalizedStrings?.language != language {
            cachedLocalizedStrings = nil
        }
        return language
    }

    static var currentLanguage: AppLanguage {
        activeLanguage
    }

    static func text(_ key: String, _ defaultValue: String) -> String {
        if let value = localizedStrings()[key] {
            return value
        }
        let nativeValue = Bundle.main.localizedString(forKey: key, value: nil, table: nil)
        if nativeValue != key {
            return nativeValue
        }
        return defaultValue
    }

    private static func format(_ key: String, _ defaultValue: String, _ arguments: CVarArg...) -> String {
        String(format: text(key, defaultValue), arguments: arguments)
    }

    private static func splitErrorCodePrefix(_ value: String) -> (String, String)? {
        guard let separator = value.firstIndex(of: ":") else { return nil }
        let code = String(value[..<separator]).trimmingCharacters(in: .whitespacesAndNewlines)
        let rest = String(value[value.index(after: separator)...]).trimmingCharacters(in: .whitespacesAndNewlines)
        guard !code.isEmpty, !rest.isEmpty, code.rangeOfCharacter(from: .whitespacesAndNewlines) == nil else {
            return nil
        }
        return (code, rest)
    }

    private static func backtickValue(in value: String, prefix: String, suffix: String) -> String? {
        guard value.hasPrefix(prefix), value.hasSuffix(suffix) else { return nil }
        let start = value.index(value.startIndex, offsetBy: prefix.count)
        let end = value.index(value.endIndex, offsetBy: -suffix.count)
        let raw = String(value[start..<end]).trimmingCharacters(in: CharacterSet(charactersIn: "` "))
        return raw.isEmpty ? nil : raw
    }

    private static func taskCockpitDuration(_ seconds: Int) -> String {
        let normalized = max(0, seconds)
        if normalized >= 60, normalized % 60 == 0 {
            return format("taskCockpit.duration.minutes", "%d minutes", normalized / 60)
        }
        if normalized >= 60 {
            return format("taskCockpit.duration.minutesSeconds", "%d min %d sec", normalized / 60, normalized % 60)
        }
        return format("taskCockpit.duration.seconds", "%d seconds", normalized)
    }

    private static func localizedStrings() -> [String: String] {
        if let cachedLocalizedStrings, cachedLocalizedStrings.language == activeLanguage {
            return cachedLocalizedStrings.strings
        }

        #if SWIFT_PACKAGE
        let strings = strings(for: activeLanguage, in: .module) ?? strings(for: activeLanguage, in: .main) ?? [:]
        #else
        let strings = strings(for: activeLanguage, in: .main) ?? [:]
        #endif
        cachedLocalizedStrings = (activeLanguage, strings)
        return strings
    }

    private static func strings(for language: AppLanguage, in parent: Bundle) -> [String: String]? {
        let resourceNames = [language.rawValue, language.rawValue.lowercased()]
        guard
            let path = resourceNames.lazy.compactMap({ parent.path(forResource: "Localizable", ofType: "strings", inDirectory: "\($0).lproj") }).first,
            let dictionary = NSDictionary(contentsOfFile: path) as? [String: String]
        else {
            return nil
        }
        return dictionary
    }

    #if DEBUG
    static func localizationResourceDiagnostics(for language: AppLanguage) -> (paths: [String], count: Int) {
        let resourceNames = [language.rawValue, language.rawValue.lowercased()]
        #if SWIFT_PACKAGE
        let parents: [Bundle] = [.module, .main]
        #else
        let parents: [Bundle] = [.main]
        #endif
        let paths = parents.flatMap { parent in
            resourceNames.compactMap { parent.path(forResource: "Localizable", ofType: "strings", inDirectory: "\($0).lproj") }
        }
        #if SWIFT_PACKAGE
        let count = strings(for: language, in: .module)?.count ?? strings(for: language, in: .main)?.count ?? 0
        #else
        let count = strings(for: language, in: .main)?.count ?? 0
        #endif
        return (paths, count)
    }
    #endif
}
