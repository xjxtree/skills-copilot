import SwiftUI

struct LegacyPrivateContentGlobalBanner: View {
    @EnvironmentObject private var store: SkillStore

    var body: some View {
        if shouldShow {
            HStack(spacing: 10) {
                Image(systemName: "exclamationmark.shield.fill")
                    .foregroundStyle(.orange)

                VStack(alignment: .leading, spacing: 2) {
                    Text(UIStrings.legacyPrivateContentGlobalTitle)
                        .font(.callout.bold())
                    Text(message)
                        .font(.caption)
                        .foregroundStyle(.secondary)
                        .lineLimit(2)
                }

                Spacer(minLength: 12)

                Button(UIStrings.legacyPrivateContentOpenSettings) {
                    SettingsNavigation.openProviderObservability()
                }
                .buttonStyle(.bordered)
            }
            .padding(.horizontal, 16)
            .padding(.vertical, 9)
            .background(.orange.opacity(0.10))
            .overlay(alignment: .bottom) {
                Divider()
            }
            .accessibilityElement(children: .contain)
        }
    }

    private var shouldShow: Bool {
        store.legacyPrivateContentInspection?.cleanupRequired == true
            || store.legacyPrivateContentCleanupError != nil
    }

    private var message: String {
        if let error = store.legacyPrivateContentCleanupError {
            return error
        }
        if let inspection = store.legacyPrivateContentInspection,
           inspection.cleanupRequired {
            return UIStrings.legacyPrivateContentGlobalSummary(
                inspection.cleanupSourceCount
            )
        }
        return UIStrings.legacyPrivateContentInspectionFailed
    }
}
