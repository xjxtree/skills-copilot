import Foundation

private struct LegacyPrivateContentCleanupParams: Encodable {
    let actionConfirmation: ActionConfirmationWire

    enum CodingKeys: String, CodingKey {
        case actionConfirmation = "action_confirmation"
    }
}

extension ServiceClient {
    func inspectLegacyPrivateContent() async throws -> LegacyPrivateContentInspection {
        let inspection: LegacyPrivateContentInspection = try await call(
            method: "privacy.inspectLegacyContent",
            params: EmptyParams()
        )
        return try validatedLegacyPrivateContentInspection(inspection)
    }

    func previewLegacyPrivateContentCleanup() async throws -> LegacyPrivateContentCleanupPreview {
        let preview: LegacyPrivateContentCleanupPreview = try await call(
            method: "privacy.previewCleanupLegacyContent",
            params: EmptyParams()
        )
        _ = try validatedLegacyPrivateContentInspection(preview.inspection)
        if !preview.inspection.cleanupRequired {
            guard preview.action == nil,
                  preview.preconditions.isEmpty,
                  preview.previewToken == nil,
                  !preview.confirmationRequired else {
                throw ClientError.invalidOutput(
                    "A clean legacy-content inspection returned an unexpected cleanup action."
                )
            }
            return preview
        }
        _ = try validatedLegacyPrivateContentCleanupAction(preview)
        return preview
    }

    func cleanupLegacyPrivateContent(
        preview: LegacyPrivateContentCleanupPreview
    ) async throws -> LegacyPrivateContentCleanupResult {
        let action = try validatedLegacyPrivateContentCleanupAction(preview)
        guard let confirmation = preview.confirmation else {
            throw ClientError.invalidOutput(
                "Legacy private-content cleanup preview omitted its confirmation binding."
            )
        }
        let result: LegacyPrivateContentCleanupResult = try await call(
            method: "privacy.cleanupLegacyContent",
            params: LegacyPrivateContentCleanupParams(
                actionConfirmation: confirmation
            )
        )
        _ = try validatedLegacyPrivateContentInspection(result.inspection)
        do {
            _ = try result.readback.validated(for: action)
        } catch {
            throw ClientError.invalidOutput(error.localizedDescription)
        }
        guard result.state == "verified",
              result.effect == "legacy_private_content_removed_or_sanitized",
              result.cleanedSourceCount > 0,
              !result.retryAllowed,
              !result.inspection.cleanupRequired,
              result.readback.domains == ["private_content"],
              result.readback.targetIDs == ["legacy-ai-private-content"] else {
            throw ClientError.invalidOutput(
                "Legacy private-content cleanup did not return a verified clean inspection."
            )
        }
        return result
    }

    private func validatedLegacyPrivateContentInspection(
        _ inspection: LegacyPrivateContentInspection
    ) throws -> LegacyPrivateContentInspection {
        let allowedFiles = Set([
            "prompt-runs.json",
            "model-task-matches.json",
            "task-preflight-history.json",
        ])
        let cleanupSources = inspection.sources.filter(\.cleanupRequired)
        guard inspection.readOnly,
              !inspection.providerRequestSent,
              !inspection.rawContentReturned,
              !inspection.writePerformed,
              inspection.cleanupSourceCount == cleanupSources.count,
              inspection.existingSourceCount == inspection.sources.count,
              inspection.cleanupRequired == !cleanupSources.isEmpty,
              Set(inspection.sources.map(\.id)).count == inspection.sources.count,
              inspection.sources.allSatisfy({
                  !$0.id.isEmpty
                      && allowedFiles.contains($0.sourceFile)
                      && ["regular_file", "symbolic_link"].contains($0.itemType)
                      && ["sanitize_metadata", "delete_leaf"].contains($0.cleanupOperation)
                      && (!$0.malformed || $0.sourceFile == "prompt-runs.json")
                      && (!$0.generatedResidue || $0.cleanupOperation == "delete_leaf")
                      && ($0.cleanupRequired || $0.state == "current_metadata_only")
              }) else {
            throw ClientError.invalidOutput(
                "Legacy private-content inspection violated its content-free read-only contract."
            )
        }
        return inspection
    }

    private func validatedLegacyPrivateContentCleanupAction(
        _ preview: LegacyPrivateContentCleanupPreview
    ) throws -> ActionDescriptorWire {
        _ = try validatedLegacyPrivateContentInspection(preview.inspection)
        guard preview.inspection.cleanupRequired,
              preview.confirmationRequired,
              let action = preview.action,
              let previewToken = preview.previewToken,
              !previewToken.isEmpty,
              !action.id.isEmpty,
              action.kind == "privacy_cleanup",
              action.intent == "clean_legacy_private_content",
              action.target.kind == "app_data",
              action.target.id == "legacy-ai-private-content",
              action.target.agent == nil,
              action.target.scope == nil,
              action.projectID == nil,
              action.impacts == ["app_local_data"],
              action.previewMethod == "privacy.previewCleanupLegacyContent",
              action.applyMethod == "privacy.cleanupLegacyContent",
              !action.sourceRevision.isEmpty,
              action.confirmationRequired,
              action.network == "none",
              action.readback == ["private_content"],
              action.evidenceRefs == ["app-data:legacy-ai-private-content"] else {
            throw ClientError.invalidOutput(
                "Legacy private-content cleanup preview returned an invalid action boundary."
            )
        }

        let cleanupIDs = Set(preview.inspection.cleanupSources.map(\.id))
        let leafPreconditions = preview.preconditions.filter {
            $0.kind == "legacy_private_content"
        }
        let contextPreconditions = preview.preconditions.filter {
            $0.kind == "prompt_context"
                && $0.targetID == "provider-action-state"
        }
        guard leafPreconditions.count == cleanupIDs.count,
              Set(leafPreconditions.map(\.targetID)) == cleanupIDs,
              contextPreconditions.count == 1,
              preview.preconditions.count == leafPreconditions.count + 1,
              preview.preconditions.allSatisfy({
                  !$0.expectedRevision.trimmingCharacters(
                      in: .whitespacesAndNewlines
                  ).isEmpty
              }) else {
            throw ClientError.invalidOutput(
                "Legacy private-content cleanup preview did not bind every cleanup source."
            )
        }
        return action
    }
}
