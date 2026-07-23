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
        _ = try requiredAction(result.preview)
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
        let action = try requiredAction(preview.preview)
        let result: SkillManagerSearchRecord = try await call(
            method: "skillManager.applySearch",
            params: SkillManagerSearchApplyParams(
                query: query,
                owner: owner,
                networkAllowed: true,
                confirmed: true,
                previewToken: preview.preview.previewToken,
                actionReference: action.reference
            ),
            timeoutMS: 120_000
        )
        guard result.hasValidPageMetadata else {
            throw ClientError.invalidOutput("skillManager.applySearch returned inconsistent page metadata")
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
        return try await skillManagerInstall(
            method: "skillManager.previewInstall",
            source: source,
            skills: skills,
            agents: agents,
            scope: scope,
            distribution: .symlink,
            networkAllowed: true,
            confirmed: false,
            previewToken: nil,
            actionReference: nil
        )
    }

    func applySkillManagerInstall(
        preview: SkillManagerMutationRecord,
        source: String,
        skills: [String],
        agents: [String],
        scope: SkillManagerScope
    ) async throws -> SkillManagerMutationRecord {
        let actionReference = try requiredActionReference(preview.preview)
        let result = try await skillManagerInstall(
            method: "skillManager.applyInstall",
            source: source,
            skills: skills,
            agents: agents,
            scope: scope,
            distribution: .symlink,
            networkAllowed: true,
            confirmed: true,
            previewToken: preview.preview.previewToken,
            actionReference: actionReference
        )
        try requireVerifiedReadback(
            result.readback,
            action: try requiredAction(preview.preview),
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
        return try await skillManagerRemove(
            method: "skillManager.previewRemove",
            skill: skill,
            agents: agents,
            scope: scope,
            cleanupLocalInstanceID: cleanupLocalInstanceID,
            networkAllowed: true,
            confirmed: false,
            previewToken: nil,
            actionReference: nil
        )
    }

    func applySkillManagerRemove(
        preview: SkillManagerMutationRecord,
        skill: String,
        agents: [String],
        scope: SkillManagerScope,
        cleanupLocalInstanceID: String?
    ) async throws -> SkillManagerMutationRecord {
        let actionReference = try requiredActionReference(preview.preview)
        let result = try await skillManagerRemove(
            method: "skillManager.applyRemove",
            skill: skill,
            agents: agents,
            scope: scope,
            cleanupLocalInstanceID: cleanupLocalInstanceID,
            networkAllowed: true,
            confirmed: true,
            previewToken: preview.preview.previewToken,
            actionReference: actionReference
        )
        try requireVerifiedReadback(
            result.readback,
            action: try requiredAction(preview.preview),
            operation: "Skill Manager remove"
        )
        return result
    }

    func previewSkillManagerUpdate(skills: [String], scope: SkillManagerScope) async throws -> SkillManagerMutationRecord {
        return try await skillManagerUpdate(
            method: "skillManager.previewUpdate",
            skills: skills,
            agents: [],
            scope: scope,
            networkAllowed: true,
            confirmed: false,
            previewToken: nil,
            actionReference: nil
        )
    }

    func applySkillManagerUpdate(preview: SkillManagerMutationRecord, skills: [String], scope: SkillManagerScope) async throws -> SkillManagerMutationRecord {
        let actionReference = try requiredActionReference(preview.preview)
        let result = try await skillManagerUpdate(
            method: "skillManager.applyUpdate",
            skills: skills,
            agents: [],
            scope: scope,
            networkAllowed: true,
            confirmed: true,
            previewToken: preview.preview.previewToken,
            actionReference: actionReference
        )
        try requireVerifiedReadback(
            result.readback,
            action: try requiredAction(preview.preview),
            operation: "Skill Manager update"
        )
        return result
    }

    func previewSkillManagerLocalCreate(name: String) async throws -> SkillManagerLocalCreateRecord {
        return try await call(
            method: "skillManager.previewLocalCreate",
            params: SkillManagerLocalCreateParams(
                name: name,
                networkAllowed: true,
                confirmed: false,
                previewToken: nil,
                actionReference: nil
            )
        )
    }

    func applySkillManagerLocalCreate(preview: SkillManagerLocalCreateRecord, name: String) async throws -> SkillManagerLocalCreateRecord {
        let actionReference = try requiredActionReference(preview.preview)
        let result: SkillManagerLocalCreateRecord = try await call(
            method: "skillManager.applyLocalCreate",
            params: SkillManagerLocalCreateParams(
                name: name,
                networkAllowed: true,
                confirmed: true,
                previewToken: preview.preview.previewToken,
                actionReference: actionReference
            ),
            timeoutMS: 120_000
        )
        try requireVerifiedReadback(
            result.readback,
            action: try requiredAction(preview.preview),
            operation: "Local skill creation"
        )
        return result
    }

    func previewSkillManagerLocalDelete(instanceID: String) async throws -> SkillManagerLocalDeleteRecord {
        try await call(
            method: "skillManager.deleteLocal",
            params: SkillManagerDeleteLocalParams(
                instanceId: instanceID,
                confirmed: false,
                previewToken: nil,
                actionReference: nil
            )
        )
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
        let result: SkillManagerLocalDeleteRecord = try await call(
            method: "skillManager.deleteLocal",
            params: SkillManagerDeleteLocalParams(
                instanceId: preview.instanceId,
                confirmed: true,
                previewToken: previewToken,
                actionReference: action.reference
            )
        )
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
        try await call(
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
    }

    func previewSkillManagerLocalArchiveImport(
        archivePath: String
    ) async throws -> SkillManagerLocalArchiveImportRecord {
        try await call(
            method: "skillManager.previewLocalArchiveImport",
            params: SkillManagerLocalArchiveImportParams(
                archivePath: archivePath,
                confirmed: false,
                previewToken: nil,
                actionReference: nil
            ),
            timeoutMS: 120_000
        )
    }

    func applySkillManagerLocalArchiveImport(
        preview: SkillManagerLocalArchiveImportRecord,
        archivePath: String
    ) async throws -> SkillManagerLocalArchiveImportRecord {
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
        try requireVerifiedReadback(
            result.readback,
            action: preview.action,
            operation: "Local ZIP update"
        )
        return result
    }

    private func skillManagerInstall(
        method: String,
        source: String,
        skills: [String],
        agents: [String],
        scope: SkillManagerScope,
        distribution: SkillManagerDistribution,
        networkAllowed: Bool,
        confirmed: Bool,
        previewToken: String?,
        actionReference: ActionReferenceWire?
    ) async throws -> SkillManagerMutationRecord {
        try await call(
            method: method,
            params: SkillManagerInstallParams(
                source: source,
                skills: skills,
                agents: agents,
                scope: scope.rawValue,
                distribution: distribution == .copy ? distribution.rawValue : nil,
                networkAllowed: networkAllowed,
                confirmed: confirmed,
                previewToken: previewToken,
                actionReference: actionReference
            ),
            timeoutMS: confirmed ? 180_000 : nil
        )
    }

    private func skillManagerRemove(
        method: String,
        skill: String,
        agents: [String],
        scope: SkillManagerScope,
        cleanupLocalInstanceID: String?,
        networkAllowed: Bool,
        confirmed: Bool,
        previewToken: String?,
        actionReference: ActionReferenceWire?
    ) async throws -> SkillManagerMutationRecord {
        try await call(
            method: method,
            params: SkillManagerRemoveParams(
                skill: skill,
                agents: agents,
                scope: scope.rawValue,
                cleanupLocalInstanceID: cleanupLocalInstanceID,
                networkAllowed: networkAllowed,
                confirmed: confirmed,
                previewToken: previewToken,
                actionReference: actionReference
            ),
            timeoutMS: confirmed ? 120_000 : nil
        )
    }

    private func skillManagerUpdate(
        method: String,
        skills: [String],
        agents: [String],
        scope: SkillManagerScope,
        networkAllowed: Bool,
        confirmed: Bool,
        previewToken: String?,
        actionReference: ActionReferenceWire?
    ) async throws -> SkillManagerMutationRecord {
        try await call(
            method: method,
            params: SkillManagerUpdateParams(
                skills: skills,
                agents: agents,
                scope: scope.rawValue,
                networkAllowed: networkAllowed,
                confirmed: confirmed,
                previewToken: previewToken,
                actionReference: actionReference
            ),
            timeoutMS: confirmed ? 180_000 : nil
        )
    }

    private func requiredActionReference(
        _ preview: SkillManagerCommandPreview
    ) throws -> ActionReferenceWire {
        try requiredAction(preview).reference
    }

    private func requiredAction(
        _ preview: SkillManagerCommandPreview
    ) throws -> ActionDescriptorWire {
        guard let action = preview.action,
              !preview.previewToken.isEmpty else {
            throw ClientError.invalidOutput(
                "The Skill Manager preview is missing its typed action confirmation."
            )
        }
        return action
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
}
