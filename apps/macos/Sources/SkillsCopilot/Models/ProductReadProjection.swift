import Foundation

struct EvidenceRef: Codable, Hashable, Identifiable {
    let id: String
    let kind: EvidenceKind
    let sourceRevision: String
    let summary: String
    let agent: ProductAgentID?
    let targetID: String?

    enum CodingKeys: String, CodingKey {
        case id
        case kind
        case sourceRevision = "source_revision"
        case summary
        case agent
        case targetID = "target_id"
    }

    @discardableResult
    func validated() throws -> EvidenceRef {
        try requireProductText(id, "evidence id is empty")
        try requireProductText(sourceRevision, "evidence source revision is empty")
        try requireSafeProductSummary(summary, "evidence summary is empty")
        if let targetID {
            try requireProductText(targetID, "evidence target id is empty")
        }
        return self
    }
}

extension ListIncompleteReason {
    var productSeverityRank: Int {
        switch self {
        case .unsupportedProtocol: 0
        case .pageFailed: 1
        case .sourceChanged: 2
        case .sourceLimited: 3
        case .unreadableSource: 4
        case .notInspected: 5
        case .staleSource: 6
        case .safetyBudget: 7
        }
    }
}

func validateProductProjectionLinks(
    evidence: [EvidenceRef],
    evidenceRevision: String,
    actions: [ActionDescriptorWire],
    actionRevision: String
) throws {
    let evidenceIDs = Set(evidence.map(\.id))
    guard evidenceIDs.count == evidence.count else {
        throw ProductProjectionValidationError.invalid("projection evidence ids are duplicated")
    }
    for reference in evidence {
        try reference.validated()
        guard reference.sourceRevision == evidenceRevision else {
            throw ProductProjectionValidationError.invalid(
                "evidence revision differs from its projection"
            )
        }
    }
    guard Set(actions.map(\.id)).count == actions.count else {
        throw ProductProjectionValidationError.invalid("projection action ids are duplicated")
    }
    for action in actions {
        do {
            try action.validatedProjection(
                sourceRevision: actionRevision,
                evidenceIDs: evidenceIDs
            )
        } catch {
            throw ProductProjectionValidationError.invalid(
                "projection action is not bound to its evidence and revision"
            )
        }
    }
}

func requireProductText(_ value: String, _ message: String) throws {
    guard !value.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty,
          !value.unicodeScalars.contains(where: CharacterSet.controlCharacters.contains) else {
        throw ProductProjectionValidationError.invalid(message)
    }
}

func requireSafeProductSummary(_ value: String, _ message: String) throws {
    try requireProductText(value, message)
    let unsafe = value.contains("file://")
        || value.contains(#"\\"#)
        || value.split(whereSeparator: \.isWhitespace).contains(where: { word in
            let token = word.trimmingCharacters(
                in: CharacterSet(charactersIn: "()[]{}\"',;")
            )
            if token.count > 1, token.hasPrefix("/") {
                return true
            }
            let bytes = Array(token.utf8)
            return bytes.count > 2
                && bytes[1] == 58
                && (bytes[2] == 47 || bytes[2] == 92)
        })
    guard !unsafe else {
        throw ProductProjectionValidationError.invalid(
            "projection summary contains a raw absolute path"
        )
    }
}

func requireSafeProductIdentity(_ value: String, _ message: String) throws {
    try requireSafeProductSummary(value, message)
    guard !value.contains("/"), !value.contains("\\") else {
        throw ProductProjectionValidationError.invalid(
            "projection identity contains a path separator"
        )
    }
}

func requireUniqueProductIDs(
    _ values: [String],
    allowEmpty: Bool,
    label: String
) throws {
    guard allowEmpty || !values.isEmpty,
          Set(values).count == values.count,
          values.allSatisfy({
              !$0.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
          }) else {
        throw ProductProjectionValidationError.invalid("\(label) are invalid")
    }
}

func requireKnownProductIDs(
    _ values: [String],
    known: Set<String>,
    allowEmpty: Bool = true,
    label: String
) throws {
    try requireUniqueProductIDs(values, allowEmpty: allowEmpty, label: label)
    guard values.allSatisfy(known.contains) else {
        throw ProductProjectionValidationError.invalid("\(label) contain an unknown id")
    }
}
