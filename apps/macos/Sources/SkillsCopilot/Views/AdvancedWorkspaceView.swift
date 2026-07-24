import SwiftUI

struct AdvancedWorkspaceView: View {
    @EnvironmentObject private var store: SkillStore
    let columnVisibility: NavigationSplitViewVisibility

    var body: some View {
        HSplitView {
            SecondarySidebarView(columnVisibility: columnVisibility)
                .frame(
                    minWidth: CGFloat(
                        UIOptimizationPresentation.skillList.minimumSecondaryColumnWidth
                    ),
                    idealWidth: CGFloat(
                        UIOptimizationPresentation.skillList.idealSecondaryColumnWidth
                    ),
                    maxWidth: CGFloat(
                        UIOptimizationPresentation.skillList.maximumSecondaryColumnWidth
                    ),
                    maxHeight: .infinity
                )

            AdvancedConfigurationDetailView()
                .frame(maxWidth: .infinity, maxHeight: .infinity)
        }
        .task {
            store.openAdvancedConfiguration()
        }
        .accessibilityIdentifier("workspace.advanced")
    }
}

private struct AdvancedConfigurationDetailView: View {
    @EnvironmentObject private var store: SkillStore

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 18) {
                AdvancedConfigurationHeader()

                if let errorMessage = store.errorMessage {
                    ErrorBanner(message: errorMessage)
                }

                if store.selectedSidebarSelection?.isConfig == true {
                    AgentConfigDetailPanel()
                } else {
                    EmptyState(
                        title: UIStrings.text(
                            "advanced.configuration.empty.title",
                            "Select configuration evidence"
                        ),
                        systemImage: "slider.horizontal.3",
                        message: UIStrings.text(
                            "advanced.configuration.empty.message",
                            "Choose a current document or a recovery snapshot. Sensitive content and physical paths remain redacted until you explicitly reveal them."
                        )
                    )
                }
            }
            .padding(.top, 16)
            .padding(.horizontal, 28)
            .padding(.bottom, 28)
            .frame(maxWidth: .infinity, alignment: .leading)
        }
        .navigationTitle(UIStrings.appWindowTitle)
    }
}

private struct AdvancedConfigurationHeader: View {
    @EnvironmentObject private var store: SkillStore

    var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            HStack(alignment: .firstTextBaseline, spacing: 10) {
                Label(
                    UIStrings.text(
                        "advanced.configuration.title",
                        "Configuration & Recovery"
                    ),
                    systemImage: "wrench.and.screwdriver"
                )
                .font(.title2.bold())

                Spacer()

                Label(
                    UIStrings.text("settings.advanced", "Advanced"),
                    systemImage: "lock.shield"
                )
                .font(.caption.bold())
                .foregroundStyle(.secondary)
            }

            Text(
                UIStrings.text(
                    "advanced.configuration.boundary",
                    "Expert inspection for the selected Agent. Reads are redacted by default; save and rollback require a fresh typed preview, explicit confirmation, and verified read-back."
                )
            )
            .font(.callout)
            .foregroundStyle(.secondary)
            .fixedSize(horizontal: false, vertical: true)

            HStack(spacing: 8) {
                Label(store.agentFilter.title, systemImage: "cpu")
                if let projectName = store.activeProjectContext?.name {
                    Label(projectName, systemImage: "folder")
                }
            }
            .font(.caption)
            .foregroundStyle(.secondary)
        }
        .padding(14)
        .frame(maxWidth: .infinity, alignment: .leading)
        .nativePanelSurface()
    }
}
