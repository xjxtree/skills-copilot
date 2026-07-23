import SwiftUI

struct LegacyPrivateContentCleanupCard: View {
    @EnvironmentObject private var store: SkillStore

    var body: some View {
        if shouldShow {
            VStack(alignment: .leading, spacing: 10) {
                titleRow

                if let inspection = store.legacyPrivateContentInspection,
                   inspection.cleanupRequired {
                    Text(UIStrings.legacyPrivateContentSummary(inspection.cleanupSourceCount))
                        .font(.caption)
                        .foregroundStyle(.secondary)

                    if let error = store.legacyPrivateContentCleanupError {
                        Text(error)
                            .font(.caption)
                            .foregroundStyle(.orange)
                    }

                    cleanupSourceList(inspection.cleanupSources)

                    if store.legacyPrivateContentCleanupPreview != nil {
                        confirmationControls
                    } else {
                        previewControls
                    }
                } else if let error = store.legacyPrivateContentCleanupError {
                    Text(error)
                        .font(.caption)
                        .foregroundStyle(.secondary)

                    Button(UIStrings.retry) {
                        Task { await store.inspectLegacyPrivateContent() }
                    }
                    .buttonStyle(.bordered)
                    .disabled(store.isInspectingLegacyPrivateContent)
                } else {
                    HStack(spacing: 8) {
                        ProgressView()
                            .controlSize(.small)
                        Text(UIStrings.legacyPrivateContentInspecting)
                            .font(.caption)
                            .foregroundStyle(.secondary)
                    }
                }
            }
            .padding(12)
            .frame(maxWidth: .infinity, alignment: .leading)
            .nativePanelSurface()
        }
    }

    private var shouldShow: Bool {
        store.isInspectingLegacyPrivateContent
            || store.legacyPrivateContentInspection?.cleanupRequired == true
            || store.legacyPrivateContentCleanupError != nil
    }

    private var titleRow: some View {
        HStack(alignment: .firstTextBaseline, spacing: 8) {
            Label(
                UIStrings.legacyPrivateContentTitle,
                systemImage: "exclamationmark.shield"
            )
            .font(.callout.bold())
            .foregroundStyle(.orange)

            Spacer()

            Text(UIStrings.legacyPrivateContentLocalOnly)
                .font(.caption2.bold())
                .foregroundStyle(.secondary)
        }
    }

    @ViewBuilder
    private func cleanupSourceList(_ sources: [LegacyPrivateContentSource]) -> some View {
        VStack(alignment: .leading, spacing: 5) {
            ForEach(sources) { source in
                HStack(spacing: 8) {
                    Image(systemName: source.cleanupOperation == "sanitize_metadata"
                        ? "doc.badge.ellipsis"
                        : "trash")
                        .frame(width: 14)
                        .foregroundStyle(.secondary)
                    Text(source.sourceFile)
                        .font(.caption.monospaced())
                        .lineLimit(1)
                    Spacer()
                    Text(cleanupOperationLabel(source.cleanupOperation))
                        .font(.caption2)
                        .foregroundStyle(.secondary)
                }
            }
        }
    }

    private var previewControls: some View {
        HStack {
            Text(UIStrings.legacyPrivateContentPreviewBoundary)
                .font(.caption2)
                .foregroundStyle(.secondary)
            Spacer()
            Button(UIStrings.legacyPrivateContentReview) {
                Task { await store.previewLegacyPrivateContentCleanup() }
            }
            .buttonStyle(.bordered)
            .disabled(
                store.isPreviewingLegacyPrivateContentCleanup
                    || store.isCleaningLegacyPrivateContent
            )
        }
    }

    private var confirmationControls: some View {
        VStack(alignment: .leading, spacing: 8) {
            Divider()
            Text(UIStrings.legacyPrivateContentConfirmation)
                .font(.caption)
                .foregroundStyle(.secondary)
            HStack {
                Spacer()
                Button(UIStrings.cancel) {
                    store.cancelLegacyPrivateContentCleanupPreview()
                }
                .buttonStyle(.bordered)
                .disabled(store.isCleaningLegacyPrivateContent)

                Button(UIStrings.legacyPrivateContentCleanNow, role: .destructive) {
                    Task { await store.confirmLegacyPrivateContentCleanup() }
                }
                .buttonStyle(.borderedProminent)
                .disabled(store.isCleaningLegacyPrivateContent)
            }
        }
    }

    private func cleanupOperationLabel(_ operation: String) -> String {
        operation == "sanitize_metadata"
            ? UIStrings.legacyPrivateContentSanitize
            : UIStrings.legacyPrivateContentDelete
    }
}
