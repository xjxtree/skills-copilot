import AppKit
import SwiftUI

struct FirstRunOnboardingView: View {
    @EnvironmentObject private var store: SkillStore
    @Environment(\.dismiss) private var dismiss
    @Binding var hasCompletedOnboarding: Bool

    var body: some View {
        VStack(alignment: .leading, spacing: 22) {
            HStack(alignment: .top, spacing: 16) {
                Image(systemName: "sparkles.rectangle.stack")
                    .font(.system(size: 36, weight: .semibold))
                    .foregroundStyle(Color.accentColor)
                    .frame(width: 56, height: 56)
                    .background(Color.accentColor.opacity(0.12), in: RoundedRectangle(cornerRadius: 14))
                    .accessibilityHidden(true)

                VStack(alignment: .leading, spacing: 6) {
                    Text(UIStrings.text("onboarding.title", "Welcome to Agent Copilot"))
                        .font(.title.bold())
                    Text(UIStrings.text(
                        "onboarding.subtitle",
                        "Inspect local agent sessions, skills, and configuration—then move directly into safe, previewed actions."
                    ))
                    .font(.callout)
                    .foregroundStyle(.secondary)
                    .fixedSize(horizontal: false, vertical: true)
                }
            }

            VStack(spacing: 10) {
                onboardingStep(
                    number: 1,
                    title: UIStrings.text("onboarding.project.title", "Choose your working project"),
                    message: UIStrings.text(
                        "onboarding.project.message",
                        "Project scope keeps session, skill, and configuration results relevant. Global agent roots remain available without a project."
                    ),
                    systemImage: "folder.badge.plus"
                )
                onboardingStep(
                    number: 2,
                    title: UIStrings.text("onboarding.refresh.title", "Refresh all local sources"),
                    message: UIStrings.text(
                        "onboarding.refresh.message",
                        "Refresh re-enumerates supported local roots and updates the current workspace after files are added, removed, or moved."
                    ),
                    systemImage: "arrow.clockwise"
                )
                onboardingStep(
                    number: 3,
                    title: UIStrings.text("onboarding.privacy.title", "Local and private by default"),
                    message: UIStrings.text(
                        "onboarding.privacy.message",
                        "Privacy Mode hides local paths. AI provider features stay off until you configure and explicitly confirm a request."
                    ),
                    systemImage: "hand.raised.fill"
                )
            }

            Spacer(minLength: 0)

            HStack(spacing: 10) {
                Button {
                    chooseProject()
                } label: {
                    Label(UIStrings.chooseProject, systemImage: "folder.badge.plus")
                }
                .buttonStyle(.bordered)
                .disabled(store.isRefreshBusy)
                .accessibilityHint(UIStrings.text(
                    "onboarding.project.action.hint",
                    "Choose a local project folder and scan its supported agent roots."
                ))

                Spacer()

                Button(UIStrings.text("onboarding.skip", "Explore Global Data")) {
                    completeOnboarding()
                }
                .buttonStyle(.borderless)

                Button {
                    completeOnboarding()
                } label: {
                    Label(
                        UIStrings.text("onboarding.start", "Get Started"),
                        systemImage: "arrow.right"
                    )
                }
                .buttonStyle(.borderedProminent)
                .keyboardShortcut(.defaultAction)
            }
        }
        .padding(28)
        .frame(width: 640, height: 500)
        .accessibilityElement(children: .contain)
        .accessibilityLabel(UIStrings.text("onboarding.title", "Welcome to Agent Copilot"))
    }

    private func onboardingStep(
        number: Int,
        title: String,
        message: String,
        systemImage: String
    ) -> some View {
        HStack(alignment: .top, spacing: 12) {
            ZStack {
                Circle()
                    .fill(Color.accentColor.opacity(0.12))
                Text(String(number))
                    .font(.callout.bold())
                    .foregroundStyle(Color.accentColor)
            }
            .frame(width: 30, height: 30)
            .accessibilityHidden(true)

            Image(systemName: systemImage)
                .font(.body.weight(.semibold))
                .foregroundStyle(.secondary)
                .frame(width: 24, height: 30)
                .accessibilityHidden(true)

            VStack(alignment: .leading, spacing: 3) {
                Text(title)
                    .font(.headline)
                Text(message)
                    .font(.callout)
                    .foregroundStyle(.secondary)
                    .fixedSize(horizontal: false, vertical: true)
            }
            .frame(maxWidth: .infinity, alignment: .leading)
        }
        .padding(12)
        .background(Color.agentCopilotPanelBackground, in: RoundedRectangle(cornerRadius: 10))
        .accessibilityElement(children: .combine)
        .accessibilityLabel("\(number). \(title)")
        .accessibilityValue(message)
    }

    private func chooseProject() {
        let panel = NSOpenPanel()
        panel.canChooseFiles = false
        panel.canChooseDirectories = true
        panel.allowsMultipleSelection = false
        panel.canCreateDirectories = false
        panel.prompt = UIStrings.chooseProject

        guard panel.runModal() == .OK, let url = panel.url else { return }
        Task {
            await store.setProject(
                rootPath: url.path,
                currentCWD: url.path,
                name: url.lastPathComponent
            )
            completeOnboarding()
        }
    }

    private func completeOnboarding() {
        hasCompletedOnboarding = true
        dismiss()
    }
}
