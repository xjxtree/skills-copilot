import Foundation

struct ConfigConflictState: Equatable {
    let attemptedRevision: String
    let latestRevision: String?
    let displayMessage: String
}

enum ConfigMutationState: Equatable {
    case idle
    case saving
    case conflict(ConfigConflictState)
    case failed(String)
}

struct RollbackConfirmation: Identifiable, Hashable {
    let snapshotID: String
    let previewToken: String

    var id: String { snapshotID }

    init(snapshotID: String, previewToken: String) {
        self.snapshotID = snapshotID
        self.previewToken = previewToken
    }

    init?(preview: SnapshotRollbackPreviewRecord) {
        guard preview.rollbackSupported,
              let previewToken = preview.previewToken,
              !previewToken.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty else {
            return nil
        }
        self.init(snapshotID: preview.snapshot.id, previewToken: previewToken)
    }
}
