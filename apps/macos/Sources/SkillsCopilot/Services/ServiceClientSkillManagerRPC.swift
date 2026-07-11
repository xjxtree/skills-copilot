import Foundation

extension ServiceClient {
    func listSkillManagerTools() async throws -> [SkillManagerToolRecord] {
        try await call(method: "skillManager.listTools", params: EmptyParams())
    }

    func searchSkillManager(query: String, owner: String?, networkAllowed: Bool) async throws -> SkillManagerSearchRecord {
        let result: SkillManagerSearchRecord = try await call(
            method: "skillManager.search",
            params: SkillManagerSearchParams(
                query: query,
                owner: owner?.nilIfBlank,
                networkAllowed: networkAllowed
            ),
            timeoutMS: networkAllowed ? 120_000 : nil
        )
        guard result.hasValidPageMetadata else {
            throw ClientError.invalidOutput("skillManager.search returned inconsistent page metadata")
        }
        return result
    }

    func listSkillManagerInstalled(agents: [String], scope: SkillManagerScope) async throws -> SkillManagerInstalledListRecord {
        let result: SkillManagerInstalledListRecord = try await call(
            method: "skillManager.listInstalled",
            params: SkillManagerListInstalledParams(
                agents: agents,
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
        scope: SkillManagerScope,
        distribution: SkillManagerDistribution,
        networkAllowed: Bool
    ) async throws -> SkillManagerMutationRecord {
        try await skillManagerInstall(
            method: "skillManager.previewInstall",
            source: source,
            skills: skills,
            agents: agents,
            scope: scope,
            distribution: distribution,
            networkAllowed: networkAllowed,
            confirmed: false,
            previewToken: nil
        )
    }

    func applySkillManagerInstall(
        preview: SkillManagerMutationRecord,
        source: String,
        skills: [String],
        agents: [String],
        scope: SkillManagerScope,
        distribution: SkillManagerDistribution,
        networkAllowed: Bool
    ) async throws -> SkillManagerMutationRecord {
        try await skillManagerInstall(
            method: "skillManager.applyInstall",
            source: source,
            skills: skills,
            agents: agents,
            scope: scope,
            distribution: distribution,
            networkAllowed: networkAllowed,
            confirmed: true,
            previewToken: preview.preview.previewToken
        )
    }

    func previewSkillManagerRemove(skill: String, agents: [String], scope: SkillManagerScope) async throws -> SkillManagerMutationRecord {
        try await skillManagerRemove(
            method: "skillManager.previewRemove",
            skill: skill,
            agents: agents,
            scope: scope,
            confirmed: false,
            previewToken: nil
        )
    }

    func applySkillManagerRemove(preview: SkillManagerMutationRecord, skill: String, agents: [String], scope: SkillManagerScope) async throws -> SkillManagerMutationRecord {
        try await skillManagerRemove(
            method: "skillManager.applyRemove",
            skill: skill,
            agents: agents,
            scope: scope,
            confirmed: true,
            previewToken: preview.preview.previewToken
        )
    }

    func previewSkillManagerUpdate(skills: [String], agents: [String], scope: SkillManagerScope, networkAllowed: Bool) async throws -> SkillManagerMutationRecord {
        try await skillManagerUpdate(
            method: "skillManager.previewUpdate",
            skills: skills,
            agents: agents,
            scope: scope,
            networkAllowed: networkAllowed,
            confirmed: false,
            previewToken: nil
        )
    }

    func applySkillManagerUpdate(preview: SkillManagerMutationRecord, skills: [String], agents: [String], scope: SkillManagerScope, networkAllowed: Bool) async throws -> SkillManagerMutationRecord {
        try await skillManagerUpdate(
            method: "skillManager.applyUpdate",
            skills: skills,
            agents: agents,
            scope: scope,
            networkAllowed: networkAllowed,
            confirmed: true,
            previewToken: preview.preview.previewToken
        )
    }

    func previewSkillManagerLocalCreate(name: String) async throws -> SkillManagerLocalCreateRecord {
        try await call(
            method: "skillManager.previewLocalCreate",
            params: SkillManagerLocalCreateParams(
                name: name,
                confirmed: false,
                previewToken: nil
            )
        )
    }

    func applySkillManagerLocalCreate(preview: SkillManagerLocalCreateRecord, name: String) async throws -> SkillManagerLocalCreateRecord {
        try await call(
            method: "skillManager.applyLocalCreate",
            params: SkillManagerLocalCreateParams(
                name: name,
                confirmed: true,
                previewToken: preview.preview.previewToken
            ),
            timeoutMS: 120_000
        )
    }

    func previewSkillManagerLocalDelete(instanceID: String) async throws -> SkillManagerLocalDeleteRecord {
        try await call(
            method: "skillManager.deleteLocal",
            params: SkillManagerDeleteLocalParams(
                instanceId: instanceID,
                confirmed: false
            )
        )
    }

    func applySkillManagerLocalDelete(instanceID: String) async throws -> SkillManagerLocalDeleteRecord {
        try await call(
            method: "skillManager.deleteLocal",
            params: SkillManagerDeleteLocalParams(
                instanceId: instanceID,
                confirmed: true
            )
        )
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
        previewToken: String?
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
                previewToken: previewToken
            ),
            timeoutMS: confirmed ? 180_000 : nil
        )
    }

    private func skillManagerRemove(
        method: String,
        skill: String,
        agents: [String],
        scope: SkillManagerScope,
        confirmed: Bool,
        previewToken: String?
    ) async throws -> SkillManagerMutationRecord {
        try await call(
            method: method,
            params: SkillManagerRemoveParams(
                skill: skill,
                agents: agents,
                scope: scope.rawValue,
                confirmed: confirmed,
                previewToken: previewToken
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
        previewToken: String?
    ) async throws -> SkillManagerMutationRecord {
        try await call(
            method: method,
            params: SkillManagerUpdateParams(
                skills: skills,
                agents: agents,
                scope: scope.rawValue,
                networkAllowed: networkAllowed,
                confirmed: confirmed,
                previewToken: previewToken
            ),
            timeoutMS: confirmed ? 180_000 : nil
        )
    }
}

private extension String {
    var nilIfBlank: String? {
        let trimmed = trimmingCharacters(in: .whitespacesAndNewlines)
        return trimmed.isEmpty ? nil : trimmed
    }
}
