struct RollbackPreviewRequestIdentity: Equatable {
    let snapshotID: String
    let generation: UInt64
}

struct AgentConfigSensitiveTogglePolicy: Equatable {
    let isSensitiveVisible: Bool
    let hasLoadedDocument: Bool
    let hasWritableBinding: Bool
    let isLoading: Bool
    let isSaving: Bool

    var isDisabled: Bool {
        guard !isSensitiveVisible else { return false }
        return isLoading
            || isSaving
            || (hasLoadedDocument && !hasWritableBinding)
    }
}

struct RollbackPreviewPresentationState<Preview: Equatable>: Equatable {
    private(set) var preview: Preview?
    private(set) var errorMessage: String?
    private(set) var selectedSnapshotID: String?
    private(set) var activeRequest: RollbackPreviewRequestIdentity?
    private var generation: UInt64 = 0

    mutating func begin(snapshotID: String) -> RollbackPreviewRequestIdentity {
        generation &+= 1
        let request = RollbackPreviewRequestIdentity(
            snapshotID: snapshotID,
            generation: generation
        )
        selectedSnapshotID = snapshotID
        activeRequest = request
        preview = nil
        errorMessage = nil
        return request
    }

    mutating func invalidate(selectedSnapshotID: String?) {
        generation &+= 1
        self.selectedSnapshotID = selectedSnapshotID
        activeRequest = nil
        preview = nil
        errorMessage = nil
    }

    mutating func replaceWithError(_ message: String?, selectedSnapshotID: String) {
        guard self.selectedSnapshotID == selectedSnapshotID else { return }
        invalidate(selectedSnapshotID: selectedSnapshotID)
        errorMessage = message
    }

    @discardableResult
    mutating func publish(
        preview: Preview,
        for request: RollbackPreviewRequestIdentity
    ) -> Bool {
        guard consume(request) else { return false }
        self.preview = preview
        errorMessage = nil
        return true
    }

    @discardableResult
    mutating func publish(
        errorMessage: String,
        for request: RollbackPreviewRequestIdentity
    ) -> Bool {
        guard consume(request) else { return false }
        preview = nil
        self.errorMessage = errorMessage
        return true
    }

    private mutating func consume(_ request: RollbackPreviewRequestIdentity) -> Bool {
        guard activeRequest == request,
              selectedSnapshotID == request.snapshotID else {
            return false
        }
        activeRequest = nil
        return true
    }
}
