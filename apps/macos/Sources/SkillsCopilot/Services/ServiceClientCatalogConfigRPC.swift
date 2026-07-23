import Foundation

extension ServiceClient {
    func previewScriptExecution(skill: SkillRecord) async throws -> ScriptExecutionPreview {
        do {
            return try await call(
                method: "script.previewExecution",
                params: ScriptExecutionParams(
                    instanceId: skill.id,
                    definitionId: skill.definitionId,
                    agent: skill.agent
                )
            )
        } catch ClientError.service(let error) where error.code == "unknown_method" {
            return .unavailable(skill: skill)
        }
    }

    func scanAll(expectedContextRevision: String? = nil) async throws -> ScanResult {
        try await call(
            method: "catalog.scanAll",
            params: CatalogScanParams(
                explicitRefresh: true,
                expectedContextRevision: expectedContextRevision
            )
        )
    }

    func scanClaude(expectedContextRevision: String? = nil) async throws -> ScanResult {
        try await call(
            method: "catalog.scanClaude",
            params: CatalogScanParams(
                explicitRefresh: true,
                expectedContextRevision: expectedContextRevision
            )
        )
    }

    func getProjectContext() async throws -> ProjectContextState {
        do {
            return try await call(method: "project.getContext", params: EmptyParams())
        } catch ClientError.service(let error) where error.code == "unknown_method" {
            return ProjectContextState(revision: "", active: nil, recent: [])
        }
    }

    func previewSetProjectContext(
        rootPath: String,
        currentCWD: String?,
        name: String?,
        expectedRevision: String
    ) async throws -> ProjectContextActionPreview {
        try await call(
            method: "project.previewSetContext",
            params: ProjectContextSetPreviewParams(
                rootPath: rootPath,
                currentCWD: currentCWD,
                name: name,
                expectedRevision: expectedRevision
            )
        )
    }

    func setProjectContext(
        rootPath: String,
        currentCWD: String?,
        name: String?,
        preview: ProjectContextActionPreview
    ) async throws -> ProjectContextApplyResult {
        guard let candidateLastUsedAt = preview.candidate.active?.lastUsedAt.flatMap(Int64.init) else {
            throw ClientError.invalidOutput("Project context preview omitted its candidate timestamp.")
        }
        return try await call(
            method: "project.setContext",
            params: ProjectContextSetApplyParams(
                rootPath: rootPath,
                currentCWD: currentCWD,
                name: name,
                candidateLastUsedAt: candidateLastUsedAt,
                confirmation: preview.confirmation
            )
        )
    }

    func previewClearProjectContext(expectedRevision: String) async throws -> ProjectContextActionPreview {
        try await call(
            method: "project.previewClearContext",
            params: ProjectContextRevisionParams(expectedRevision: expectedRevision)
        )
    }

    func clearProjectContext(preview: ProjectContextActionPreview) async throws -> ProjectContextApplyResult {
        try await call(
            method: "project.clearContext",
            params: ProjectContextConfirmationParams(confirmation: preview.confirmation)
        )
    }

    func previewRemoveRecentProjectContext(
        id: String,
        expectedRevision: String
    ) async throws -> ProjectContextActionPreview {
        try await call(
            method: "project.previewRemoveRecentContext",
            params: ProjectContextIDPreviewParams(id: id, expectedRevision: expectedRevision)
        )
    }

    func removeRecentProjectContext(
        id: String,
        preview: ProjectContextActionPreview
    ) async throws -> ProjectContextApplyResult {
        try await call(
            method: "project.removeRecentContext",
            params: ProjectContextIDApplyParams(id: id, confirmation: preview.confirmation)
        )
    }

    func previewClearRecentProjectContexts(
        expectedRevision: String
    ) async throws -> ProjectContextActionPreview {
        try await call(
            method: "project.previewClearRecentContexts",
            params: ProjectContextRevisionParams(expectedRevision: expectedRevision)
        )
    }

    func clearRecentProjectContexts(
        preview: ProjectContextActionPreview
    ) async throws -> ProjectContextApplyResult {
        try await call(
            method: "project.clearRecentContexts",
            params: ProjectContextConfirmationParams(confirmation: preview.confirmation)
        )
    }

    func validateProjectContext(rootPath: String, currentCWD: String?, name: String?) async throws -> ProjectContext {
        try await call(
            method: "project.validateContext",
            params: ProjectContextParams(rootPath: rootPath, currentCWD: currentCWD, name: name)
        )
    }

    func getSkill(instanceID: String) async throws -> SkillDetailRecord {
        try await call(
            method: "catalog.getSkill",
            params: GetSkillParams(instanceId: instanceID)
        )
    }

    func listFindings() async throws -> [RuleFindingRecord] {
        try await call(method: "catalog.listFindings", params: EmptyParams())
    }

    func listFindingTriage() async throws -> [FindingTriageRecord] {
        try await call(method: "catalog.listFindingTriage", params: EmptyParams())
    }

    func listRuleTuning() async throws -> [RuleTuningRecord] {
        do {
            let list: RuleTuningList = try await call(method: "rules.listTuning", params: EmptyParams())
            return list.records
        } catch ClientError.service(let error) where error.code == "unknown_method" {
            return []
        }
    }

    func listConflicts() async throws -> [ConflictGroupRecord] {
        try await call(method: "catalog.listConflicts", params: EmptyParams())
    }

    func listSnapshots() async throws -> [ConfigSnapshotRecord] {
        try await call(method: "snapshot.list", params: EmptyParams())
    }

    func listAgentConfigSnapshots(agent: String, scope: String? = nil) async throws -> [ConfigSnapshotRecord] {
        try await call(
            method: "snapshot.listAgentConfig",
            params: ListAgentConfigSnapshotsParams(agent: agent, scope: scope)
        )
    }

    func listAgentConfigSnapshotPage(
        agent: String,
        scope: String?,
        limit: Int = 100,
        cursor: String?,
        sourceRevision: String?
    ) async throws -> ConfigSnapshotPageResult {
        try await call(
            method: "snapshot.listAgentConfigPage",
            params: ListAgentConfigPageParams(
                agent: agent,
                scope: scope,
                limit: limit,
                cursor: cursor,
                sourceRevision: sourceRevision
            )
        )
    }

    func listSkillEvents(instanceID: String, limit: Int? = nil) async throws -> [SkillEventRecord] {
        try await call(
            method: "skill.listEvents",
            params: ListSkillEventsParams(instanceId: instanceID, limit: limit)
        )
    }

    func listSkillEventPage(
        instanceID: String,
        limit: Int = 100,
        cursor: String?,
        sourceRevision: String?
    ) async throws -> SkillEventPageResult {
        try await call(
            method: "skill.listEventsPage",
            params: ListSkillEventsPageParams(
                instanceId: instanceID,
                limit: limit,
                cursor: cursor,
                sourceRevision: sourceRevision
            )
        )
    }

    func previewBatchSkillToggles(instanceIDs: [String], on: Bool) async throws -> BatchTogglePreview {
        let params = BatchToggleParams(
            instanceIDs: instanceIDs,
            targetEnabled: on,
            confirmation: nil
        )
        return try await call(method: "batch.previewSkillToggles", params: params)
    }

    func applyBatchSkillToggles(preview: BatchTogglePreview) async throws -> BatchToggleApplyResult {
        guard let action = preview.actionDescriptor,
              let previewToken = preview.previewToken,
              !previewToken.isEmpty else {
            throw ClientError.invalidOutput(
                "A writable batch preview is missing its typed action confirmation."
            )
        }
        let params = BatchToggleParams(
            // Re-submit the complete reviewed selection. The server-side action
            // binding includes skipped/no-op entries and requested_count, so
            // dropping them here would turn every mixed preview into a stale
            // action before apply.
            instanceIDs: (preview.affectedSkills + preview.skippedItems).map(\.instanceID),
            targetEnabled: preview.targetEnabled,
            confirmation: ActionConfirmationWire(
                action: action,
                previewToken: previewToken
            )
        )
        let result: BatchToggleApplyResult = try await call(
            method: "batch.applySkillToggles",
            params: params
        )
        guard result.actionDescriptor == action,
              let readback = result.readback else {
            throw ClientError.invalidOutput(
                "The batch apply response is missing its action-bound read-back."
            )
        }
        do {
            try readback.validated(for: action)
        } catch {
            throw ClientError.invalidOutput(error.localizedDescription)
        }
        return result
    }

    func previewToolInstall(skill: SkillRecord, target: ToolInstallTarget) async throws -> ToolGlobalInstallPreview {
        try await call(
            method: "skill.install",
            params: ToolInstallPreviewParams(
                instanceId: skill.id,
                targetAgent: target.rawValue,
                targetScope: "agent-global",
                confirmed: false,
                actionConfirmation: nil
            )
        )
    }

    func confirmToolInstall(
        skill: SkillRecord,
        preview: ToolGlobalInstallPreview
    ) async throws -> ToolGlobalInstallPreview {
        guard let action = preview.action,
              let previewToken = preview.previewToken,
              !previewToken.isEmpty else {
            throw ClientError.invalidOutput(
                "The install preview is missing its typed action confirmation."
            )
        }
        let result: ToolGlobalInstallPreview = try await call(
            method: "skill.install",
            params: ToolInstallPreviewParams(
                instanceId: skill.id,
                targetAgent: preview.target.rawValue,
                targetScope: "agent-global",
                confirmed: true,
                actionConfirmation: ActionConfirmationWire(
                    action: action,
                    previewToken: previewToken
                )
            )
        )
        guard result.action == action,
              let readback = result.readback else {
            throw ClientError.invalidOutput(
                "The install response is missing its action-bound read-back."
            )
        }
        do {
            try readback.validated(for: action)
        } catch {
            throw ClientError.invalidOutput(error.localizedDescription)
        }
        return result
    }

    func readClaudeSettings() async throws -> ConfigDocumentRecord {
        try await call(method: "config.readClaudeSettings", params: EmptyParams())
    }

    func readAgentConfig(agent: String, scope: String? = nil) async throws -> [ConfigDocumentRecord] {
        try await call(
            method: "config.readAgentConfig",
            params: ReadAgentConfigParams(agent: agent, scope: scope)
        )
    }

    func previewClaudeSettingsSave(
        content: String,
        expectedRevision: String
    ) async throws -> ConfigSavePreviewRecord {
        try await call(
            method: "config.previewSaveClaudeSettings",
            params: PreviewSaveClaudeSettingsParams(
                content: content,
                expectedRevision: expectedRevision
            )
        )
    }

    func saveClaudeSettings(
        content: String,
        confirmation: ActionConfirmationWire
    ) async throws -> ConfigSaveApplyRecord {
        try await call(
            method: "config.saveClaudeSettings",
            params: SaveClaudeSettingsParams(content: content, confirmation: confirmation)
        )
    }

    func previewSnapshotRollback(snapshotID: String) async throws -> SnapshotRollbackPreviewRecord {
        try await call(
            method: "snapshot.previewRollback",
            params: SnapshotParams(snapshotId: snapshotID)
        )
    }

    func rollbackSnapshot(
        snapshotID: String,
        confirmation: ActionConfirmationWire
    ) async throws -> SnapshotRollbackApplyRecord {
        try await call(
            method: "snapshot.rollback",
            params: RollbackSnapshotParams(
                snapshotId: snapshotID,
                confirmation: confirmation
            )
        )
    }
}
