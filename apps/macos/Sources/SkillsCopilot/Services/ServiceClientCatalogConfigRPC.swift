import Foundation

extension ServiceClient {
    func previewScriptExecution(skill: SkillRecord) async throws -> ScriptExecutionPreview {
        try await call(
            method: "script.previewExecution",
            params: ScriptExecutionParams(
                instanceId: skill.id,
                definitionId: skill.definitionId,
                agent: skill.agent
            )
        )
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
        try await call(method: "project.getContext", params: EmptyParams())
    }

    func previewSetProjectContext(
        rootPath: String,
        currentCWD: String?,
        name: String?,
        expectedRevision: String
    ) async throws -> ProjectContextActionPreview {
        let preview: ProjectContextActionPreview = try await call(
            method: "project.previewSetContext",
            params: ProjectContextSetPreviewParams(
                rootPath: rootPath,
                currentCWD: currentCWD,
                name: name,
                expectedRevision: expectedRevision
            )
        )
        guard let projectID = preview.candidate.active?.id else {
            throw ClientError.invalidOutput("Project context preview omitted its candidate.")
        }
        try validateProjectAction(
            preview,
            previewMethod: "project.previewSetContext",
            applyMethod: "project.setContext",
            intent: "set_project_context",
            targetKind: "project",
            targetID: projectID,
            projectID: .exact(projectID)
        )
        return preview
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
        let result: ProjectContextApplyResult = try await call(
            method: "project.setContext",
            params: ProjectContextSetApplyParams(
                rootPath: rootPath,
                currentCWD: currentCWD,
                name: name,
                candidateLastUsedAt: candidateLastUsedAt,
                confirmation: preview.confirmation
            )
        )
        try validateProjectApply(result, preview: preview)
        return result
    }

    func previewClearProjectContext(expectedRevision: String) async throws -> ProjectContextActionPreview {
        let preview: ProjectContextActionPreview = try await call(
            method: "project.previewClearContext",
            params: ProjectContextRevisionParams(expectedRevision: expectedRevision)
        )
        try validateProjectAction(
            preview,
            previewMethod: "project.previewClearContext",
            applyMethod: "project.clearContext",
            intent: "clear_project_context",
            targetKind: "config",
            targetID: "project-context",
            projectID: .absent
        )
        return preview
    }

    func clearProjectContext(preview: ProjectContextActionPreview) async throws -> ProjectContextApplyResult {
        let result: ProjectContextApplyResult = try await call(
            method: "project.clearContext",
            params: ProjectContextConfirmationParams(confirmation: preview.confirmation)
        )
        try validateProjectApply(result, preview: preview)
        return result
    }

    func previewRemoveRecentProjectContext(
        id: String,
        expectedRevision: String
    ) async throws -> ProjectContextActionPreview {
        let preview: ProjectContextActionPreview = try await call(
            method: "project.previewRemoveRecentContext",
            params: ProjectContextIDPreviewParams(id: id, expectedRevision: expectedRevision)
        )
        try validateProjectAction(
            preview,
            previewMethod: "project.previewRemoveRecentContext",
            applyMethod: "project.removeRecentContext",
            intent: "remove_recent_project_context",
            targetKind: "project",
            targetID: id,
            projectID: .exact(id)
        )
        return preview
    }

    func removeRecentProjectContext(
        id: String,
        preview: ProjectContextActionPreview
    ) async throws -> ProjectContextApplyResult {
        let result: ProjectContextApplyResult = try await call(
            method: "project.removeRecentContext",
            params: ProjectContextIDApplyParams(id: id, confirmation: preview.confirmation)
        )
        try validateProjectApply(result, preview: preview)
        return result
    }

    func previewClearRecentProjectContexts(
        expectedRevision: String
    ) async throws -> ProjectContextActionPreview {
        let preview: ProjectContextActionPreview = try await call(
            method: "project.previewClearRecentContexts",
            params: ProjectContextRevisionParams(expectedRevision: expectedRevision)
        )
        try validateProjectAction(
            preview,
            previewMethod: "project.previewClearRecentContexts",
            applyMethod: "project.clearRecentContexts",
            intent: "clear_recent_project_contexts",
            targetKind: "config",
            targetID: "project-context-recents",
            projectID: .optional
        )
        return preview
    }

    func clearRecentProjectContexts(
        preview: ProjectContextActionPreview
    ) async throws -> ProjectContextApplyResult {
        let result: ProjectContextApplyResult = try await call(
            method: "project.clearRecentContexts",
            params: ProjectContextConfirmationParams(confirmation: preview.confirmation)
        )
        try validateProjectApply(result, preview: preview)
        return result
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

    func listRuleTuning() async throws -> [RuleTuningRecord] {
        let list: RuleTuningList = try await call(method: "rules.listTuning", params: EmptyParams())
        return list.records
    }

    func listConflicts() async throws -> [ConflictGroupRecord] {
        try await call(method: "catalog.listConflicts", params: EmptyParams())
    }

    func listSnapshots() async throws -> [ConfigSnapshotRecord] {
        try await call(method: "snapshot.list", params: EmptyParams())
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
        let preview: BatchTogglePreview = try await call(
            method: "batch.previewSkillToggles",
            params: params
        )
        if preview.applySupported {
            guard let action = preview.actionDescriptor else {
                throw ClientError.invalidOutput("Batch preview omitted its typed action.")
            }
            let agents = Set(preview.affectedSkills.map(\.agent))
            let scopes = Set(preview.affectedSkills.map(\.scope))
            let expectedAgent: ActionStringExpectation = agents.count == 1
                ? .exact(agents.first!)
                : .absent
            let expectedScope: ActionStringExpectation = scopes.count == 1
                ? .exact(scopes.first!)
                : .absent
            do {
                try action.validated(
                    previewMethod: "batch.previewSkillToggles",
                    applyMethod: "batch.applySkillToggles",
                    network: "none",
                    expectation: ActionDescriptorExpectation(
                        kind: "toggle_skill",
                        intent: on ? "enable_skill" : "disable_skill",
                        targetKind: "skill",
                        targetID: .present,
                        targetAgent: expectedAgent,
                        targetScope: expectedScope,
                        projectID: .optional,
                        impacts: ["app_local_data", "agent_config"],
                        readback: ["catalog_skills", "agent_config", "config_snapshots"]
                    )
                )
                try action.validatedBatchToggleBinding()
                try preview.preconditions.validated(
                    kinds: ["agent_config", "catalog_record"]
                )
            } catch {
                throw ClientError.invalidOutput(error.localizedDescription)
            }
        }
        return preview
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
        let preview: ToolGlobalInstallPreview = try await call(
            method: "skill.install",
            params: ToolInstallPreviewParams(
                instanceId: skill.id,
                targetAgent: target.rawValue,
                targetScope: "agent-global",
                confirmed: false,
                actionConfirmation: nil
            )
        )
        if preview.writeBackEnabled {
            try validateToolInstallAction(preview, skillID: skill.id, target: target)
        }
        return preview
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
        try validateToolInstallAction(preview, skillID: skill.id, target: preview.target)
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
        let preview: ConfigSavePreviewRecord = try await call(
            method: "config.previewSaveClaudeSettings",
            params: PreviewSaveClaudeSettingsParams(
                content: content,
                expectedRevision: expectedRevision
            )
        )
        try validateConfigAction(
            preview.action,
            preconditions: preview.preconditions,
            previewMethod: "config.previewSaveClaudeSettings",
            applyMethod: "config.saveClaudeSettings",
            kind: "save_config",
            intent: "save_config",
            impacts: ["agent_config", "app_local_data"],
            readback: ["catalog_skills", "skill_aggregates", "agent_config", "config_snapshots"],
            preconditionKinds: ["agent_config"],
            targetAgent: .exact("claude-code"),
            targetScope: .exact("agent-global")
        )
        return preview
    }

    func saveClaudeSettings(
        content: String,
        confirmation: ActionConfirmationWire
    ) async throws -> ConfigSaveApplyRecord {
        let result: ConfigSaveApplyRecord = try await call(
            method: "config.saveClaudeSettings",
            params: SaveClaudeSettingsParams(content: content, confirmation: confirmation)
        )
        guard result.action.reference == confirmation.reference else {
            throw ClientError.invalidOutput("Config save response belongs to another action.")
        }
        _ = try result.readback.validated(for: result.action)
        return result
    }

    func previewSnapshotRollback(snapshotID: String) async throws -> SnapshotRollbackPreviewRecord {
        let preview: SnapshotRollbackPreviewRecord = try await call(
            method: "snapshot.previewRollback",
            params: SnapshotParams(snapshotId: snapshotID)
        )
        try validateConfigAction(
            preview.action,
            preconditions: preview.preconditions,
            previewMethod: "snapshot.previewRollback",
            applyMethod: "snapshot.rollback",
            kind: "rollback_config",
            intent: "rollback_config",
            impacts: ["agent_config", "app_local_data"],
            readback: ["catalog_skills", "skill_aggregates", "agent_config"],
            preconditionKinds: ["catalog_record", "agent_config"],
            targetAgent: .oneOf(ActionDescriptorWire.configMutationAgents),
            targetScope: .oneOf(["agent-global", "agent-project"])
        )
        return preview
    }

    func rollbackSnapshot(
        snapshotID: String,
        confirmation: ActionConfirmationWire
    ) async throws -> SnapshotRollbackApplyRecord {
        let result: SnapshotRollbackApplyRecord = try await call(
            method: "snapshot.rollback",
            params: RollbackSnapshotParams(
                snapshotId: snapshotID,
                confirmation: confirmation
            )
        )
        guard result.action.reference == confirmation.reference else {
            throw ClientError.invalidOutput("Snapshot rollback response belongs to another action.")
        }
        _ = try result.readback.validated(for: result.action)
        return result
    }

    private func validateProjectAction(
        _ preview: ProjectContextActionPreview,
        previewMethod: String,
        applyMethod: String,
        intent: String,
        targetKind: String,
        targetID: String,
        projectID: ActionStringExpectation
    ) throws {
        do {
            try preview.action.validated(
                previewMethod: previewMethod,
                applyMethod: applyMethod,
                network: "none",
                expectation: ActionDescriptorExpectation(
                    kind: "project_context",
                    intent: intent,
                    targetKind: targetKind,
                    targetID: .exact(targetID),
                    targetAgent: .absent,
                    targetScope: .absent,
                    projectID: projectID,
                    impacts: ["app_local_data"],
                    readback: ["project_context"]
                )
            )
            try preview.preconditions.validated(kinds: ["project_context"])
            guard !preview.previewToken.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty else {
                throw ActionDescriptorValidationError.invalidLifecycle
            }
        } catch {
            throw ClientError.invalidOutput(error.localizedDescription)
        }
    }

    private func validateProjectApply(
        _ result: ProjectContextApplyResult,
        preview: ProjectContextActionPreview
    ) throws {
        guard result.action == preview.action else {
            throw ClientError.invalidOutput("Project context response belongs to another preview.")
        }
        _ = try result.readback.validated(for: preview.action)
    }

    private func validateToolInstallAction(
        _ preview: ToolGlobalInstallPreview,
        skillID: String,
        target: ToolInstallTarget
    ) throws {
        guard let action = preview.action,
              preview.skillID == skillID,
              preview.target == target,
              preview.confirmationRequired,
              let previewToken = preview.previewToken,
              !previewToken.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty else {
            throw ClientError.invalidOutput("Tool install preview is incomplete.")
        }
        do {
            try action.validated(
                previewMethod: "skill.install",
                applyMethod: "skill.install",
                network: "none",
                expectation: ActionDescriptorExpectation(
                    kind: "install_skill",
                    intent: "install_skill",
                    targetKind: "skill",
                    targetID: .present,
                    targetAgent: .exact(target.rawValue),
                    targetScope: .exact("agent-global"),
                    projectID: .absent,
                    impacts: ["app_local_data", "skill_files"],
                    readback: ["catalog_skills", "skill_files"]
                )
            )
            try preview.preconditions.validated(
                kinds: ["catalog_record", "source_file", "target_file"]
            )
        } catch {
            throw ClientError.invalidOutput(error.localizedDescription)
        }
    }

    private func validateConfigAction(
        _ action: ActionDescriptorWire,
        preconditions: [ActionPreconditionWire],
        previewMethod: String,
        applyMethod: String,
        kind: String,
        intent: String,
        impacts: [String],
        readback: [String],
        preconditionKinds: Set<String>,
        targetAgent: ActionStringExpectation,
        targetScope: ActionStringExpectation
    ) throws {
        do {
            try action.validated(
                previewMethod: previewMethod,
                applyMethod: applyMethod,
                network: "none",
                expectation: ActionDescriptorExpectation(
                    kind: kind,
                    intent: intent,
                    targetKind: "config",
                    targetID: .present,
                    targetAgent: targetAgent,
                    targetScope: targetScope,
                    projectID: .optional,
                    impacts: impacts,
                    readback: readback
                )
            )
            try action.validatedConfigMutationBinding()
            try preconditions.validated(kinds: preconditionKinds)
        } catch {
            throw ClientError.invalidOutput(error.localizedDescription)
        }
    }
}
