import Foundation

struct ResumeCapability: Codable, Hashable {
    let state: ResumeCapabilityState
    let argv: [String]
    let unsupportedReason: ResumeUnsupportedReason?
    let copyOnly: Bool

    enum CodingKeys: String, CodingKey {
        case state
        case argv
        case unsupportedReason = "unsupported_reason"
        case copyOnly = "copy_only"
    }

    init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        state = try container.decode(ResumeCapabilityState.self, forKey: .state)
        argv = try container.decodeIfPresent([String].self, forKey: .argv) ?? []
        unsupportedReason = try container.decodeIfPresent(
            ResumeUnsupportedReason.self,
            forKey: .unsupportedReason
        )
        copyOnly = try container.decode(Bool.self, forKey: .copyOnly)
    }

    @discardableResult
    func validated() throws -> ResumeCapability {
        guard copyOnly else {
            throw ProductProjectionValidationError.invalid(
                "session continuation is not copy-only"
            )
        }
        switch state {
        case .supported:
            guard !argv.isEmpty,
                  argv.allSatisfy({ !$0.isEmpty }),
                  unsupportedReason == nil else {
                throw ProductProjectionValidationError.invalid(
                    "supported resume capability is incomplete"
                )
            }
        case .unsupported:
            guard argv.isEmpty, unsupportedReason != nil else {
                throw ProductProjectionValidationError.invalid(
                    "unsupported resume capability exposes argv"
                )
            }
        }
        return self
    }
}

struct SessionContinuationRecord: Decodable, Hashable, Identifiable {
    let id: String
    let agent: ProductAgentID
    let projectID: String?
    let title: String
    let intent: String?
    let startedAt: Int64?
    let endedAt: Int64?
    let modifiedAt: Int64
    let sourceKind: String
    let sourceRevision: String
    let snapshotRevision: String
    let coverage: SourceCoverage
    let resume: ResumeCapability
    let evidence: [EvidenceRef]
    let actions: [ActionDescriptorWire]

    enum CodingKeys: String, CodingKey {
        case id
        case agent
        case projectID = "project_id"
        case title
        case intent
        case startedAt = "started_at"
        case endedAt = "ended_at"
        case modifiedAt = "modified_at"
        case sourceKind = "source_kind"
        case sourceRevision = "source_revision"
        case snapshotRevision = "snapshot_revision"
        case coverage
        case resume
        case evidence
        case actions
    }

    init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        id = try container.decode(String.self, forKey: .id)
        agent = try container.decode(ProductAgentID.self, forKey: .agent)
        projectID = try container.decodeIfPresent(String.self, forKey: .projectID)
        title = try container.decode(String.self, forKey: .title)
        intent = try container.decodeIfPresent(String.self, forKey: .intent)
        startedAt = try container.decodeIfPresent(Int64.self, forKey: .startedAt)
        endedAt = try container.decodeIfPresent(Int64.self, forKey: .endedAt)
        modifiedAt = try container.decode(Int64.self, forKey: .modifiedAt)
        sourceKind = try container.decode(String.self, forKey: .sourceKind)
        sourceRevision = try container.decode(String.self, forKey: .sourceRevision)
        snapshotRevision = try container.decode(String.self, forKey: .snapshotRevision)
        coverage = try container.decode(SourceCoverage.self, forKey: .coverage)
        resume = try container.decode(ResumeCapability.self, forKey: .resume)
        evidence = try container.decodeIfPresent([EvidenceRef].self, forKey: .evidence) ?? []
        actions = try container.decodeIfPresent([ActionDescriptorWire].self, forKey: .actions) ?? []
        try validate()
    }

    private func validate() throws {
        try requireProductText(id, "session continuation id is empty")
        try requireProductText(title, "session continuation title is empty")
        try requireProductText(sourceKind, "session continuation source kind is empty")
        try requireProductText(sourceRevision, "session native revision is empty")
        try requireProductText(snapshotRevision, "session snapshot revision is empty")
        if let projectID {
            try requireSafeProductIdentity(projectID, "session project id is empty")
        }
        try coverage.validated()
        try resume.validated()
        guard coverage.isComplete || resume.state != .supported else {
            throw ProductProjectionValidationError.invalid(
                "incomplete session exposes supported resume"
            )
        }
        try validateProductProjectionLinks(
            evidence: evidence,
            evidenceRevision: sourceRevision,
            actions: actions,
            actionRevision: snapshotRevision
        )
    }
}
