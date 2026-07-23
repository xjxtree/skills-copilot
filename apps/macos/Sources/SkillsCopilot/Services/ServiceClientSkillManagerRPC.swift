import Foundation

extension ServiceClient {
    func listSkillManagerTools() async throws -> [SkillManagerToolRecord] {
        try await call(method: "skillManager.listTools", params: EmptyParams())
    }

    func searchSkillManager(
        query: String,
        owner: String?
    ) async throws -> SkillManagerSearchRecord {
        let result: SkillManagerSearchRecord = try await call(
            method: "skillManager.search",
            params: SkillManagerSearchParams(
                query: query,
                owner: owner,
                networkAllowed: true
            ),
            timeoutMS: 120_000
        )
        guard result.hasValidPageMetadata else {
            throw ClientError.invalidOutput("skillManager.search returned inconsistent page metadata")
        }
        _ = try requiredAction(
            result.preview,
            previewMethod: "skillManager.search",
            applyMethod: "skillManager.applySearch",
            kind: "refresh_evidence",
            intent: "inspect_evidence",
            network: "required"
        )
        guard result.preview.requiresConfirmation,
              result.preview.networkRequired,
              result.preview.networkAllowed,
              !result.preview.willRun,
              result.output == nil,
              result.results.isEmpty,
              result.readback == nil else {
            throw ClientError.invalidOutput(
                "skillManager.search must return a local-only confirmation preview"
            )
        }
        return result
    }

    func applySkillManagerSearch(
        preview: SkillManagerSearchRecord,
        query: String,
        owner: String?
    ) async throws -> SkillManagerSearchRecord {
        let action = try requiredAction(
            preview.preview,
            previewMethod: "skillManager.search",
            applyMethod: "skillManager.applySearch",
            kind: "refresh_evidence",
            intent: "inspect_evidence",
            network: "required"
        )
        let previewToken = try requiredPreviewToken(preview.preview)
        let result: SkillManagerSearchRecord = try await call(
            method: "skillManager.applySearch",
            params: SkillManagerSearchApplyParams(
                query: query,
                owner: owner,
                networkAllowed: true,
                confirmed: true,
                previewToken: previewToken,
                actionReference: action.reference
            ),
            timeoutMS: 120_000
        )
        guard result.hasValidPageMetadata else {
            throw ClientError.invalidOutput("skillManager.applySearch returned inconsistent page metadata")
        }
        guard result.preview.action == action else {
            throw ClientError.invalidOutput("Skill Manager search response belongs to another action.")
        }
        try requireVerifiedReadback(
            result.readback,
            action: action,
            operation: "Skill Manager search"
        )
        return result
    }

    func listSkillManagerInstalled(scope: SkillManagerScope) async throws -> SkillManagerInstalledListRecord {
        let result: SkillManagerInstalledListRecord = try await call(
            method: "skillManager.listInstalled",
            params: SkillManagerListInstalledParams(
                agents: [],
                scope: scope.rawValue
            ),
            timeoutMS: 120_000
        )
        guard result.hasValidPageMetadata else {
            throw ClientError.invalidOutput("skillManager.listInstalled returned inconsistent page metadata")
        }
        return result
    }

    func previewSkillManagerInstall(
        source: String,
        skills: [String],
        agents: [String],
        scope: SkillManagerScope
    ) async throws -> SkillManagerMutationRecord {
        let result: SkillManagerMutationRecord = try await call(
            method: "skillManager.previewInstall",
            params: SkillManagerInstallParams(
                source: source,
                skills: skills,
                agents: agents,
                scope: scope.rawValue,
                distribution: nil,
                networkAllowed: true,
                confirmed: false,
                previewToken: nil,
                actionReference: nil
            )
        )
        _ = try requiredAction(
            result.preview,
            previewMethod: "skillManager.previewInstall",
            applyMethod: "skillManager.applyInstall",
            kind: "manager_install",
            intent: "manager_install",
            network: "required",
            targetAgent: agentBinding(for: agents),
            targetScope: .exact(scope == .project ? "agent-project" : "agent-global"),
            projectID: .optional
        )
        return result
    }

    func applySkillManagerInstall(
        preview: SkillManagerMutationRecord,
        source: String,
        skills: [String],
        agents: [String],
        scope: SkillManagerScope
    ) async throws -> SkillManagerMutationRecord {
        let action = try requiredAction(
            preview.preview,
            previewMethod: "skillManager.previewInstall",
            applyMethod: "skillManager.applyInstall",
            kind: "manager_install",
            intent: "manager_install",
            network: "required",
            targetAgent: agentBinding(for: agents),
            targetScope: .exact(scope == .project ? "agent-project" : "agent-global"),
            projectID: .optional
        )
        let result: SkillManagerMutationRecord = try await call(
            method: "skillManager.applyInstall",
            params: SkillManagerInstallParams(
                source: source,
                skills: skills,
                agents: agents,
                scope: scope.rawValue,
                distribution: nil,
                networkAllowed: true,
                confirmed: true,
                previewToken: preview.preview.previewToken,
                actionReference: action.reference
            ),
            timeoutMS: 180_000
        )
        guard result.preview.action == action else {
            throw ClientError.invalidOutput("Skill Manager install response belongs to another action.")
        }
        try requireVerifiedReadback(
            result.readback,
            action: action,
            operation: "Skill Manager install"
        )
        return result
    }

    func previewSkillManagerRemove(
        skill: String,
        agents: [String],
        scope: SkillManagerScope,
        cleanupLocalInstanceID: String?
    ) async throws -> SkillManagerMutationRecord {
        let result: SkillManagerMutationRecord = try await call(
            method: "skillManager.previewRemove",
            params: SkillManagerRemoveParams(
                skill: skill,
                agents: agents,
                scope: scope.rawValue,
                cleanupLocalInstanceID: cleanupLocalInstanceID,
                networkAllowed: true,
                confirmed: false,
                previewToken: nil,
                actionReference: nil
            )
        )
        _ = try requiredAction(
            result.preview,
            previewMethod: "skillManager.previewRemove",
            applyMethod: "skillManager.applyRemove",
            kind: "manager_remove",
            intent: "manager_remove",
            network: "required",
            targetAgent: agentBinding(for: agents),
            targetScope: .exact(scope == .project ? "agent-project" : "agent-global"),
            projectID: .optional
        )
        return result
    }

    func applySkillManagerRemove(
        preview: SkillManagerMutationRecord,
        skill: String,
        agents: [String],
        scope: SkillManagerScope,
        cleanupLocalInstanceID: String?
    ) async throws -> SkillManagerMutationRecord {
        let action = try requiredAction(
            preview.preview,
            previewMethod: "skillManager.previewRemove",
            applyMethod: "skillManager.applyRemove",
            kind: "manager_remove",
            intent: "manager_remove",
            network: "required",
            targetAgent: agentBinding(for: agents),
            targetScope: .exact(scope == .project ? "agent-project" : "agent-global"),
            projectID: .optional
        )
        let result: SkillManagerMutationRecord = try await call(
            method: "skillManager.applyRemove",
            params: SkillManagerRemoveParams(
                skill: skill,
                agents: agents,
                scope: scope.rawValue,
                cleanupLocalInstanceID: cleanupLocalInstanceID,
                networkAllowed: true,
                confirmed: true,
                previewToken: preview.preview.previewToken,
                actionReference: action.reference
            ),
            timeoutMS: 120_000
        )
        guard result.preview.action == action else {
            throw ClientError.invalidOutput("Skill Manager remove response belongs to another action.")
        }
        try requireVerifiedReadback(
            result.readback,
            action: action,
            operation: "Skill Manager remove"
        )
        return result
    }

    func previewSkillManagerUpdate(skills: [String], scope: SkillManagerScope) async throws -> SkillManagerMutationRecord {
        let result: SkillManagerMutationRecord = try await call(
            method: "skillManager.previewUpdate",
            params: SkillManagerUpdateParams(
                skills: skills,
                agents: [],
                scope: scope.rawValue,
                networkAllowed: true,
                confirmed: false,
                previewToken: nil,
                actionReference: nil
            )
        )
        _ = try requiredAction(
            result.preview,
            previewMethod: "skillManager.previewUpdate",
            applyMethod: "skillManager.applyUpdate",
            kind: "manager_update",
            intent: "manager_update",
            network: "required",
            targetAgent: .absent,
            targetScope: .exact(scope == .project ? "agent-project" : "agent-global"),
            projectID: .optional
        )
        return result
    }

    func applySkillManagerUpdate(preview: SkillManagerMutationRecord, skills: [String], scope: SkillManagerScope) async throws -> SkillManagerMutationRecord {
        let action = try requiredAction(
            preview.preview,
            previewMethod: "skillManager.previewUpdate",
            applyMethod: "skillManager.applyUpdate",
            kind: "manager_update",
            intent: "manager_update",
            network: "required",
            targetAgent: .absent,
            targetScope: .exact(scope == .project ? "agent-project" : "agent-global"),
            projectID: .optional
        )
        let result: SkillManagerMutationRecord = try await call(
            method: "skillManager.applyUpdate",
            params: SkillManagerUpdateParams(
                skills: skills,
                agents: [],
                scope: scope.rawValue,
                networkAllowed: true,
                confirmed: true,
                previewToken: preview.preview.previewToken,
                actionReference: action.reference
            ),
            timeoutMS: 180_000
        )
        guard result.preview.action == action else {
            throw ClientError.invalidOutput("Skill Manager update response belongs to another action.")
        }
        try requireVerifiedReadback(
            result.readback,
            action: action,
            operation: "Skill Manager update"
        )
        return result
    }

    func previewSkillManagerLocalCreate(name: String) async throws -> SkillManagerLocalCreateRecord {
        let result: SkillManagerLocalCreateRecord = try await call(
            method: "skillManager.previewLocalCreate",
            params: SkillManagerLocalCreateParams(
                name: name,
                networkAllowed: true,
                confirmed: false,
                previewToken: nil,
                actionReference: nil
            )
        )
        _ = try requiredAction(
            result.preview,
            previewMethod: "skillManager.previewLocalCreate",
            applyMethod: "skillManager.applyLocalCreate",
            kind: "manager_local_create",
            intent: "manager_local_create",
            network: "required"
        )
        return result
    }

    func applySkillManagerLocalCreate(preview: SkillManagerLocalCreateRecord, name: String) async throws -> SkillManagerLocalCreateRecord {
        let action = try requiredAction(
            preview.preview,
            previewMethod: "skillManager.previewLocalCreate",
            applyMethod: "skillManager.applyLocalCreate",
            kind: "manager_local_create",
            intent: "manager_local_create",
            network: "required"
        )
        let result: SkillManagerLocalCreateRecord = try await call(
            method: "skillManager.applyLocalCreate",
            params: SkillManagerLocalCreateParams(
                name: name,
                networkAllowed: true,
                confirmed: true,
                previewToken: preview.preview.previewToken,
                actionReference: action.reference
            ),
            timeoutMS: 120_000
        )
        guard result.preview.action == action else {
            throw ClientError.invalidOutput("Local skill creation response belongs to another action.")
        }
        try requireVerifiedReadback(
            result.readback,
            action: action,
            operation: "Local skill creation"
        )
        return result
    }

    func previewSkillManagerLocalDelete(instanceID: String) async throws -> SkillManagerLocalDeleteRecord {
        let result: SkillManagerLocalDeleteRecord = try await call(
            method: "skillManager.deleteLocal",
            params: SkillManagerDeleteLocalParams(
                instanceId: instanceID,
                confirmed: false,
                previewToken: nil,
                actionReference: nil
            )
        )
        if result.physicalDeleteAllowed {
            guard let action = result.action,
                  let previewToken = result.previewToken,
                  !previewToken.isEmpty else {
                throw ClientError.invalidOutput(
                    "Deletable local skill preview omitted its service-owned action."
                )
            }
            try validateSkillManagerAction(
                action,
                preconditions: result.preconditions,
                previewMethod: "skillManager.deleteLocal",
                applyMethod: "skillManager.deleteLocal",
                kind: "manager_local_delete",
                intent: "manager_local_delete",
                network: "none"
            )
        } else if result.action != nil || result.previewToken != nil {
            throw ClientError.invalidOutput(
                "Blocked local deletion returned an unexpected authorization token."
            )
        }
        return result
    }

    func applySkillManagerLocalDelete(
        preview: SkillManagerLocalDeleteRecord
    ) async throws -> SkillManagerLocalDeleteRecord {
        guard let action = preview.action,
              let previewToken = preview.previewToken,
              !previewToken.isEmpty else {
            throw ClientError.invalidOutput(
                "The local delete preview is missing its typed action confirmation."
            )
        }
        try validateSkillManagerAction(
            action,
            preconditions: preview.preconditions,
            previewMethod: "skillManager.deleteLocal",
            applyMethod: "skillManager.deleteLocal",
            kind: "manager_local_delete",
            intent: "manager_local_delete",
            network: "none"
        )
        let result: SkillManagerLocalDeleteRecord = try await call(
            method: "skillManager.deleteLocal",
            params: SkillManagerDeleteLocalParams(
                instanceId: preview.instanceId,
                confirmed: true,
                previewToken: previewToken,
                actionReference: action.reference
            )
        )
        guard result.action == action else {
            throw ClientError.invalidOutput("Local skill deletion response belongs to another action.")
        }
        try requireVerifiedReadback(
            result.readback,
            action: action,
            operation: "Local skill deletion"
        )
        return result
    }

    func previewSkillManagerLocalArchiveUpdate(
        instanceID: String,
        archivePath: String
    ) async throws -> SkillManagerLocalArchiveUpdateRecord {
        let result: SkillManagerLocalArchiveUpdateRecord = try await call(
            method: "skillManager.previewLocalArchiveUpdate",
            params: SkillManagerLocalArchiveUpdateParams(
                instanceId: instanceID,
                archivePath: archivePath,
                confirmed: false,
                previewToken: nil,
                actionReference: nil
            ),
            timeoutMS: 120_000
        )
        try validateSkillManagerAction(
            result.action,
            preconditions: result.preconditions,
            previewMethod: "skillManager.previewLocalArchiveUpdate",
            applyMethod: "skillManager.applyLocalArchiveUpdate",
            kind: "manager_local_archive_update",
            intent: "manager_local_archive_update",
            network: "none"
        )
        return result
    }

    func previewSkillManagerLocalArchiveImport(
        archivePath: String
    ) async throws -> SkillManagerLocalArchiveImportRecord {
        let result: SkillManagerLocalArchiveImportRecord = try await call(
            method: "skillManager.previewLocalArchiveImport",
            params: SkillManagerLocalArchiveImportParams(
                archivePath: archivePath,
                confirmed: false,
                previewToken: nil,
                actionReference: nil
            ),
            timeoutMS: 120_000
        )
        try validateSkillManagerAction(
            result.action,
            preconditions: result.preconditions,
            previewMethod: "skillManager.previewLocalArchiveImport",
            applyMethod: "skillManager.applyLocalArchiveImport",
            kind: "manager_local_archive_import",
            intent: "manager_local_archive_import",
            network: "none"
        )
        return result
    }

    func applySkillManagerLocalArchiveImport(
        preview: SkillManagerLocalArchiveImportRecord,
        archivePath: String
    ) async throws -> SkillManagerLocalArchiveImportRecord {
        try validateSkillManagerAction(
            preview.action,
            preconditions: preview.preconditions,
            previewMethod: "skillManager.previewLocalArchiveImport",
            applyMethod: "skillManager.applyLocalArchiveImport",
            kind: "manager_local_archive_import",
            intent: "manager_local_archive_import",
            network: "none"
        )
        let result: SkillManagerLocalArchiveImportRecord = try await call(
            method: "skillManager.applyLocalArchiveImport",
            params: SkillManagerLocalArchiveImportParams(
                archivePath: archivePath,
                confirmed: true,
                previewToken: preview.previewToken,
                actionReference: preview.action.reference
            ),
            timeoutMS: 120_000
        )
        guard result.action == preview.action else {
            throw ClientError.invalidOutput("Local ZIP import response belongs to another action.")
        }
        try requireVerifiedReadback(
            result.readback,
            action: preview.action,
            operation: "Local ZIP import"
        )
        return result
    }

    func applySkillManagerLocalArchiveUpdate(
        preview: SkillManagerLocalArchiveUpdateRecord,
        instanceID: String,
        archivePath: String
    ) async throws -> SkillManagerLocalArchiveUpdateRecord {
        try validateSkillManagerAction(
            preview.action,
            preconditions: preview.preconditions,
            previewMethod: "skillManager.previewLocalArchiveUpdate",
            applyMethod: "skillManager.applyLocalArchiveUpdate",
            kind: "manager_local_archive_update",
            intent: "manager_local_archive_update",
            network: "none"
        )
        let result: SkillManagerLocalArchiveUpdateRecord = try await call(
            method: "skillManager.applyLocalArchiveUpdate",
            params: SkillManagerLocalArchiveUpdateParams(
                instanceId: instanceID,
                archivePath: archivePath,
                confirmed: true,
                previewToken: preview.previewToken,
                actionReference: preview.action.reference
            ),
            timeoutMS: 120_000
        )
        guard result.action == preview.action else {
            throw ClientError.invalidOutput("Local ZIP update response belongs to another action.")
        }
        try requireVerifiedReadback(
            result.readback,
            action: preview.action,
            operation: "Local ZIP update"
        )
        return result
    }

    private func requiredAction(
        _ preview: SkillManagerCommandPreview,
        previewMethod: String,
        applyMethod: String,
        kind: String,
        intent: String,
        network: String,
        targetAgent: ActionStringExpectation? = nil,
        targetScope: ActionStringExpectation? = nil,
        projectID: ActionStringExpectation? = nil
    ) throws -> ActionDescriptorWire {
        guard let action = preview.action,
              let previewToken = preview.previewToken,
              !previewToken.isEmpty else {
            throw ClientError.invalidOutput(
                "The Skill Manager preview is missing its typed action confirmation."
            )
        }
        try validateSkillManagerAction(
            action,
            preconditions: preview.preconditions,
            previewMethod: previewMethod,
            applyMethod: applyMethod,
            kind: kind,
            intent: intent,
            network: network,
            targetAgent: targetAgent,
            targetScope: targetScope,
            projectID: projectID
        )
        return action
    }

    private func requiredPreviewToken(
        _ preview: SkillManagerCommandPreview
    ) throws -> String {
        guard let previewToken = preview.previewToken,
              !previewToken.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty else {
            throw ClientError.invalidOutput(
                "The Skill Manager preview is missing its opaque confirmation token."
            )
        }
        return previewToken
    }

    private func validateSkillManagerAction(
        _ action: ActionDescriptorWire,
        preconditions: [ActionPreconditionWire],
        previewMethod: String,
        applyMethod: String,
        kind: String,
        intent: String,
        network: String,
        targetAgent: ActionStringExpectation? = nil,
        targetScope: ActionStringExpectation? = nil,
        projectID: ActionStringExpectation? = nil
    ) throws {
        let expectation: ActionDescriptorExpectation
        let preconditionKinds: Set<String>
        switch kind {
        case "refresh_evidence":
            expectation = ActionDescriptorExpectation(
                kind: kind,
                intent: intent,
                targetKind: "skill",
                targetID: .present,
                targetAgent: targetAgent ?? .absent,
                targetScope: targetScope ?? .optional,
                projectID: projectID ?? .optional,
                impacts: ["read_only", "external_manager", "app_local_data"],
                readback: ["manager_inventory"]
            )
            preconditionKinds = ["source_file"]
        case "manager_install", "manager_remove", "manager_update":
            expectation = ActionDescriptorExpectation(
                kind: kind,
                intent: intent,
                targetKind: "skill",
                targetID: .present,
                targetAgent: targetAgent ?? .absent,
                targetScope: targetScope ?? .oneOf(["agent-global", "agent-project"]),
                projectID: projectID ?? .optional,
                impacts: ["app_local_data", "external_manager", "skill_files"],
                readback: ["catalog_skills", "skill_files", "manager_inventory"]
            )
            preconditionKinds = ["target_file", "source_file"]
        case "manager_local_create":
            expectation = ActionDescriptorExpectation(
                kind: kind,
                intent: intent,
                targetKind: "skill",
                targetID: .present,
                targetAgent: .absent,
                targetScope: .absent,
                projectID: projectID ?? .optional,
                impacts: ["external_manager", "skill_files", "app_local_data"],
                readback: ["catalog_skills", "skill_files"]
            )
            preconditionKinds = ["target_file", "source_file"]
        case "manager_local_delete":
            expectation = ActionDescriptorExpectation(
                kind: kind,
                intent: intent,
                targetKind: "skill",
                targetID: .present,
                targetAgent: .exact("tool-global"),
                targetScope: .exact("tool-global"),
                projectID: .absent,
                impacts: ["skill_files", "app_local_data"],
                readback: ["skill_files", "catalog_skills"]
            )
            preconditionKinds = ["catalog_record", "source_file"]
        case "manager_local_archive_import":
            expectation = ActionDescriptorExpectation(
                kind: kind,
                intent: intent,
                targetKind: "skill",
                targetID: .present,
                targetAgent: .exact("tool-global"),
                targetScope: .exact("tool-global"),
                projectID: .absent,
                impacts: ["app_local_data", "skill_files"],
                readback: ["catalog_skills", "skill_files"]
            )
            preconditionKinds = ["archive", "target_file", "catalog_record"]
        case "manager_local_archive_update":
            expectation = ActionDescriptorExpectation(
                kind: kind,
                intent: intent,
                targetKind: "skill",
                targetID: .present,
                targetAgent: .exact("tool-global"),
                targetScope: .exact("tool-global"),
                projectID: .absent,
                impacts: ["app_local_data", "skill_files"],
                readback: ["catalog_skills", "skill_files"]
            )
            preconditionKinds = ["archive", "source_file", "catalog_record"]
        default:
            throw ClientError.invalidOutput(
                "The Skill Manager preview uses an undeclared action kind."
            )
        }
        do {
            try action.validated(
                previewMethod: previewMethod,
                applyMethod: applyMethod,
                network: network,
                expectation: expectation
            )
            try preconditions.validated(kinds: preconditionKinds)
            if kind == "refresh_evidence" {
                try action.validatedOptionalProjectContextScope()
            }
        } catch {
            throw ClientError.invalidOutput(error.localizedDescription)
        }
    }

    private func requireVerifiedReadback(
        _ readback: ActionReadbackWire?,
        action: ActionDescriptorWire,
        operation: String
    ) throws {
        guard let readback else {
            throw ClientError.invalidOutput(
                "\(operation) returned no action-bound read-back."
            )
        }
        do {
            try readback.validated(for: action)
        } catch {
            throw ClientError.invalidOutput(error.localizedDescription)
        }
    }

    private func agentBinding(for agents: [String]) -> ActionStringExpectation {
        let normalized = Array(Set(agents.filter { !$0.isEmpty })).sorted()
        return normalized.count == 1 ? .exact(normalized[0]) : .absent
    }
}
