import Foundation

struct LegacyPrivateContentSource: Codable, Hashable, Identifiable {
    let id: String
    let sourceFile: String
    let itemType: String
    let state: String
    let cleanupOperation: String
    let cleanupRequired: Bool
    let malformed: Bool
    let generatedResidue: Bool

    enum CodingKeys: String, CodingKey {
        case id
        case sourceFile = "source_file"
        case itemType = "item_type"
        case state
        case cleanupOperation = "cleanup_operation"
        case cleanupRequired = "cleanup_required"
        case malformed
        case generatedResidue = "generated_residue"
    }
}

struct LegacyPrivateContentInspection: Codable, Hashable {
    let generatedBy: String
    let cleanupRequired: Bool
    let cleanupSourceCount: Int
    let existingSourceCount: Int
    let sources: [LegacyPrivateContentSource]
    let readOnly: Bool
    let providerRequestSent: Bool
    let rawContentReturned: Bool
    let writePerformed: Bool

    enum CodingKeys: String, CodingKey {
        case generatedBy = "generated_by"
        case cleanupRequired = "cleanup_required"
        case cleanupSourceCount = "cleanup_source_count"
        case existingSourceCount = "existing_source_count"
        case sources
        case readOnly = "read_only"
        case providerRequestSent = "provider_request_sent"
        case rawContentReturned = "raw_content_returned"
        case writePerformed = "write_performed"
    }

    var cleanupSources: [LegacyPrivateContentSource] {
        sources.filter(\.cleanupRequired)
    }

    var taskPreflightCleanupRequired: Bool {
        cleanupSources.contains { $0.sourceFile == "task-preflight-history.json" }
    }
}

struct LegacyPrivateContentCleanupPreview: Codable, Hashable {
    let inspection: LegacyPrivateContentInspection
    let action: ActionDescriptorWire?
    let preconditions: [ActionPreconditionWire]
    let previewToken: String?
    let confirmationRequired: Bool

    enum CodingKeys: String, CodingKey {
        case inspection
        case action
        case preconditions
        case previewToken = "preview_token"
        case confirmationRequired = "confirmation_required"
    }

    var confirmation: ActionConfirmationWire? {
        guard confirmationRequired,
              let action,
              let previewToken,
              !previewToken.isEmpty else {
            return nil
        }
        return ActionConfirmationWire(action: action, previewToken: previewToken)
    }
}

struct LegacyPrivateContentCleanupResult: Codable, Hashable {
    let inspection: LegacyPrivateContentInspection
    let cleanedSourceCount: Int
    let state: String
    let effect: String
    let retryAllowed: Bool
    let readback: ActionReadbackWire

    enum CodingKeys: String, CodingKey {
        case inspection
        case cleanedSourceCount = "cleaned_source_count"
        case state
        case effect
        case retryAllowed = "retry_allowed"
        case readback
    }
}
