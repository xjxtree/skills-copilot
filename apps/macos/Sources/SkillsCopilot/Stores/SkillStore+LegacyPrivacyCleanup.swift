import Foundation

@MainActor
extension SkillStore {
    func inspectLegacyPrivateContent() async {
        guard !isInspectingLegacyPrivateContent else { return }
        isInspectingLegacyPrivateContent = true
        defer { isInspectingLegacyPrivateContent = false }

        do {
            let inspection = try await service.inspectLegacyPrivateContent()
            legacyPrivateContentInspection = inspection
            legacyPrivateContentCleanupError = nil
            if legacyPrivateContentCleanupPreview?.inspection != inspection {
                legacyPrivateContentCleanupPreview = nil
            }
        } catch {
            legacyPrivateContentCleanupError = UIStrings.legacyPrivateContentInspectionFailed
        }
    }

    func previewLegacyPrivateContentCleanup() async {
        guard !isPreviewingLegacyPrivateContentCleanup,
              !isCleaningLegacyPrivateContent else {
            return
        }
        isPreviewingLegacyPrivateContentCleanup = true
        legacyPrivateContentCleanupError = nil
        defer { isPreviewingLegacyPrivateContentCleanup = false }

        do {
            let preview = try await service.previewLegacyPrivateContentCleanup()
            legacyPrivateContentInspection = preview.inspection
            guard preview.inspection.cleanupRequired,
                  preview.confirmation != nil else {
                legacyPrivateContentCleanupPreview = nil
                return
            }
            legacyPrivateContentCleanupPreview = preview
        } catch {
            legacyPrivateContentCleanupPreview = nil
            legacyPrivateContentCleanupError = UIStrings.legacyPrivateContentPreviewFailed
        }
    }

    func cancelLegacyPrivateContentCleanupPreview() {
        guard !isCleaningLegacyPrivateContent else { return }
        legacyPrivateContentCleanupPreview = nil
    }

    func confirmLegacyPrivateContentCleanup() async {
        guard !isCleaningLegacyPrivateContent,
              let preview = legacyPrivateContentCleanupPreview,
              preview.confirmation != nil else {
            return
        }
        isCleaningLegacyPrivateContent = true
        legacyPrivateContentCleanupError = nil
        legacyPrivateContentCleanupPreview = nil
        defer { isCleaningLegacyPrivateContent = false }

        do {
            let result = try await service.cleanupLegacyPrivateContent(preview: preview)
            legacyPrivateContentInspection = result.inspection
        } catch {
            await inspectLegacyPrivateContent()
            legacyPrivateContentCleanupError = UIStrings.legacyPrivateContentCleanupFailed
        }
    }
}
