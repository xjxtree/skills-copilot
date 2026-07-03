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
    static var conflicts: String { text("detail.conflicts", "Conflicts") }
    static var cleanupQueue: String { text("cleanup.queue", "Cleanup Queue") }
    static var cleanupKindFinding: String { text("cleanup.kind.finding", "Findings") }
    static var cleanupKindIntegrity: String { text("cleanup.kind.integrity", "Integrity") }
    static var cleanupKindConflict: String { text("cleanup.kind.conflict", "Same-agent conflicts") }
    static var cleanupKindAnalysis: String { text("cleanup.kind.analysis", "Analysis insights") }
    static var cleanupPriorityCritical: String { text("cleanup.priority.critical", "Critical") }
    static var cleanupPriorityHigh: String { text("cleanup.priority.high", "High") }
    static var cleanupPriorityMedium: String { text("cleanup.priority.medium", "Medium") }
    static var cleanupPriorityLow: String { text("cleanup.priority.low", "Low") }
    static var cleanupPriorityInfo: String { text("cleanup.priority.info", "Info") }
    static var cleanupFilterKind: String { text("cleanup.filter.kind", "Kind") }
    static var cleanupFilterPriority: String { text("cleanup.filter.priority", "Priority") }
    static var cleanupFilterAllKinds: String { text("cleanup.filter.allKinds", "All kinds") }
    static var cleanupFilterAllPriorities: String { text("cleanup.filter.allPriorities", "All priorities") }
    static var cleanupFilterCriticalHigh: String { text("cleanup.filter.criticalHigh", "Critical / High") }
    static var cleanupFilterLowInfo: String { text("cleanup.filter.lowInfo", "Low / Info") }
    static var cleanupUntitledItem: String { text("cleanup.item.untitled", "Cleanup item") }
    static var cleanupDefaultNextAction: String { text("cleanup.item.nextAction", "Open detail") }
    static var cleanupUnavailableFallback: String { text("cleanup.unavailableFallback", "Cleanup Queue is unavailable in this service build. Showing a local empty read-only fallback; no writes, scripts, AI provider calls, or credentials are used.") }
    static var cleanupQueueReadOnlyBoundary: String { text("cleanup.readOnlyBoundary", "Work through open findings, integrity issues, same-agent conflicts, and analysis insights from one read-only queue. Actions only open existing detail views; they do not write agent config, edit skills, execute scripts, call an AI provider, or store credentials.") }
    static var cleanupEmptyTitle: String { text("cleanup.empty.title", "No Cleanup Queue items") }
    static var cleanupEmptyMessage: String { text("cleanup.empty.message", "There are no open cleanup items for the current service response.") }
    static var cleanupNoFilteredItems: String { text("cleanup.empty.filtered", "No queue items match the selected kind, priority, and agent filters.") }
    static var cleanupAIBlocked: String { text("cleanup.safety.aiBlocked", "AI blocked") }
    static var cleanupCredentialsBlocked: String { text("cleanup.safety.credentialsBlocked", "Credentials blocked") }
    static var cleanupOpenExistingDetailHelp: String { text("cleanup.action.openExistingDetail.help", "Open the existing read-only detail section for this item.") }
    static var crossAgentComparisonTitle: String { text("comparison.crossAgent.title", "Cross-agent Comparison") }
    static var crossAgentComparisonBoundary: String { text("comparison.crossAgent.boundary", "Compare same-name or similar skills across Claude Code, Codex, opencode, Pi, Hermes, and OpenClaw by state, source, scope/root, findings, writable capability, and differences. This view is read-only: it cannot write config, edit skills, create snapshots, execute scripts, call an AI provider, or read credentials.") }
    static var crossAgentComparisonGroups: String { text("comparison.crossAgent.groups", "Groups") }
    static var crossAgentComparisonAgents: String { text("comparison.crossAgent.agents", "Agents") }
    static var crossAgentComparisonRiskGroups: String { text("comparison.crossAgent.riskGroups", "Risk groups") }
    static var crossAgentComparisonWritableMismatch: String { text("comparison.crossAgent.writableMismatch", "Writable mismatch") }
    static var crossAgentComparisonDifferences: String { text("comparison.crossAgent.differences", "Differences") }
    static var crossAgentComparisonWritable: String { text("comparison.crossAgent.writable", "Writable verified") }
    static var crossAgentComparisonUntitled: String { text("comparison.crossAgent.untitled", "Comparison group") }
    static var crossAgentComparisonMatchName: String { text("comparison.crossAgent.match.name", "Same or similar name") }
    static var crossAgentComparisonMatchSimilarName: String { text("comparison.crossAgent.match.similarName", "Similar name with definition differences") }
    static var crossAgentComparisonNoSelectedGroup: String { text("comparison.crossAgent.empty.selected", "No selected-skill comparison group") }
    static var crossAgentComparisonNoSelectedGroupMessage: String { text("comparison.crossAgent.empty.selected.message", "The selected skill does not currently share a same-name or similar cross-agent group in this catalog/filter context.") }
    static var crossAgentComparisonLocalFallback: String { text("comparison.crossAgent.localFallback", "Comparison service is unavailable in this build. Showing a local read-only catalog comparison fallback.") }
    static var crossAgentComparisonDifferenceEnabled: String { text("comparison.crossAgent.difference.enabled", "Enabled state differs") }
    static var crossAgentComparisonDifferenceWritable: String { text("comparison.crossAgent.difference.writable", "Writable capability differs") }
    static var crossAgentComparisonDifferenceSource: String { text("comparison.crossAgent.difference.source", "Source/root differs") }
    static var crossAgentComparisonDifferenceFindings: String { text("comparison.crossAgent.difference.findings", "Finding counts differ") }
    static var crossAgentComparisonDifferenceDefinition: String { text("comparison.crossAgent.difference.definition", "Definition IDs differ") }
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
    static var localReportTitle: String { text("localReport.title", "Agent Usage Report Export") }
    static var localReportBoundary: String { text("localReport.boundary", "User-triggered local redacted agent and skill usage report only. No public distribution, provider calls, credentials, script execution, config mutation, or background sync.") }
    static var localReportFormat: String { text("localReport.format", "Format") }
    static var localReportFormatMarkdown: String { text("localReport.format.markdown", "Markdown") }
    static var localReportFormatJSON: String { text("localReport.format.json", "JSON") }
    static var localReportExport: String { text("localReport.export", "Export") }
    static var localReportExporting: String { text("localReport.exporting", "Exporting usage report...") }
    static var localReportUnavailableFallback: String { text("localReport.unavailableFallback", "Local report export is unavailable in this service build. No file was written.") }
    static var localReportNoFile: String { text("localReport.noFile", "No local file") }
    static var localReportNoSections: String { text("localReport.noSections", "No section list returned") }
    static var localReportExportedSummary: String { text("localReport.exportedSummary", "Local redacted agent usage report exported.") }
    static var localReportSections: String { text("localReport.sections", "Sections") }
    static var localReportRedacted: String { text("localReport.redacted", "Redacted") }
    static var localReportNotRedactedWarning: String { text("localReport.notRedactedWarning", "Service did not mark this report as redacted. Review before sharing.") }
    static func localReportSectionTitle(_ key: String) -> String {
        switch key {
        case "current_state":
            text("localReport.section.currentState", "Current state")
        case "installed_skills":
            text("localReport.section.installedSkills", "Installed skills")
        case "issues":
            text("localReport.section.issues", "Issues")
        case "task_preflight":
            text("localReport.section.taskPreflight", "Task Preflight")
        case "analysis_results":
            text("localReport.section.analysisResults", "Intelligent analysis")
        case "safety":
            text("localReport.section.safety", "Safety")
        default:
            key
        }
    }
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
    static var settingsNavLanguageSubtitle: String { text("settings.nav.language.subtitle", "Interface and privacy") }
    static var settingsNavProviderSubtitle: String { text("settings.nav.provider.subtitle", "Connection and Keychain") }
    static var settingsNavObservabilitySubtitle: String { text("settings.nav.observability.subtitle", "Usage and logs") }
    static var settingsNavServiceSubtitle: String { text("settings.nav.service.subtitle", "Local sidecar") }
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
    static var llmSkillAnalysis: String { text("llm.skillAnalysis", "AI Skill Analysis") }
    static var llmSkillAnalysisSelectedScope: String { text("llm.skillAnalysis.scope.selected", "Selected skill") }
    static var llmSkillAnalysisVisibleScope: String { text("llm.skillAnalysis.scope.visible", "Visible skills") }
    static var llmSkillAnalysisSafetyTitle: String { text("llm.skillAnalysis.safetyTitle", "Read-only prepare only") }
    static var llmSkillAnalysisSafetyCopy: String { text("llm.skillAnalysis.safetyCopy", "No provider call is made by default. This preview cannot write skill files or agent config, cannot execute scripts, and does not save credentials.") }
    static var llmSkillAnalysisPrepareSelected: String { text("llm.skillAnalysis.prepareSelected", "Prepare Selected") }
    static var llmSkillAnalysisPrepareVisible: String { text("llm.skillAnalysis.prepareVisible", "Prepare Visible") }
    static var llmSkillAnalysisUnavailable: String { text("llm.skillAnalysis.unavailable", "AI skill analysis prepare is unavailable in this service build; preview remains disabled and read-only.") }
    static var llmSkillAnalysisUnavailablePrompt: String { text("llm.skillAnalysis.unavailablePrompt", "Service method llm.prepareSkillAnalysis is unavailable. No provider request was prepared.") }
    static var llmSkillAnalysisUnavailableSummary: String { text("llm.skillAnalysis.unavailableSummary", "Disabled fallback preview only. No writes, no scripts, no credentials, and no provider call.") }
    static var llmSkillAnalysisPromptDraft: String { text("llm.skillAnalysis.promptDraft", "Prepared prompt draft") }
    static var llmSkillAnalysisSummaryDraft: String { text("llm.skillAnalysis.summaryDraft", "Summary draft") }
    static var llmSkillAnalysisIncludedSkills: String { text("llm.skillAnalysis.includedSkills", "Included skills") }
    static var llmSkillAnalysisExcludedMissing: String { text("llm.skillAnalysis.excludedMissing", "Excluded / missing") }
    static var llmSkillAnalysisNoDraft: String { text("llm.skillAnalysis.noDraft", "No draft text returned by the service.") }
    static var llmSkillAnalysisNoIncludedSkills: String { text("llm.skillAnalysis.noIncludedSkills", "No included skills returned.") }
    static var llmSkillAnalysisWriteBack: String { text("llm.skillAnalysis.writeBack", "Write-back") }
    static var llmSkillAnalysisScriptExecution: String { text("llm.skillAnalysis.scriptExecution", "Script execution") }
    static var llmSkillAnalysisCredentialStorage: String { text("llm.skillAnalysis.credentialStorage", "Credential storage") }
    static var llmSkillAnalysisConfirmation: String { text("llm.skillAnalysis.confirmation", "Confirmation") }
    static var llmSkillAnalysisBlocked: String { text("llm.skillAnalysis.blocked", "Blocked") }
    static var llmSkillAnalysisRequired: String { text("llm.skillAnalysis.required", "Required") }
    static var llmSkillAnalysisEnabledUnsafe: String { text("llm.skillAnalysis.enabledUnsafe", "Enabled by service") }
    static var skillQualityTitle: String { text("quality.title", "AI Skill Quality Score") }
    static var skillQualityBoundary: String { text("quality.boundary", "User-triggered, read-only scoring from local evidence. The score cannot write skill files, mutate agent config, create snapshots, change triage, execute scripts, or read credentials.") }
    static var skillQualityScoreAction: String { text("quality.action.score", "Score Quality") }
    static var skillQualityUnavailable: String { text("quality.unavailable", "Quality scoring is unavailable in this service build.") }
    static var skillQualityPromptUnavailable: String { text("quality.promptUnavailable", "Quality prompt preview is unavailable in this service build; no provider request was prepared.") }
    static var skillQualityScore: String { text("quality.score", "Score") }
    static var skillQualityBand: String { text("quality.band", "Band") }
    static var skillQualityComponents: String { text("quality.components", "Components") }
    static var skillQualityEvidence: String { text("quality.evidence", "Evidence") }
    static var skillQualityRiskNotes: String { text("quality.riskNotes", "Risk notes") }
    static var skillQualitySuggestions: String { text("quality.suggestions", "Suggested improvements") }
    static var skillQualityNoComponents: String { text("quality.empty.components", "No component scores returned.") }
    static var skillQualityNoEvidence: String { text("quality.empty.evidence", "No evidence items returned.") }
    static var skillQualityNoRisks: String { text("quality.empty.risks", "No risk notes returned.") }
    static var skillQualityNoSuggestions: String { text("quality.empty.suggestions", "No suggestions returned.") }
    static var skillQualitySafety: String { text("quality.safety", "Safety flags") }
    static var skillQualityProviderNotSent: String { text("quality.safety.providerNotSent", "Provider not sent") }
    static var skillQualityWritesBlocked: String { text("quality.safety.writesBlocked", "Writes blocked") }
    static var skillQualityScriptsBlocked: String { text("quality.safety.scriptsBlocked", "Scripts blocked") }
    static var skillQualityMutationsBlocked: String { text("quality.safety.mutationsBlocked", "Config/triage mutations blocked") }
    static var skillQualityCredentialsBlocked: String { text("quality.safety.credentialsBlocked", "Credentials blocked") }
    static var taskReadinessTitle: String { text("taskReadiness.title", "AI Task Readiness Check") }
    static var taskReadinessBoundary: String { text("taskReadiness.boundary", "User-triggered, read-only task fit check from local evidence. It cannot write skill files, mutate agent config, create snapshots, change triage, execute scripts, or read credentials.") }
    static var taskReadinessTask: String { text("taskReadiness.task", "Task") }
    static var taskReadinessTaskPlaceholder: String { text("taskReadiness.task.placeholder", "Describe the task to test against this skill") }
    static var taskReadinessCheckAction: String { text("taskReadiness.action.check", "Check Readiness") }
    static var taskReadinessTaskRequired: String { text("taskReadiness.taskRequired", "Enter a task before checking readiness.") }
    static var taskReadinessUnavailable: String { text("taskReadiness.unavailable", "Task readiness check is unavailable in this service build.") }
    static var taskReadinessPromptUnavailable: String { text("taskReadiness.promptUnavailable", "Task readiness prompt preview is unavailable in this service build; no provider request was prepared.") }
    static var taskReadinessScore: String { text("taskReadiness.score", "Readiness") }
    static var taskReadinessBand: String { text("taskReadiness.band", "Band") }
    static var taskReadinessCandidates: String { text("taskReadiness.candidates", "Candidate skills") }
    static var taskReadinessGaps: String { text("taskReadiness.gaps", "Gaps / missing capabilities") }
    static var taskReadinessBlockers: String { text("taskReadiness.blockers", "Blockers") }
    static var taskReadinessRiskNotes: String { text("taskReadiness.riskNotes", "Risk notes") }
    static var taskReadinessEvidence: String { text("taskReadiness.evidence", "Evidence") }
    static var taskReadinessNoCandidates: String { text("taskReadiness.empty.candidates", "No candidate skills returned.") }
    static var taskReadinessNoGaps: String { text("taskReadiness.empty.gaps", "No gaps returned.") }
    static var taskReadinessNoBlockers: String { text("taskReadiness.empty.blockers", "No blockers returned.") }
    static var taskReadinessNoRisks: String { text("taskReadiness.empty.risks", "No risk notes returned.") }
    static var taskReadinessNoEvidence: String { text("taskReadiness.empty.evidence", "No evidence items returned.") }
    static var crossAgentReadinessTitle: String { text("crossAgentReadiness.title", "Cross-agent Task Readiness") }
    static var crossAgentReadinessBoundary: String { text("crossAgentReadiness.boundary", "User-triggered, read-only cross-agent task fit comparison from local readiness, routing, benchmark, regression, and accuracy evidence. It cannot call a provider, write skill files, mutate agent config, create snapshots, change triage, execute scripts, read credentials, persist raw prompts/responses/traces, sync cloud data, or emit telemetry.") }
    static var crossAgentReadinessTaskPlaceholder: String { text("crossAgentReadiness.task.placeholder", "Describe a task, or reuse the current readiness/routing task") }
    static var crossAgentReadinessCompareAction: String { text("crossAgentReadiness.action.compare", "Compare Agents") }
    static var crossAgentReadinessTaskRequired: String { text("crossAgentReadiness.taskRequired", "Enter a task before comparing agents.") }
    static var crossAgentReadinessUnavailable: String { text("crossAgentReadiness.unavailable", "Cross-agent task readiness is unavailable in this service build.") }
    static var crossAgentReadinessRecommendedAgent: String { text("crossAgentReadiness.recommendedAgent", "Recommended agent") }
    static var crossAgentReadinessNoRecommendation: String { text("crossAgentReadiness.empty.recommendation", "No recommended agent returned.") }
    static var crossAgentReadinessAgents: String { text("crossAgentReadiness.agents", "Per-agent readiness") }
    static var crossAgentReadinessNoAgents: String { text("crossAgentReadiness.empty.agents", "No agent readiness rows returned.") }
    static var crossAgentReadinessReadinessScore: String { text("crossAgentReadiness.readinessScore", "Readiness") }
    static var crossAgentReadinessComparisonScore: String { text("crossAgentReadiness.comparisonScore", "Comparison") }
    static var crossAgentReadinessRoutingScore: String { text("crossAgentReadiness.routingScore", "Routing") }
    static var crossAgentReadinessBestSkill: String { text("crossAgentReadiness.bestSkill", "Best skill") }
    static var crossAgentReadinessCandidateCount: String { text("crossAgentReadiness.candidateCount", "Candidates") }
    static var crossAgentReadinessEnabledState: String { text("crossAgentReadiness.enabledState", "Enabled state") }
    static var crossAgentReadinessScopeState: String { text("crossAgentReadiness.scopeState", "Scope state") }
    static var crossAgentReadinessRiskState: String { text("crossAgentReadiness.riskState", "Risk state") }
    static var crossAgentReadinessAccuracy: String { text("crossAgentReadiness.accuracy", "Accuracy context") }
    static var crossAgentReadinessRegression: String { text("crossAgentReadiness.regression", "Regression context") }
    static var crossAgentReadinessReasons: String { text("crossAgentReadiness.reasons", "Reasons") }
    static var crossAgentReadinessNoReasons: String { text("crossAgentReadiness.empty.reasons", "No reasons returned.") }
    static var crossAgentReadinessEvidence: String { text("crossAgentReadiness.evidence", "Evidence") }
    static var crossAgentReadinessNoEvidence: String { text("crossAgentReadiness.empty.evidence", "No evidence returned.") }
    static var crossAgentReadinessGapsIssues: String { text("crossAgentReadiness.gapsIssues", "Gaps / issues") }
    static var crossAgentReadinessNoGapsIssues: String { text("crossAgentReadiness.empty.gapsIssues", "No gaps or issues returned.") }
    static var crossAgentReadinessSafetyFlags: String { text("crossAgentReadiness.safetyFlags", "Safety flags") }
    static var crossAgentReadinessNoResult: String { text("crossAgentReadiness.empty.result", "No cross-agent readiness comparison loaded.") }
    static var routingConfidenceTitle: String { text("routingConfidence.title", "AI Routing Confidence") }
    static var routingConfidenceBoundary: String { text("routingConfidence.boundary", "User-triggered, read-only route ranking from local evidence. It cannot write skill files, mutate agent config, create snapshots, change triage, execute scripts, or read credentials.") }
    static var routingConfidenceTaskPlaceholder: String { text("routingConfidence.task.placeholder", "Describe the task to rank route fit") }
    static var routingConfidenceAction: String { text("routingConfidence.action.rank", "Rank Routes") }
    static var routingConfidenceTaskRequired: String { text("routingConfidence.taskRequired", "Enter a task before ranking routes.") }
    static var routingConfidenceUnavailable: String { text("routingConfidence.unavailable", "Routing confidence is unavailable in this service build.") }
    static var routingConfidencePromptUnavailable: String { text("routingConfidence.promptUnavailable", "Routing confidence prompt preview is unavailable in this service build; no provider request was prepared.") }
    static var routingConfidenceScore: String { text("routingConfidence.score", "Confidence") }
    static var routingConfidenceBand: String { text("routingConfidence.band", "Band") }
    static var routingConfidenceRoutes: String { text("routingConfidence.routes", "Candidate routes") }
    static var routingConfidenceMatchReasons: String { text("routingConfidence.matchReasons", "Match reasons") }
    static var routingConfidenceAmbiguity: String { text("routingConfidence.ambiguity", "Ambiguity / collision warnings") }
    static var routingConfidenceWrongPick: String { text("routingConfidence.wrongPick", "Wrong-pick / miss risks") }
    static var routingConfidenceEvidence: String { text("routingConfidence.evidence", "Evidence") }
    static var routingConfidenceNoRoutes: String { text("routingConfidence.empty.routes", "No candidate routes returned.") }
    static var routingConfidenceNoReasons: String { text("routingConfidence.empty.reasons", "No match reasons returned.") }
    static var routingConfidenceNoAmbiguity: String { text("routingConfidence.empty.ambiguity", "No ambiguity warnings returned.") }
    static var routingConfidenceNoWrongPick: String { text("routingConfidence.empty.wrongPick", "No wrong-pick or miss risks returned.") }
    static var routingConfidenceNoEvidence: String { text("routingConfidence.empty.evidence", "No evidence items returned.") }
    static var routingAccuracyTitle: String { text("routingAccuracy.title", "Routing Accuracy Dashboard") }
    static var routingAccuracyBoundary: String { text("routingAccuracy.boundary", "User-triggered local trace, benchmark, and regression accuracy view. It cannot call a provider, write skill files, mutate agent config, create snapshots, change triage, execute scripts, read credentials, persist raw prompts/responses/traces, sync cloud data, or emit telemetry.") }
    static var routingAccuracyLoadAction: String { text("routingAccuracy.action.load", "Load Dashboard") }
    static var routingAccuracyUnavailable: String { text("routingAccuracy.unavailable", "Routing accuracy dashboard is unavailable in this service build.") }
    static var routingAccuracyGeneratedBy: String { text("routingAccuracy.generatedBy", "Generated by") }
    static var routingAccuracyCatalog: String { text("routingAccuracy.catalog", "Catalog") }
    static var routingAccuracyWindow: String { text("routingAccuracy.window", "Window") }
    static var routingAccuracyAvailable: String { text("routingAccuracy.available", "Available") }
    static var routingAccuracyUnavailableShort: String { text("routingAccuracy.unavailable.short", "Unavailable") }
    static var routingAccuracyHitRate: String { text("routingAccuracy.hitRate", "Hit rate") }
    static var routingAccuracyAccuracyRate: String { text("routingAccuracy.accuracyRate", "Accuracy rate") }
    static var routingAccuracyKnownOutcomeRate: String { text("routingAccuracy.knownOutcomeRate", "Known-outcome rate") }
    static var routingAccuracyMissRate: String { text("routingAccuracy.missRate", "Miss rate") }
    static var routingAccuracyWrongPickRate: String { text("routingAccuracy.wrongPickRate", "Wrong-pick rate") }
    static var routingAccuracyAmbiguousRate: String { text("routingAccuracy.ambiguousRate", "Ambiguous rate") }
    static var routingAccuracyUnknownRate: String { text("routingAccuracy.unknownRate", "Unknown rate") }
    static var routingAccuracyImports: String { text("routingAccuracy.imports", "Imports") }
    static var routingAccuracyBenchmarks: String { text("routingAccuracy.benchmarks", "Benchmarks") }
    static var routingAccuracyBenchmarkMatched: String { text("routingAccuracy.benchmarkMatched", "Benchmark matched") }
    static var routingAccuracyBenchmarkGaps: String { text("routingAccuracy.benchmarkGaps", "Benchmark gaps") }
    static var routingAccuracyMissingBenchmarks: String { text("routingAccuracy.missingBenchmarks", "Missing benchmarks") }
    static var routingAccuracyRegressions: String { text("routingAccuracy.regressions", "Regressions") }
    static var routingAccuracyAvgConfidence: String { text("routingAccuracy.avgConfidence", "Avg confidence") }
    static var routingAccuracyGaps: String { text("routingAccuracy.gaps", "Gaps") }
    static var routingAccuracyBlockers: String { text("routingAccuracy.blockers", "Blockers") }
    static var routingAccuracyAgents: String { text("routingAccuracy.agents", "Per-agent accuracy") }
    static var routingAccuracyNoAgents: String { text("routingAccuracy.empty.agents", "No agent rows returned.") }
    static var routingAccuracyHistory: String { text("routingAccuracy.history", "History") }
    static var routingAccuracyNoHistory: String { text("routingAccuracy.empty.history", "No history returned.") }
    static var routingAccuracyRecentEvidence: String { text("routingAccuracy.recentEvidence", "Recent evidence") }
    static var routingAccuracyNoEvidence: String { text("routingAccuracy.empty.evidence", "No recent evidence returned.") }
    static var routingAccuracyNoGaps: String { text("routingAccuracy.empty.gaps", "No gaps returned.") }
    static var routingAccuracyBlockerNotes: String { text("routingAccuracy.blockerNotes", "Blocker notes") }
    static var routingAccuracyNoBlockers: String { text("routingAccuracy.empty.blockers", "No blocker notes returned.") }
    static var routingAccuracySafetyFlags: String { text("routingAccuracy.safetyFlags", "Safety flags") }
    static var routingAccuracySafetyClear: String { text("routingAccuracy.safety.clear", "Read-only flags clear") }
    static var routingAccuracyRawTraceStored: String { text("routingAccuracy.safety.rawTraceStored", "Raw trace stored") }
    static var routingAccuracyCloudSync: String { text("routingAccuracy.safety.cloudSync", "Cloud sync") }
    static var routingAccuracyTelemetry: String { text("routingAccuracy.safety.telemetry", "Telemetry") }
    static var routingAccuracyPromptRequest: String { text("routingAccuracy.promptRequest", "Prompt request") }
    static var routingAccuracyNoDashboard: String { text("routingAccuracy.empty.dashboard", "No routing accuracy dashboard loaded.") }
    static var routingAccuracyDays: String { text("routingAccuracy.days", "%d days") }
    static var staleDriftTitle: String { text("staleDrift.title", "Stale / Drift Detection") }
    static var staleDriftBoundary: String { text("staleDrift.boundary", "User-triggered local stale and drift review from catalog, readiness, routing, benchmark, regression, and accuracy evidence. It cannot call a provider, write skill files, mutate agent config, create snapshots, change triage, execute scripts, read credentials, persist raw prompts/responses/traces, sync cloud data, or emit telemetry.") }
    static var staleDriftDetectAction: String { text("staleDrift.action.detect", "Detect Stale / Drift") }
    static var staleDriftUnavailable: String { text("staleDrift.unavailable", "Stale / drift detection is unavailable in this service build.") }
    static var staleDriftNoResult: String { text("staleDrift.empty.result", "No stale / drift detection loaded.") }
    static var staleDriftStale: String { text("staleDrift.stale", "Stale") }
    static var staleDriftDrift: String { text("staleDrift.drift", "Drift") }
    static var staleDriftCandidates: String { text("staleDrift.candidates", "Candidates") }
    static var staleDriftCandidate: String { text("staleDrift.candidate", "Stale / drift candidate") }
    static var staleDriftAffectedAgents: String { text("staleDrift.affectedAgents", "Affected agents") }
    static var staleDriftReadinessImpact: String { text("staleDrift.readinessImpact", "Readiness impact") }
    static var staleDriftHighRisk: String { text("staleDrift.highRisk", "High risk") }
    static var staleDriftLastSeen: String { text("staleDrift.lastSeen", "Last seen") }
    static var staleDriftReasons: String { text("staleDrift.reasons", "Reasons") }
    static var staleDriftSignals: String { text("staleDrift.signals", "Signals") }
    static var staleDriftNoCandidates: String { text("staleDrift.empty.candidates", "No stale or drift candidates returned.") }
    static var staleDriftNoReadinessImpact: String { text("staleDrift.empty.readinessImpact", "No readiness impact rows returned.") }
    static var staleDriftNoReasons: String { text("staleDrift.empty.reasons", "No reasons returned.") }
    static var staleDriftNoSignals: String { text("staleDrift.empty.signals", "No signals returned.") }
    static var staleDriftSafetyFlags: String { text("staleDrift.safetyFlags", "Safety flags") }
    static var knowledgeTitle: String { text("knowledge.title", "Local Knowledge Index") }
    static var knowledgeBoundary: String { text("knowledge.boundary", "User-triggered, read-only local search across skill purpose, metadata, tags, rules, tools, and evidence. It cannot call a provider, write skill files, mutate agent config, create snapshots, change triage, execute scripts, read credentials, sync cloud data, or emit telemetry.") }
    static var knowledgeQuery: String { text("knowledge.query", "Knowledge query") }
    static var knowledgeQueryPlaceholder: String { text("knowledge.query.placeholder", "Search purpose, tools, rules, tags, or evidence") }
    static var knowledgeSearchAction: String { text("knowledge.action.search", "Search Knowledge") }
    static var knowledgeQueryRequired: String { text("knowledge.queryRequired", "Enter a query before searching the local knowledge index.") }
    static var knowledgeUnavailable: String { text("knowledge.unavailable", "Local knowledge search is unavailable in this service build.") }
    static var knowledgeNoResult: String { text("knowledge.empty.result", "No knowledge search loaded.") }
    static var knowledgeNoRows: String { text("knowledge.empty.rows", "No knowledge rows returned.") }
    static var knowledgeRows: String { text("knowledge.rows", "Knowledge rows") }
    static var knowledgeMatches: String { text("knowledge.matches", "Matches") }
    static var knowledgeMatchedFields: String { text("knowledge.matchedFields", "Matched fields") }
    static var knowledgeKeywords: String { text("knowledge.keywords", "Keywords") }
    static var knowledgeTools: String { text("knowledge.tools", "Tools") }
    static var knowledgeRules: String { text("knowledge.rules", "Rules") }
    static var knowledgeCapabilities: String { text("knowledge.capabilities", "Capabilities") }
    static var knowledgeRisks: String { text("knowledge.risks", "Risk tags") }
    static var knowledgeFacets: String { text("knowledge.facets", "Facets") }
    static var knowledgeFacet: String { text("knowledge.facet", "Facet") }
    static var knowledgeNoFacets: String { text("knowledge.empty.facets", "No facets returned.") }
    static var knowledgeGapNotes: String { text("knowledge.gapNotes", "Gap notes") }
    static var knowledgeBlockerNotes: String { text("knowledge.blockerNotes", "Blocker notes") }
    static var knowledgeSafetyFlags: String { text("knowledge.safetyFlags", "Safety flags") }
    static var localSkillMapTitle: String { text("localSkillMap.title", "Local Skill Map") }
    static var localSkillMapBoundary: String { text("localSkillMap.boundary", "User-triggered, read-only local map of skill relationships, domains, gaps, blockers, and evidence. It cannot call a provider, write skill files, mutate agent config, create snapshots, change triage, execute scripts, read credentials, sync cloud data, or emit telemetry.") }
    static var localSkillMapAction: String { text("localSkillMap.action.build", "Build Map") }
    static var localSkillMapUnavailable: String { text("localSkillMap.unavailable", "Local skill map is unavailable in this service build.") }
    static var localSkillMapNoResult: String { text("localSkillMap.empty.result", "No local skill map loaded.") }
    static var localSkillMapNodes: String { text("localSkillMap.nodes", "Map nodes") }
    static var localSkillMapEdges: String { text("localSkillMap.edges", "Map edges") }
    static var localSkillMapClusters: String { text("localSkillMap.clusters", "Clusters / domains") }
    static var localSkillMapNoNodes: String { text("localSkillMap.empty.nodes", "No map nodes returned.") }
    static var localSkillMapNoEdges: String { text("localSkillMap.empty.edges", "No map edges returned.") }
    static var localSkillMapNoClusters: String { text("localSkillMap.empty.clusters", "No clusters or domains returned.") }
    static var localSkillMapSelectedContext: String { text("localSkillMap.selectedContext", "Selected skill context") }
    static var localSkillMapRelation: String { text("localSkillMap.relation", "Relation") }
    static var localSkillMapStrength: String { text("localSkillMap.strength", "Strength") }
    static var localSkillMapNodeIDs: String { text("localSkillMap.nodeIDs", "Node IDs") }
    static var localSkillMapDirection: String { text("localSkillMap.direction", "Direction") }
    static var skillLifecycleTimelineTitle: String { text("skillLifecycleTimeline.title", "Skill Lifecycle Timeline") }
    static var skillLifecycleTimelineBoundary: String { text("skillLifecycleTimeline.boundary", "User-triggered, deterministic, read-only lifecycle timeline from existing local catalog, scan, finding, routing, session, provider-observability, remediation, and provenance evidence. It cannot send provider requests, write skill files, mutate agent config, create snapshots, change triage, execute scripts, read credentials, persist raw prompts/responses/traces, sync cloud data, or emit telemetry.") }
    static var skillLifecycleTimelineAction: String { text("skillLifecycleTimeline.action.load", "Load Timeline") }
    static var skillLifecycleTimelineUnavailable: String { text("skillLifecycleTimeline.unavailable", "Skill lifecycle timeline is unavailable in this service build.") }
    static var skillLifecycleTimelineNoResult: String { text("skillLifecycleTimeline.empty.result", "No skill lifecycle timeline loaded.") }
    static var skillLifecycleTimelineEvents: String { text("skillLifecycleTimeline.events", "Timeline events") }
    static var skillLifecycleTimelineSkillRows: String { text("skillLifecycleTimeline.skillRows", "Skill rows") }
    static var skillLifecycleTimelineAgentRows: String { text("skillLifecycleTimeline.agentRows", "Agent rows") }
    static var skillLifecycleTimelineNoRows: String { text("skillLifecycleTimeline.empty.rows", "No lifecycle rows returned.") }
    static var skillLifecycleTimelineEventTypes: String { text("skillLifecycleTimeline.eventTypes", "Event types") }
    static var skillLifecycleTimelineStages: String { text("skillLifecycleTimeline.stages", "Lifecycle stages") }
    static var skillLifecycleTimelineOccurredAt: String { text("skillLifecycleTimeline.occurredAt", "Occurred") }
    static var skillLifecycleTimelineEventType: String { text("skillLifecycleTimeline.eventType", "Event type") }
    static var skillLifecycleTimelineLifecycleStage: String { text("skillLifecycleTimeline.lifecycleStage", "Lifecycle stage") }
    static var guidedCleanupFlowTitle: String { text("guidedCleanup.title", "Guided Cleanup Flow") }
    static var guidedCleanupFlowBoundary: String { text("guidedCleanup.boundary", "User-triggered, deterministic guided cleanup from local catalog, findings, remediation, readiness, routing, lifecycle, and history evidence. Planning is read-only. Recording a guided step stores only app-local redacted cleanup metadata through cleanup.recordGuidedStep; this panel cannot apply fixes, write skill files, mutate agent config, create or roll back snapshots, change triage, execute scripts, send provider requests, read credentials, persist raw prompts/responses/traces, sync cloud data, or emit telemetry.") }
    static var guidedCleanupFlowNoWriteBoundary: String { text("guidedCleanup.noWriteBoundary", "Guidance and app-local metadata only. No Apply, Fix, Enable, Disable, Write, Snapshot, Rollback, Script, Provider Send, or Triage action is exposed here.") }
    static var guidedCleanupFlowAction: String { text("guidedCleanup.action.load", "Load Flow") }
    static var guidedCleanupFlowRecordAction: String { text("guidedCleanup.action.record", "Record Step Metadata") }
    static var guidedCleanupFlowUnavailable: String { text("guidedCleanup.unavailable", "Guided cleanup flow is unavailable in this service build.") }
    static var guidedCleanupRecordUnavailable: String { text("guidedCleanup.record.unavailable", "Recording guided cleanup metadata is unavailable in this service build.") }
    static var guidedCleanupFlowNoResult: String { text("guidedCleanup.empty.result", "No guided cleanup flow loaded.") }
    static var guidedCleanupFlowSteps: String { text("guidedCleanup.steps", "Flow steps") }
    static var guidedCleanupFlowStep: String { text("guidedCleanup.step", "Guided step") }
    static var guidedCleanupFlowNoSteps: String { text("guidedCleanup.empty.steps", "No guided steps returned.") }
    static var guidedCleanupFlowIssueGroups: String { text("guidedCleanup.issueGroups", "Issue groups") }
    static var guidedCleanupFlowIssueGroup: String { text("guidedCleanup.issueGroup", "Issue group") }
    static var guidedCleanupFlowNoIssueGroups: String { text("guidedCleanup.empty.issueGroups", "No issue groups returned.") }
    static var guidedCleanupFlowSafeActions: String { text("guidedCleanup.safeActions", "Safe actions") }
    static var guidedCleanupFlowSafeAction: String { text("guidedCleanup.safeAction", "Safe action") }
    static var guidedCleanupFlowNoSafeActions: String { text("guidedCleanup.empty.safeActions", "No safe next actions returned.") }
    static var guidedCleanupFlowRecordedSteps: String { text("guidedCleanup.recordedSteps", "Recorded steps") }
    static var guidedCleanupFlowRecordedStep: String { text("guidedCleanup.recordedStep", "Recorded step") }
    static var guidedCleanupFlowNoRecordedSteps: String { text("guidedCleanup.empty.recordedSteps", "No recorded guided cleanup steps returned.") }
    static var guidedCleanupFlowRecommended: String { text("guidedCleanup.recommended", "Recommended") }
    static var guidedCleanupFlowOrder: String { text("guidedCleanup.order", "Order") }
    static var guidedCleanupFlowRecordGuidance: String { text("guidedCleanup.recordGuidance", "Record that this step was reviewed as app-local metadata only.") }
    static var guidedCleanupFlowRecordResult: String { text("guidedCleanup.record.result", "Guided cleanup record") }
    static var guidedCleanupFlowRecordDefaultNote: String { text("guidedCleanup.record.defaultNote", "Recorded from native Guided Cleanup Flow as app-local redacted metadata only; no cleanup was applied.") }
    static var guidedCleanupFlowAppLocalOnly: String { text("guidedCleanup.appLocalOnly", "App-local only") }
    static var guidedCleanupFlowMetadataRedacted: String { text("guidedCleanup.metadataRedacted", "Metadata redacted") }
    static var guidedCleanupFlowPreviewOnly: String { text("guidedCleanup.previewOnly", "Preview only") }
    static var guidedCleanupFlowExistingSafeEntry: String { text("guidedCleanup.existingSafeEntry", "Existing safe entry") }
    static var guidedCleanupFlowCanApplyFix: String { text("guidedCleanup.canApplyFix", "Can apply fix") }
    static var guidedCleanupSafeActionEntryMethod: String { text("guidedCleanup.safeAction.entryMethod", "Entry method") }
    static var guidedCleanupSafeActionPreviewRequired: String { text("guidedCleanup.safeAction.previewRequired", "Preview required") }
    static var guidedCleanupSafeActionConfirmationRequired: String { text("guidedCleanup.safeAction.confirmationRequired", "Confirmation required") }
    static var guidedCleanupSafeLinkOpen: String { text("guidedCleanup.safeLink.open", "Open safe entry") }
    static var guidedCleanupSafeLinkConfirmOpen: String { text("guidedCleanup.safeLink.confirmOpen", "Confirm open") }
    static var guidedCleanupSafeLinkCancelOpen: String { text("guidedCleanup.safeLink.cancelOpen", "Cancel") }
    static var guidedCleanupSafeLinkApplyBlocked: String { text("guidedCleanup.safeLink.applyBlocked", "Guided cleanup links cannot apply changes.") }
    static var guidedCleanupSafeLinkHelp: String { text("guidedCleanup.safeLink.help", "Open an existing safe preview or read-only review entry.") }
    static var guidedCleanupSafeLinkTarget: String { text("guidedCleanup.safeLink.target", "Safe link target") }
    static var guidedCleanupSafeLinkTrigger: String { text("guidedCleanup.safeLink.trigger", "Safe link trigger") }
    static var providerObservabilityTitle: String { text("providerObservability.title", "Provider Observability") }
    static var providerObservabilityBoundary: String { text("providerObservability.boundary", "User-triggered, deterministic, read-only dashboard from redacted app-local prompt-run and provider-call metadata. It does not send provider requests, read credentials, expose raw prompts or responses, write files, mutate agent config, create snapshots, execute scripts, sync cloud data, or emit telemetry.") }
    static var providerObservabilityAction: String { text("providerObservability.action.build", "Build Observability") }
    static var providerObservabilityUnavailable: String { text("providerObservability.unavailable", "Provider observability is unavailable in this service build.") }
    static var providerObservabilityNoResult: String { text("providerObservability.empty.result", "No provider observability dashboard loaded.") }
    static var providerObservabilitySettingsMode: String { text("providerObservability.settings.mode", "Observability view") }
    static var providerObservabilityDashboard: String { text("providerObservability.settings.dashboard", "Dashboard") }
    static var providerObservabilityLogs: String { text("providerObservability.settings.logs", "Logs") }
    static var providerObservabilityIssuesOnly: String { text("providerObservability.settings.issuesOnly", "Issues only") }
    static var providerObservabilityNoFilteredCalls: String { text("providerObservability.empty.filteredCalls", "No provider logs match the current filters.") }
    static var providerObservabilityCalls: String { text("providerObservability.calls", "Calls") }
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
    static var providerObservabilityChartStatus: String { text("providerObservability.chart.status", "Call status") }
    static var providerObservabilityChartModelTokens: String { text("providerObservability.chart.modelTokens", "Model tokens") }
    static var providerObservabilityChartDestinationCost: String { text("providerObservability.chart.destinationCost", "Destination cost") }
    static var providerObservabilityChartModelLatency: String { text("providerObservability.chart.modelLatency", "Model latency") }
    static var providerObservabilityChartModelTaskConfidence: String { text("providerObservability.chart.modelTaskConfidence", "Model-task fit") }
    static var providerObservabilityChartEmpty: String { text("providerObservability.chart.empty", "No chart data") }
    static var taskCockpitTitle: String { text("taskCockpit.title", "Task Preflight") }
    static var taskCockpitBoundary: String { text("taskCockpit.boundary", "Read-only local preflight: decide whether the task is ready to hand off, which agent/skill fits, and what must be clarified first.") }
    static var taskCockpitReadOnlyFootnote: String { text("taskCockpit.readOnlyFootnote", "Read-only preflight: no provider call, config write, or script execution.") }
    static var taskCockpitAction: String { text("taskCockpit.action.build", "Build Preflight") }
    static var taskCockpitRetry: String { text("taskCockpit.action.retry", "Retry") }
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
    static var taskCockpitSessionContext: String { text("taskCockpit.sessionContext", "Session-review context") }
    static var taskCockpitProviderContext: String { text("taskCockpit.providerContext", "Provider-observability context") }
    static var taskCockpitRemediationContext: String { text("taskCockpit.remediationContext", "Remediation context") }
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
    static var taskCockpitProgressTitle: String { text("taskCockpit.progress.title", "Progressive feedback") }
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
    static var similarGroupingTitle: String { text("similarGrouping.title", "Similar Skill Grouping") }
    static var similarGroupingBoundary: String { text("similarGrouping.boundary", "User-triggered, read-only local grouping for duplicate, similar, and confusable skills across catalog evidence. It cannot call a provider, write skill files, mutate agent config, create snapshots, change triage, execute scripts, read credentials, persist raw prompts/responses/traces, sync cloud data, or emit telemetry.") }
    static var similarGroupingAction: String { text("similarGrouping.action.group", "Group Similar Skills") }
    static var similarGroupingUnavailable: String { text("similarGrouping.unavailable", "Similar skill grouping is unavailable in this service build.") }
    static var similarGroupingNoResult: String { text("similarGrouping.empty.result", "No similar skill grouping loaded.") }
    static var similarGroupingNoGroups: String { text("similarGrouping.empty.groups", "No similar skill groups returned.") }
    static var similarGroupingGroups: String { text("similarGrouping.groups", "Groups") }
    static var similarGroupingGroup: String { text("similarGrouping.group", "Similar group") }
    static var similarGroupingMembers: String { text("similarGrouping.members", "Members") }
    static var similarGroupingDuplicate: String { text("similarGrouping.type.duplicate", "Duplicate") }
    static var similarGroupingSimilar: String { text("similarGrouping.type.similar", "Similar") }
    static var similarGroupingConfusable: String { text("similarGrouping.type.confusable", "Confusable") }
    static var similarGroupingHighAmbiguity: String { text("similarGrouping.highAmbiguity", "High ambiguity") }
    static var similarGroupingCoverageRedundancy: String { text("similarGrouping.coverageRedundancy", "Coverage redundancy") }
    static var similarGroupingRoutingAmbiguity: String { text("similarGrouping.routingAmbiguity", "Routing ambiguity") }
    static var similarGroupingWhyGrouped: String { text("similarGrouping.whyGrouped", "Why grouped") }
    static var similarGroupingSharedTerms: String { text("similarGrouping.sharedTerms", "Shared terms") }
    static var similarGroupingSourceSignals: String { text("similarGrouping.sourceSignals", "Source signals") }
    static var similarGroupingQuality: String { text("similarGrouping.quality", "Quality") }
    static var similarGroupingReadiness: String { text("similarGrouping.readiness", "Readiness") }
    static var similarGroupingStaleDrift: String { text("similarGrouping.staleDrift", "Stale / drift") }
    static var capabilityTaxonomyTitle: String { text("capabilityTaxonomy.title", "Capability Taxonomy") }
    static var capabilityTaxonomyBoundary: String { text("capabilityTaxonomy.boundary", "User-triggered, read-only local taxonomy for capability domains, coverage, gaps, blockers, representative skills, and evidence. It cannot call a provider, write skill files, mutate agent config, create snapshots, change triage, execute scripts, read credentials, persist raw prompts/responses/traces, sync cloud data, or emit telemetry.") }
    static var capabilityTaxonomyAction: String { text("capabilityTaxonomy.action.build", "Build Taxonomy") }
    static var capabilityTaxonomyUnavailable: String { text("capabilityTaxonomy.unavailable", "Capability taxonomy is unavailable in this service build.") }
    static var capabilityTaxonomyNoResult: String { text("capabilityTaxonomy.empty.result", "No capability taxonomy loaded.") }
    static var capabilityTaxonomyNoDomains: String { text("capabilityTaxonomy.empty.domains", "No capability domains returned.") }
    static var capabilityTaxonomyDomains: String { text("capabilityTaxonomy.domains", "Domains") }
    static var capabilityTaxonomyDomain: String { text("capabilityTaxonomy.domain", "Capability domain") }
    static var capabilityTaxonomyCapability: String { text("capabilityTaxonomy.capability", "Capability") }
    static var capabilityTaxonomyCoverage: String { text("capabilityTaxonomy.coverage", "Coverage") }
    static var capabilityTaxonomyAgentCoverage: String { text("capabilityTaxonomy.agentCoverage", "Agent coverage") }
    static var capabilityTaxonomyRepresentativeSkills: String { text("capabilityTaxonomy.representativeSkills", "Representative skills") }
    static var workspaceReadinessTitle: String { text("workspaceReadiness.title", "Workspace Readiness") }
    static var workspaceReadinessBoundary: String { text("workspaceReadiness.boundary", "User-triggered, read-only local workspace readiness check for expected work, enabled/scoped skills, agent coverage, capability gaps, blockers, and evidence. It cannot call a provider, write skill files, mutate agent config, create snapshots, change triage, execute scripts, read credentials, persist raw prompts/responses/traces, sync cloud data, or emit telemetry.") }
    static var workspaceReadinessAction: String { text("workspaceReadiness.action.check", "Check Workspace") }
    static var workspaceReadinessUnavailable: String { text("workspaceReadiness.unavailable", "Workspace readiness is unavailable in this service build.") }
    static var workspaceReadinessNoResult: String { text("workspaceReadiness.empty.result", "No workspace readiness check loaded.") }
    static var workspaceReadinessChecklist: String { text("workspaceReadiness.checklist", "Readiness checklist") }
    static var workspaceReadinessNoChecklist: String { text("workspaceReadiness.empty.checklist", "No checklist rows returned.") }
    static var workspaceReadinessChecklistItem: String { text("workspaceReadiness.checklist.item", "Readiness check") }
    static var workspaceReadinessAgentRows: String { text("workspaceReadiness.agents", "Agent readiness") }
    static var workspaceReadinessNoAgentRows: String { text("workspaceReadiness.empty.agents", "No agent readiness rows returned.") }
    static var workspaceReadinessCapabilityRows: String { text("workspaceReadiness.capabilities", "Capability readiness") }
    static var workspaceReadinessNoCapabilityRows: String { text("workspaceReadiness.empty.capabilities", "No capability readiness rows returned.") }
    static var workspaceReadinessOverall: String { text("workspaceReadiness.overall", "Overall") }
    static var workspaceReadinessReady: String { text("workspaceReadiness.ready", "Ready") }
    static var workspaceReadinessPartial: String { text("workspaceReadiness.partial", "Partial") }
    static var workspaceReadinessBlocked: String { text("workspaceReadiness.blocked", "Blocked") }
    static var workspaceReadinessRequired: String { text("workspaceReadiness.required", "Required") }
    static var workspaceReadinessMatched: String { text("workspaceReadiness.matched", "Matched") }
    static var workspaceReadinessEnabled: String { text("workspaceReadiness.enabled", "Enabled") }
    static var remediationPlanTitle: String { text("remediationPlan.title", "AI Remediation Planner") }
    static var remediationPlanBoundary: String { text("remediationPlan.boundary", "User-triggered, local-only, deterministic remediation planning from findings, gaps, routing ambiguity, stale/drift, readiness, taxonomy, workspace, and evidence signals. It is guidance-only: it cannot call a provider, write skill files, mutate agent config, create snapshots, change triage, execute scripts, read credentials, persist raw prompts/responses/traces, sync cloud data, or emit telemetry.") }
    static var remediationPlanAction: String { text("remediationPlan.action.plan", "Plan Remediation") }
    static var remediationPlanUnavailable: String { text("remediationPlan.unavailable", "Remediation planning is unavailable in this service build.") }
    static var remediationPlanNoResult: String { text("remediationPlan.empty.result", "No remediation plan loaded.") }
    static var remediationPlanItem: String { text("remediationPlan.item", "Remediation item") }
    static var remediationPlanItems: String { text("remediationPlan.items", "Plan items") }
    static var remediationPlanNoItems: String { text("remediationPlan.empty.items", "No remediation plan items returned.") }
    static var remediationPlanPriorities: String { text("remediationPlan.priorities", "Priority rows") }
    static var remediationPlanNoPriorities: String { text("remediationPlan.empty.priorities", "No priority rows returned.") }
    static var remediationPlanCritical: String { text("remediationPlan.critical", "Critical") }
    static var remediationPlanQuickWins: String { text("remediationPlan.quickWins", "Quick wins") }
    static var remediationPlanAmbiguity: String { text("remediationPlan.ambiguity", "Ambiguity") }
    static var remediationPlanDrift: String { text("remediationPlan.drift", "Stale / drift") }
    static var remediationPlanCategory: String { text("remediationPlan.category", "Category") }
    static var remediationPlanGuidanceOnly: String { text("remediationPlan.guidanceOnly", "Guidance only") }
    static var remediationPlanNextArea: String { text("remediationPlan.nextArea", "Review area") }
    static var remediationPlanReviewGuidance: String { text("remediationPlan.reviewGuidance", "Review the supporting evidence in existing safe UI areas; no direct write action is available from this plan.") }
    static var fixPreviewTitle: String { text("fixPreview.title", "Fix Preview Drafts") }
    static var fixPreviewBoundary: String { text("fixPreview.boundary", "User-triggered, local-only draft previews for likely skill fixes. Drafts are copy-only guidance: this panel cannot call a provider, write skill files, mutate agent config, create snapshots, change triage, execute scripts, read credentials, persist raw prompts/responses/traces, sync cloud data, or emit telemetry.") }
    static var fixPreviewCopyOnlyBoundary: String { text("fixPreview.copyOnlyBoundary", "Copy proposed text into an existing safe edit flow if you choose to use it. No Apply or Write action is exposed here.") }
    static var fixPreviewAction: String { text("fixPreview.action.preview", "Preview Drafts") }
    static var fixPreviewUnavailable: String { text("fixPreview.unavailable", "Fix preview drafts are unavailable in this service build.") }
    static var fixPreviewNoResult: String { text("fixPreview.empty.result", "No fix preview drafts loaded.") }
    static var fixPreviewDraft: String { text("fixPreview.draft", "Fix draft") }
    static var fixPreviewDrafts: String { text("fixPreview.drafts", "Drafts") }
    static var fixPreviewNoDrafts: String { text("fixPreview.empty.drafts", "No fix preview drafts returned.") }
    static var fixPreviewFrontmatter: String { text("fixPreview.type.frontmatter", "Frontmatter") }
    static var fixPreviewDescription: String { text("fixPreview.type.description", "Description") }
    static var fixPreviewPermissions: String { text("fixPreview.type.permissions", "Permissions") }
    static var fixPreviewDependency: String { text("fixPreview.type.dependency", "Dependency") }
    static var fixPreviewPolicy: String { text("fixPreview.type.policy", "Policy") }
    static var fixPreviewDraftType: String { text("fixPreview.draftType", "Draft type") }
    static var fixPreviewFinding: String { text("fixPreview.finding", "Finding") }
    static var fixPreviewCurrentSnippet: String { text("fixPreview.currentSnippet", "Current snippet") }
    static var fixPreviewProposedSnippet: String { text("fixPreview.proposedSnippet", "Proposed draft") }
    static var fixPreviewCopyDraft: String { text("fixPreview.copyDraft", "Copy Draft") }
    static var fixPreviewEditGuidanceFallback: String { text("fixPreview.editGuidance.fallback", "Review this draft in the relevant existing editor or source file; this preview does not apply changes.") }
    static var impactPreviewTitle: String { text("impactPreview.title", "Impact Preview") }
    static var impactPreviewBoundary: String { text("impactPreview.boundary", "User-triggered, local-only impact preview for remediation work. It estimates task, agent, skill, risk, and rollback effects from deterministic local evidence only; this panel cannot call a provider, write skill files, mutate agent config, create snapshots, change triage, execute scripts, read credentials, persist raw prompts/responses/traces, sync cloud data, or emit telemetry.") }
    static var impactPreviewNoWriteBoundary: String { text("impactPreview.noWriteBoundary", "Preview impact only. No Apply, Confirm, Write, Snapshot, or Rollback action is exposed here.") }
    static var impactPreviewAction: String { text("impactPreview.action.preview", "Preview Impact") }
    static var impactPreviewUnavailable: String { text("impactPreview.unavailable", "Impact preview is unavailable in this service build.") }
    static var impactPreviewNoResult: String { text("impactPreview.empty.result", "No impact preview loaded.") }
    static var impactPreviewImpact: String { text("impactPreview.impact", "Impact") }
    static var impactPreviewImpacts: String { text("impactPreview.impacts", "Impacts") }
    static var impactPreviewNoImpacts: String { text("impactPreview.empty.impacts", "No general impact rows returned.") }
    static var impactPreviewTaskImpacts: String { text("impactPreview.taskImpacts", "Task impacts") }
    static var impactPreviewNoTaskImpacts: String { text("impactPreview.empty.taskImpacts", "No task impact rows returned.") }
    static var impactPreviewAgentImpacts: String { text("impactPreview.agentImpacts", "Agent impacts") }
    static var impactPreviewNoAgentImpacts: String { text("impactPreview.empty.agentImpacts", "No agent impact rows returned.") }
    static var impactPreviewSkillImpacts: String { text("impactPreview.skillImpacts", "Skill impacts") }
    static var impactPreviewNoSkillImpacts: String { text("impactPreview.empty.skillImpacts", "No skill impact rows returned.") }
    static var impactPreviewRiskDeltas: String { text("impactPreview.riskDeltas", "Risk deltas") }
    static var impactPreviewNoRiskDeltas: String { text("impactPreview.empty.riskDeltas", "No risk delta rows returned.") }
    static var impactPreviewSnapshotRollback: String { text("impactPreview.snapshotRollback", "Snapshot / rollback") }
    static var impactPreviewNoSnapshotRollback: String { text("impactPreview.empty.snapshotRollback", "No snapshot or rollback rows returned.") }
    static var impactPreviewNoWrite: String { text("impactPreview.noWrite", "No-write flags") }
    static var impactPreviewBefore: String { text("impactPreview.before", "Before") }
    static var impactPreviewAfter: String { text("impactPreview.after", "After") }
    static var impactPreviewDelta: String { text("impactPreview.delta", "Delta") }
    static var remediationBatchReviewTitle: String { text("batchReview.title", "Batch Review Workflow") }
    static var remediationBatchReviewBoundary: String { text("batchReview.boundary", "User-triggered, local-only batch review workflow for remediation candidates. It groups task, risk, rule, agent, and workspace review items from deterministic local evidence; this panel cannot call a provider, write skill files, mutate agent config, create snapshots, change triage, execute scripts, read credentials, persist raw prompts/responses/traces, sync cloud data, or emit telemetry.") }
    static var remediationBatchReviewNoWriteBoundary: String { text("batchReview.noWriteBoundary", "Review workflow only. No Apply, Confirm, Write, Snapshot, Rollback, Script, or Provider Send action is exposed here.") }
    static var remediationBatchReviewAction: String { text("batchReview.action.review", "Review Batch") }
    static var remediationBatchReviewUnavailable: String { text("batchReview.unavailable", "Batch review workflow is unavailable in this service build.") }
    static var remediationBatchReviewNoResult: String { text("batchReview.empty.result", "No batch review loaded.") }
    static var remediationBatchReviewControls: String { text("batchReview.controls", "Review controls") }
    static var remediationBatchReviewControlTask: String { text("batchReview.control.task", "Task") }
    static var remediationBatchReviewControlRisk: String { text("batchReview.control.risk", "Risk") }
    static var remediationBatchReviewControlRule: String { text("batchReview.control.rule", "Rule") }
    static var remediationBatchReviewControlAgent: String { text("batchReview.control.agent", "Agent") }
    static var remediationBatchReviewControlWorkspace: String { text("batchReview.control.workspace", "Workspace") }
    static var remediationBatchReviewControlBlocked: String { text("batchReview.control.blocked", "Show blockers") }
    static var remediationBatchReviewGroups: String { text("batchReview.groups", "Review groups") }
    static var remediationBatchReviewNoGroups: String { text("batchReview.empty.groups", "No review groups returned.") }
    static var remediationBatchReviewItems: String { text("batchReview.items", "Review items") }
    static var remediationBatchReviewNoItems: String { text("batchReview.empty.items", "No review items returned.") }
    static var remediationBatchReviewGroup: String { text("batchReview.group", "Review group") }
    static var remediationBatchReviewItem: String { text("batchReview.item", "Review item") }
    static var remediationBatchReviewDimensions: String { text("batchReview.dimensions", "Dimensions") }
    static var remediationBatchReviewRiskLevels: String { text("batchReview.riskLevels", "Risk levels") }
    static var remediationBatchReviewRuleIDs: String { text("batchReview.ruleIDs", "Rules") }
    static var remediationBatchReviewSafeNextSteps: String { text("batchReview.safeNextSteps", "Safe next steps") }
    static var remediationBatchReviewSafeNextStep: String { text("batchReview.safeNextStep", "Safe next step") }
    static var remediationBatchReviewSafeNextStepFallback: String { text("batchReview.safeNextStep.fallback", "Open the relevant existing safe review area") }
    static var remediationBatchReviewPreviewOnly: String { text("batchReview.previewOnly", "Preview only") }
    static var remediationBatchReviewReviewArea: String { text("batchReview.reviewArea", "Review area") }
    static var remediationBatchReviewTaskRows: String { text("batchReview.taskRows", "Task rows") }
    static var remediationBatchReviewRiskRows: String { text("batchReview.riskRows", "Risk rows") }
    static var remediationBatchReviewRuleRows: String { text("batchReview.ruleRows", "Rule rows") }
    static var remediationBatchReviewAgentRows: String { text("batchReview.agentRows", "Agent rows") }
    static var remediationBatchReviewWorkspaceRows: String { text("batchReview.workspaceRows", "Workspace rows") }
    static var remediationHistoryTitle: String { text("remediationHistory.title", "Remediation History") }
    static var remediationHistoryBoundary: String { text("remediationHistory.boundary", "User-triggered, app-local remediation history for review/audit metadata only. Loading history is read-only; recording history stores local audit metadata through the service, but this panel cannot apply remediation, write skill files, mutate agent config, create or roll back snapshots, change triage, execute scripts, send provider requests, read credentials, persist raw prompts/responses/traces, sync cloud data, or emit telemetry.") }
    static var remediationHistoryNoWriteBoundary: String { text("remediationHistory.noWriteBoundary", "Local audit only. No Apply, Remediate, Write, Snapshot, Rollback, Script, Provider Send, or Triage action is exposed here.") }
    static var remediationHistoryLoadAction: String { text("remediationHistory.action.load", "Load History") }
    static var remediationHistoryRecordAction: String { text("remediationHistory.action.record", "Record Local Audit") }
    static var remediationHistoryUnavailable: String { text("remediationHistory.unavailable", "Remediation history is unavailable in this service build.") }
    static var remediationHistoryRecordUnavailable: String { text("remediationHistory.record.unavailable", "Recording remediation history is unavailable in this service build.") }
    static var remediationHistoryNoResult: String { text("remediationHistory.empty.result", "No remediation history loaded.") }
    static var remediationHistoryRecords: String { text("remediationHistory.records", "History records") }
    static var remediationHistoryNoRecords: String { text("remediationHistory.empty.records", "No remediation history records returned.") }
    static var remediationHistoryRecord: String { text("remediationHistory.record", "History record") }
    static var remediationHistoryRecorded: String { text("remediationHistory.recorded", "Recorded") }
    static var remediationHistoryRecurrence: String { text("remediationHistory.recurrence", "Recurrence") }
    static var remediationHistoryReopened: String { text("remediationHistory.reopened", "Reopened") }
    static var remediationHistoryReadinessImprovement: String { text("remediationHistory.readinessImprovement", "Readiness improvement") }
    static var remediationHistoryDecisions: String { text("remediationHistory.decisions", "Decisions") }
    static var remediationHistoryStatuses: String { text("remediationHistory.statuses", "Statuses") }
    static var remediationHistoryDecision: String { text("remediationHistory.decision", "Decision") }
    static var remediationHistoryDecisionReviewed: String { text("remediationHistory.decision.reviewed", "Reviewed") }
    static var remediationHistoryStatusRecorded: String { text("remediationHistory.status.recorded", "Recorded") }
    static var remediationHistoryRecordedAt: String { text("remediationHistory.recordedAt", "Recorded at") }
    static var remediationHistoryUpdatedAt: String { text("remediationHistory.updatedAt", "Updated at") }
    static var remediationHistorySourceMethod: String { text("remediationHistory.sourceMethod", "Source method") }
    static var remediationHistoryRecordResult: String { text("remediationHistory.record.result", "Record result") }
    static var remediationHistoryRecordDefaultNote: String { text("remediationHistory.record.defaultNote", "Recorded from native Analysis as app-local remediation audit metadata only; no remediation was applied.") }
    static var taskBenchmarkTitle: String { text("taskBenchmark.title", "Task Benchmark Set") }
    static var taskBenchmarkBoundary: String { text("taskBenchmark.boundary", "User-triggered, local benchmark evaluation for task routing. Local evaluation does not call a provider and cannot write skill files, mutate agent config, create snapshots, change triage, execute scripts, or read credentials.") }
    static var taskBenchmarkTaskPlaceholder: String { text("taskBenchmark.task.placeholder", "Optional benchmark task text; otherwise the current readiness/routing task is used") }
    static var taskBenchmarkSaveAction: String { text("taskBenchmark.action.save", "Save Benchmark") }
    static var taskBenchmarkLoadAction: String { text("taskBenchmark.action.load", "Load Benchmarks") }
    static var taskBenchmarkEvaluateAction: String { text("taskBenchmark.action.evaluate", "Evaluate Set") }
    static var taskBenchmarkDeleteAction: String { text("taskBenchmark.action.delete", "Delete benchmark") }
    static var taskBenchmarkTaskRequired: String { text("taskBenchmark.taskRequired", "Enter a task before saving a benchmark.") }
    static var taskBenchmarkUnavailable: String { text("taskBenchmark.unavailable", "Task benchmark set is unavailable in this service build.") }
    static var taskBenchmarkDeleteUnavailable: String { text("taskBenchmark.deleteUnavailable", "Deleting benchmarks is unavailable in this service build.") }
    static var taskBenchmarkSuccessCriterion: String { text("taskBenchmark.successCriterion", "Top route should match the selected expected skill or an acceptable local agent/scope route.") }
    static var taskBenchmarkListTitle: String { text("taskBenchmark.list", "Benchmarks") }
    static var taskBenchmarkNoBenchmarks: String { text("taskBenchmark.empty.benchmarks", "No benchmarks returned.") }
    static var taskBenchmarkEvaluationTitle: String { text("taskBenchmark.evaluation", "Benchmark evaluation") }
    static var taskBenchmarkAverageScore: String { text("taskBenchmark.averageScore", "Average") }
    static var taskBenchmarkEvaluated: String { text("taskBenchmark.evaluated", "Evaluated") }
    static var taskBenchmarkMatched: String { text("taskBenchmark.matched", "Expected matched") }
    static var taskBenchmarkAcceptableMatched: String { text("taskBenchmark.acceptableMatched", "Acceptable matched") }
    static var taskBenchmarkPerBenchmark: String { text("taskBenchmark.perBenchmark", "Per-benchmark results") }
    static var taskBenchmarkNoEvaluations: String { text("taskBenchmark.empty.evaluations", "No benchmark evaluations returned.") }
    static var taskBenchmarkTopRoute: String { text("taskBenchmark.topRoute", "Top route") }
    static var taskBenchmarkExpected: String { text("taskBenchmark.expected", "Expected") }
    static var taskBenchmarkAcceptable: String { text("taskBenchmark.acceptable", "Acceptable") }
    static var taskBenchmarkExpectedCovered: String { text("taskBenchmark.expected.covered", "Expected covered") }
    static var taskBenchmarkExpectedMissed: String { text("taskBenchmark.expected.missed", "Expected missed") }
    static var taskBenchmarkAcceptableCovered: String { text("taskBenchmark.acceptable.covered", "Acceptable covered") }
    static var taskBenchmarkAcceptableMissed: String { text("taskBenchmark.acceptable.missed", "Acceptable missed") }
    static var taskBenchmarkBlockers: String { text("taskBenchmark.blockers", "Blockers") }
    static var taskBenchmarkGaps: String { text("taskBenchmark.gaps", "Gaps") }
    static var taskBenchmarkSafetyFlags: String { text("taskBenchmark.safetyFlags", "Safety flags") }
    static var taskBenchmarkNoBlockers: String { text("taskBenchmark.empty.blockers", "No blockers returned.") }
    static var taskBenchmarkNoGaps: String { text("taskBenchmark.empty.gaps", "No gaps returned.") }
    static var taskBenchmarkNoSafetyFlags: String { text("taskBenchmark.empty.safetyFlags", "No safety flags returned.") }
    static var routingRegressionTitle: String { text("routingRegression.title", "Routing Regression") }
    static var routingRegressionBoundary: String { text("routingRegression.boundary", "User-triggered, app-local regression detection from saved benchmark baselines. Detection is deterministic and cannot call a provider, write skill files, mutate agent config, create snapshots, change triage, execute scripts, or read credentials.") }
    static var routingRegressionSaveBaselineAction: String { text("routingRegression.action.saveBaseline", "Save Baseline") }
    static var routingRegressionDetectAction: String { text("routingRegression.action.detect", "Detect Regressions") }
    static var routingRegressionUnavailable: String { text("routingRegression.unavailable", "Routing regression detection is unavailable in this service build.") }
    static var routingRegressionNoBaseline: String { text("routingRegression.empty.baseline", "No routing baseline shown yet.") }
    static var routingRegressionBaselineStatus: String { text("routingRegression.baselineStatus", "Baseline status") }
    static var routingRegressionDetectionTitle: String { text("routingRegression.detection", "Regression detection") }
    static var routingRegressionCount: String { text("routingRegression.count", "Regressions") }
    static var routingRegressionImproved: String { text("routingRegression.improved", "Improved") }
    static var routingRegressionUnchanged: String { text("routingRegression.unchanged", "Unchanged") }
    static var routingRegressionAverageScoreDelta: String { text("routingRegression.averageScoreDelta", "Average delta") }
    static var routingRegressionMatchChanges: String { text("routingRegression.matchChanges", "Match changes") }
    static var routingRegressionTopRouteChanges: String { text("routingRegression.topRouteChanges", "Top-route changes") }
    static var routingRegressionItems: String { text("routingRegression.items", "Regression items") }
    static var routingRegressionNoItems: String { text("routingRegression.empty.items", "No regressions returned.") }
    static var routingRegressionNewBlockers: String { text("routingRegression.newBlockers", "New blockers") }
    static var routingRegressionNoNewBlockers: String { text("routingRegression.empty.newBlockers", "No new blockers returned.") }
    static var routingRegressionNewGaps: String { text("routingRegression.newGaps", "New gaps") }
    static var routingRegressionNoNewGaps: String { text("routingRegression.empty.newGaps", "No new gaps returned.") }
    static var routingRegressionTopRouteChanged: String { text("routingRegression.topRouteChanged", "Top route changed") }
    static var routingRegressionMatchStatus: String { text("routingRegression.matchStatus", "Match status") }
    static var routingRegressionTopRouteChange: String { text("routingRegression.topRouteChange", "Top route") }
    static var traceImportTitle: String { text("traceImport.title", "Agent Behavior Trace Import") }
    static var traceImportBoundary: String { text("traceImport.boundary", "User-triggered local trace import for routing behavior review. Results show redacted excerpts and metadata only; local import cannot call a provider, write skill files, mutate agent config, create snapshots, change triage, execute scripts, or read credentials.") }
    static var traceImportProviderBoundary: String { text("traceImport.providerBoundary", "Provider explanations remain copy-only and must use prompt preview, redaction, and confirmation; this import panel does not send provider requests.") }
    static var traceImportTextPlaceholder: String { text("traceImport.placeholder.text", "Paste local transcript or log text to import") }
    static var traceImportTitlePlaceholder: String { text("traceImport.placeholder.title", "Optional title") }
    static var traceImportTaskPlaceholder: String { text("traceImport.placeholder.task", "Optional task text") }
    static var traceImportExpectedPlaceholder: String { text("traceImport.placeholder.expected", "Optional expected skill names, separated by commas") }
    static var traceImportImportAction: String { text("traceImport.action.import", "Import Trace") }
    static var traceImportLoadAction: String { text("traceImport.action.load", "Load Imports") }
    static var traceImportDeleteAction: String { text("traceImport.action.delete", "Delete import") }
    static var traceImportInputRequired: String { text("traceImport.inputRequired", "Paste trace text before importing.") }
    static var traceImportUnavailable: String { text("traceImport.unavailable", "Trace import is unavailable in this service build.") }
    static var traceImportDeleteUnavailable: String { text("traceImport.deleteUnavailable", "Deleting trace imports is unavailable in this service build.") }
    static var traceImportLatest: String { text("traceImport.latest", "Latest trace outcome") }
    static var traceImportImports: String { text("traceImport.imports", "Trace imports") }
    static var traceImportNoImports: String { text("traceImport.empty.imports", "No trace imports returned.") }
    static var traceImportOutcome: String { text("traceImport.outcome", "Outcome") }
    static var traceImportDetectedSkills: String { text("traceImport.detectedSkills", "Detected skills") }
    static var traceImportExpectedSkills: String { text("traceImport.expectedSkills", "Expected skills") }
    static var traceImportRedactedExcerpt: String { text("traceImport.redactedExcerpt", "Redacted excerpt") }
    static var traceImportRedactionSummary: String { text("traceImport.redactionSummary", "Redaction summary") }
    static var traceImportReasons: String { text("traceImport.reasons", "Reasons") }
    static var traceImportEvidence: String { text("traceImport.evidence", "Evidence") }
    static var traceImportNoSkills: String { text("traceImport.empty.skills", "No skills returned.") }
    static var traceImportNoExcerpt: String { text("traceImport.empty.excerpt", "No redacted excerpt returned.") }
    static var traceImportNoReasons: String { text("traceImport.empty.reasons", "No reasons returned.") }
    static var agentSessionReviewTitle: String { text("sessionReview.title", "Agent Session Skill Review") }
    static var agentSessionReviewBoundary: String { text("sessionReview.boundary", "User-triggered app-local session review for pasted transcript metadata. It detects skill use, expected matches, interference, safe next steps, and safety flags without provider calls, skill writes, agent config mutation, snapshots, triage changes, scripts, credentials, raw prompt/response persistence, cloud sync, or telemetry.") }
    static var agentSessionReviewNoWriteBoundary: String { text("sessionReview.noWriteBoundary", "Review only. No Apply, Confirm, Write, Snapshot, Rollback, Script, Provider Send, or Triage action is exposed here.") }
    static var agentSessionReviewAppLocal: String { text("sessionReview.appLocal", "App-local metadata") }
    static var agentSessionReviewTranscriptPlaceholder: String { text("sessionReview.placeholder.transcript", "Paste session transcript or agent log text") }
    static var agentSessionReviewTaskPlaceholder: String { text("sessionReview.placeholder.task", "Optional task text") }
    static var agentSessionReviewExpectedPlaceholder: String { text("sessionReview.placeholder.expected", "Optional expected skill names, separated by commas") }
    static var agentSessionReviewAction: String { text("sessionReview.action.review", "Review Session") }
    static var agentSessionReviewLoadAction: String { text("sessionReview.action.load", "Load Analysis History") }
    static var agentSessionReviewDeleteAction: String { text("sessionReview.action.delete", "Delete review") }
    static var agentSessionReviewInputRequired: String { text("sessionReview.inputRequired", "Paste a session transcript before reviewing.") }
    static var agentSessionReviewUnavailable: String { text("sessionReview.unavailable", "Agent session skill review is unavailable in this service build.") }
    static var agentSessionReviewDeleteUnavailable: String { text("sessionReview.deleteUnavailable", "Deleting session skill reviews is unavailable in this service build.") }
    static var agentSessionReviewLatest: String { text("sessionReview.latest", "Latest session review") }
    static var agentSessionReviewReviews: String { text("sessionReview.reviews", "Session reviews") }
    static var agentSessionReviewNoReviews: String { text("sessionReview.empty.reviews", "No session skill reviews returned.") }
    static var agentSessionReviewRecord: String { text("sessionReview.record", "Session review") }
    static var agentSessionReviewOutcome: String { text("sessionReview.outcome", "Outcome") }
    static var agentSessionReviewDetectedSkills: String { text("sessionReview.detectedSkills", "Detected skills") }
    static var agentSessionReviewExpectedSkills: String { text("sessionReview.expectedSkills", "Expected skills") }
    static var agentSessionReviewInterference: String { text("sessionReview.interference", "Interference") }
    static var agentSessionReviewNoInterference: String { text("sessionReview.empty.interference", "No interference returned.") }
    static var agentSessionReviewSafeNextSteps: String { text("sessionReview.safeNextSteps", "Safe next steps") }
    static var agentSessionReviewNoSafeNextSteps: String { text("sessionReview.empty.safeNextSteps", "No safe next steps returned.") }
    static var agentSessionReviewRedactedExcerpt: String { text("sessionReview.redactedExcerpt", "Redacted excerpt") }
    static var agentSessionReviewNoExcerpt: String { text("sessionReview.empty.excerpt", "No redacted excerpt returned.") }
    static var agentSessionReviewReasons: String { text("sessionReview.reasons", "Review notes") }
    static var agentSessionReviewNoReasons: String { text("sessionReview.empty.reasons", "No review notes returned.") }
    static var agentSessionReviewNoSkills: String { text("sessionReview.empty.skills", "No skills returned.") }
    static var llmAssist: String { text("llm.assist", "LLM Assist") }
    static var llmEnabled: String { text("llm.enabled", "Enabled") }
    static var llmDisabled: String { text("llm.disabled", "Disabled") }
    static var llmPreparing: String { text("llm.preparing", "Preparing...") }
    static var llmPreparePrompt: String { text("llm.preparePrompt", "Choose an action to preview tokens and cost.") }
    static var llmDisabledFallback: String { text("llm.disabledFallback", "LLM assist is unavailable in this build.") }
    static var llmProvider: String { text("llm.provider", "Provider") }
    static var llmModel: String { text("llm.model", "Model") }
    static var llmTokens: String { text("llm.tokens", "Tokens") }
    static var llmCost: String { text("llm.cost", "Cost") }
    static var llmConfirmationRequired: String { text("llm.confirmationRequired", "Confirmation required before any LLM call.") }
    static var llmDraftCopyRequired: String { text("llm.draftCopyRequired", "Draft output requires user confirmation and copy.") }
    static var llmReviewPreview: String { text("llm.reviewPreview", "Read-only review preview") }
    static var llmReviewPurpose: String { text("llm.reviewPurpose", "Purpose") }
    static var llmReviewRisk: String { text("llm.reviewRisk", "Risk") }
    static var llmReviewSignals: String { text("llm.reviewSignals", "Signals") }
    static var llmReviewFindings: String { text("llm.reviewFindings", "Finding explanations") }
    static var llmReviewCrossAgentFit: String { text("llm.reviewCrossAgentFit", "Cross-agent fit") }
    static var llmReviewRedaction: String { text("llm.reviewRedaction", "Redaction") }
    static var llmReviewNoFindings: String { text("llm.reviewNoFindings", "No finding explanations in this preview.") }
    static var llmReviewNoSignals: String { text("llm.reviewNoSignals", "No risk signals in this preview.") }
    static var llmReviewNoActions: String { text("llm.reviewNoActions", "No provider request, write action, or execution action is available from this preview.") }
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

    static func crossAgentComparisonFilterContext(_ agent: String) -> String {
        format("comparison.crossAgent.filterContext", "Context: %@ filter. Service data is preferred when available; otherwise this panel uses local catalog-only comparison.", agent)
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

    static func taskBenchmarkExpectedCurrentSkill(_ skill: String, _ agent: String) -> String {
        format("taskBenchmark.expectedCurrentSkill", "Expected and acceptable route: %@ (%@)", skill, agent)
    }

    static func agentConfigTimelineRollbackConfirm(_ target: String) -> String {
        format("sidebar.agentConfigTimeline.rollbackConfirm", "Rollback restores this agent config file only after confirmation. Skill content snapshots are not included. Target: %@", target)
    }

    static func agentProfileRiskSubsetInline(_ count: Int) -> String {
        format("agentCopilot.agentProfile.riskSubset.inline", "Risk-related %d", count)
    }

    static func mcpServerArgEnvSummary(args: Int, envKeys: Int) -> String {
        format("mcpServerPreview.counts", "Args: %d · Env keys: %d", args, envKeys)
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

    static func localReportExported(_ filename: String) -> String {
        format("localReport.exported", "Exported local redacted report: %@.", filename)
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

    static func cleanupAgentFilterNote(_ agent: String) -> String {
        format("cleanup.filter.agentNote", "Agent filter: %@", agent)
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
