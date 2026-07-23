import Foundation

struct ActionTargetWire: Codable, Hashable {
    let kind: String
    let id: String
    let agent: String?
    let scope: String?
}

struct ActionDescriptorWire: Codable, Hashable {
    let id: String
    let kind: String
    let intent: String
    let target: ActionTargetWire
    let projectID: String?
    let impacts: [String]
    let previewMethod: String
    let applyMethod: String?
    let sourceRevision: String
    let confirmationRequired: Bool
    let network: String
    let readback: [String]
    let evidenceRefs: [String]

    enum CodingKeys: String, CodingKey {
        case id
        case kind
        case intent
        case target
        case projectID = "project_id"
        case impacts
        case previewMethod = "preview_method"
        case applyMethod = "apply_method"
        case sourceRevision = "source_revision"
        case confirmationRequired = "confirmation_required"
        case network
        case readback
        case evidenceRefs = "evidence_refs"
    }

    var reference: ActionReferenceWire {
        ActionReferenceWire(
            actionID: id,
            sourceRevision: sourceRevision,
            projectID: projectID,
            target: target
        )
    }
}

struct ActionReferenceWire: Codable, Hashable {
    let actionID: String
    let sourceRevision: String
    let projectID: String?
    let target: ActionTargetWire

    enum CodingKeys: String, CodingKey {
        case actionID = "action_id"
        case sourceRevision = "source_revision"
        case projectID = "project_id"
        case target
    }
}

struct ActionConfirmationWire: Codable, Hashable {
    let reference: ActionReferenceWire
    let previewToken: String
    let confirmed: Bool

    enum CodingKeys: String, CodingKey {
        case reference
        case previewToken = "preview_token"
        case confirmed
    }

    init(action: ActionDescriptorWire, previewToken: String) {
        reference = action.reference
        self.previewToken = previewToken
        confirmed = true
    }
}
