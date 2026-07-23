import Foundation

struct ConfigSavePreviewRecord: Codable, Hashable {
    let action: ActionDescriptorWire
    let preconditions: [ActionPreconditionWire]
    let previewToken: String
    let current: ConfigDocumentRecord
    let candidateContentDigest: String
    let currentRevision: String
    let changed: Bool

    enum CodingKeys: String, CodingKey {
        case action
        case preconditions
        case previewToken = "preview_token"
        case current
        case candidateContentDigest = "candidate_content_digest"
        case currentRevision = "current_revision"
        case changed
    }

    var confirmation: ActionConfirmationWire {
        ActionConfirmationWire(action: action, previewToken: previewToken)
    }
}

struct ConfigSaveApplyRecord: Codable, Hashable {
    let action: ActionDescriptorWire
    let document: ConfigDocumentRecord
    let snapshotID: String
    let readback: ActionReadbackRecordWire

    enum CodingKeys: String, CodingKey {
        case action
        case document
        case snapshotID = "snapshot_id"
        case readback
    }
}

struct SnapshotRollbackPreviewRecord: Codable, Identifiable, Hashable {
    let action: ActionDescriptorWire
    let preconditions: [ActionPreconditionWire]
    let previewToken: String
    let snapshot: ConfigSnapshotRecord
    let snapshotContentDigest: String
    let currentContent: String
    let currentReadError: String?
    let currentRevision: String
    let changed: Bool
    let redacted: Bool
    let rollbackSupported: Bool

    var id: String { snapshot.id }

    enum CodingKeys: String, CodingKey {
        case action
        case preconditions
        case previewToken = "preview_token"
        case snapshot
        case snapshotContentDigest = "snapshot_content_digest"
        case currentContent = "current_content"
        case currentReadError = "current_read_error"
        case currentRevision = "current_revision"
        case changed
        case redacted
        case rollbackSupported = "rollback_supported"
    }

    init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        action = try container.decode(ActionDescriptorWire.self, forKey: .action)
        preconditions = try container.decode([ActionPreconditionWire].self, forKey: .preconditions)
        previewToken = try container.decode(String.self, forKey: .previewToken)
        snapshot = try container.decode(ConfigSnapshotRecord.self, forKey: .snapshot)
        snapshotContentDigest = try container.decode(String.self, forKey: .snapshotContentDigest)
        currentContent = try container.decode(String.self, forKey: .currentContent)
        currentReadError = try container.decodeIfPresent(String.self, forKey: .currentReadError)
        currentRevision = try container.decode(String.self, forKey: .currentRevision)
        changed = try container.decode(Bool.self, forKey: .changed)
        redacted = try container.decodeIfPresent(Bool.self, forKey: .redacted) ?? false
        let serviceSupportsRollback =
            try container.decodeIfPresent(Bool.self, forKey: .rollbackSupported) ?? !redacted
        rollbackSupported = serviceSupportsRollback
            && !currentRevision.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
            && !previewToken.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
            && action.confirmationRequired
            && action.applyMethod == "snapshot.rollback"
    }
}

struct SnapshotRollbackApplyRecord: Codable, Hashable {
    let action: ActionDescriptorWire
    let snapshotID: String
    let document: ConfigDocumentRecord
    let readback: ActionReadbackRecordWire

    enum CodingKeys: String, CodingKey {
        case action
        case snapshotID = "snapshot_id"
        case document
        case readback
    }
}

struct ConfigConflictState: Equatable {
    let attemptedRevision: String
    let latestRevision: String?
    let displayMessage: String
}

enum ConfigMutationState: Equatable {
    case idle
    case previewing
    case awaitingConfirmation
    case saving
    case conflict(ConfigConflictState)
    case failed(String)
}

struct ConfigSaveConfirmation: Identifiable, Hashable {
    let content: String
    let preview: ConfigSavePreviewRecord

    var id: String { preview.action.id }
    var wire: ActionConfirmationWire { preview.confirmation }
}

struct RollbackConfirmation: Identifiable, Hashable {
    let snapshotID: String
    let action: ActionDescriptorWire
    let previewToken: String

    var id: String { snapshotID }
    var wire: ActionConfirmationWire {
        ActionConfirmationWire(action: action, previewToken: previewToken)
    }

    init(snapshotID: String, action: ActionDescriptorWire, previewToken: String) {
        self.snapshotID = snapshotID
        self.action = action
        self.previewToken = previewToken
    }

    init?(preview: SnapshotRollbackPreviewRecord) {
        guard preview.rollbackSupported,
              !preview.previewToken.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty else {
            return nil
        }
        self.init(
            snapshotID: preview.snapshot.id,
            action: preview.action,
            previewToken: preview.previewToken
        )
    }
}
