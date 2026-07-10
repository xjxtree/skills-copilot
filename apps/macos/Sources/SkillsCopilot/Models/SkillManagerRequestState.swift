import Foundation

enum SkillManagerRequestKey: Hashable {
    case search(query: String, owner: String?, networkAllowed: Bool)
    case installed(agents: [String], scope: SkillManagerScope)
    case mutation(SkillManagerMutationInputs)
    case localCreate(name: String)
    case localDelete(instanceID: String)
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

    init(
        kind: Kind,
        source: String?,
        skills: [String],
        agents: [String],
        scope: SkillManagerScope,
        distribution: SkillManagerDistribution?,
        networkAllowed: Bool
    ) {
        self.kind = kind
        self.source = source?.trimmingCharacters(in: .whitespacesAndNewlines)
        self.skills = Self.canonicalValues(skills)
        self.agents = Self.canonicalValues(agents)
        self.scope = scope
        self.distribution = distribution
        self.networkAllowed = networkAllowed
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
    let inputs: SkillManagerMutationInputs
    let result: SkillManagerMutationRecord

    var previewToken: String { result.preview.previewToken }
}

struct SkillManagerLocalCreateConfirmation: Hashable {
    let name: String
    let result: SkillManagerLocalCreateRecord

    var previewToken: String { result.preview.previewToken }
}

struct SkillManagerLocalDeleteConfirmation: Hashable {
    let instanceID: String
    let result: SkillManagerLocalDeleteRecord
}

struct SkillManagerRequestGeneration: Equatable {
    let value: UInt64
    let key: SkillManagerRequestKey
}
