import Foundation

enum SkillManagerRequestKey: Hashable {
    case search(query: String, owner: String?, networkAllowed: Bool)
    case installedInventory
    case mutation(SkillManagerMutationInputs)
    case localCreate(name: String)
    case localDelete(instanceID: String)
    case localArchiveImport(archivePath: String)
    case localArchiveUpdate(instanceID: String, archivePath: String)
}

struct SkillManagerMutationInputs: Hashable {
    enum Kind: String, Hashable {
        case install
        case remove
        case update
    }

    let kind: Kind
    let source: String?
    let skills: [String]
    let agents: [String]
    let scope: SkillManagerScope
    let distribution: SkillManagerDistribution?
    let networkAllowed: Bool
    let cleanupLocalInstanceID: String?

    init(
        kind: Kind,
        source: String?,
        skills: [String],
        agents: [String],
        scope: SkillManagerScope,
        distribution: SkillManagerDistribution?,
        networkAllowed: Bool,
        cleanupLocalInstanceID: String? = nil
    ) {
        self.kind = kind
        self.source = source?.trimmingCharacters(in: .whitespacesAndNewlines)
        self.skills = Self.canonicalValues(skills)
        self.agents = Self.canonicalValues(agents)
        self.scope = scope
        self.distribution = distribution
        self.networkAllowed = networkAllowed
        self.cleanupLocalInstanceID = cleanupLocalInstanceID
    }

    private static func canonicalValues(_ values: [String]) -> [String] {
        Array(
            Set(
                values
                    .map { $0.trimmingCharacters(in: .whitespacesAndNewlines) }
                    .filter { !$0.isEmpty }
            )
        ).sorted()
    }
}

struct SkillManagerMutationConfirmation: Hashable {
    let generation: SkillManagerRequestGeneration
    let inputs: SkillManagerMutationInputs
    let result: SkillManagerMutationRecord

    var previewToken: String { result.preview.previewToken ?? "" }
}

struct SkillManagerSearchConfirmation: Hashable {
    let generation: SkillManagerRequestGeneration
    let query: String
    let owner: String?
    let result: SkillManagerSearchRecord
}

struct SkillManagerLocalCreateConfirmation: Hashable {
    let generation: SkillManagerRequestGeneration
    let name: String
    let result: SkillManagerLocalCreateRecord

    var previewToken: String { result.preview.previewToken ?? "" }
}

struct SkillManagerLocalDeleteConfirmation: Hashable {
    let generation: SkillManagerRequestGeneration
    let instanceID: String
    let result: SkillManagerLocalDeleteRecord
}

struct SkillManagerLocalArchiveUpdateConfirmation: Hashable {
    let generation: SkillManagerRequestGeneration
    let instanceID: String
    let archivePath: String
    let result: SkillManagerLocalArchiveUpdateRecord
}

struct SkillManagerLocalArchiveImportConfirmation: Hashable {
    let generation: SkillManagerRequestGeneration
    let archivePath: String
    let result: SkillManagerLocalArchiveImportRecord
}

struct SkillManagerRequestGeneration: Hashable {
    let value: UInt64
    let key: SkillManagerRequestKey
}
