import Testing
@testable import SkillsCopilot

@Suite("MainWindowModelTests")
struct MainWindowModelTests {
    @Test("MainWindowModelTests")
    func run() throws {
        try mainWindowConfigurationIsStable()
        try taskCockpitAccessibilityIdentifiersAreStable()
        try configuredMainWindowWinsReopenScoring()
        try onboardingStorageKeyIsVersioned()
    }

    private func mainWindowConfigurationIsStable() throws {
        try expectEqual(AppAccessibilityID.mainWindow, "skills-copilot.main-window", "Main window accessibility identifier should stay stable for Computer Use.")
        try expectEqual(AppAccessibilityID.mainContent, "skills-copilot.main-content", "Main content accessibility identifier should stay stable.")
        try expectEqual(MainWindowModel.windowIdentifierRawValue, AppAccessibilityID.mainWindow, "Window identifier should match the AX identifier.")
        try expectEqual(MainWindowModel.autosaveName, "SkillsCopilot.MainWindow", "Main window autosave name should remain stable.")
        try expectEqual(MainWindowModel.minimumWidth, 1024, "Main window minimum width should fit supported compact laptop layouts.")
        try expectEqual(MainWindowModel.minimumHeight, 600, "Main window minimum height should match the launch smoke expectation.")
        try expectEqual(
            MainWindowModel.usesCompactLayout(width: 1024),
            true,
            "The minimum supported width should use the compact two-column layout."
        )
        try expectEqual(
            MainWindowModel.usesCompactLayout(width: 1349),
            false,
            "Wide windows should retain the three-column layout."
        )
        try expectEqual(
            MainWindowModel.maximumReadableDetailWidth <= 680,
            true,
            "Skill detail prose should stay within the readable line-width budget."
        )
    }

    private func taskCockpitAccessibilityIdentifiersAreStable() throws {
        try expectEqual(AppAccessibilityID.taskCockpitPanel, "skills-copilot.task-cockpit.panel", "Task Cockpit panel AX identifier should stay stable.")
        try expectEqual(AppAccessibilityID.taskCockpitInput, "skills-copilot.task-cockpit.input", "Task Cockpit input AX identifier should stay stable.")
        try expectEqual(AppAccessibilityID.taskCockpitBuildButton, "skills-copilot.task-cockpit.build", "Task Cockpit build AX identifier should stay stable.")
        try expectEqual(AppAccessibilityID.taskCockpitStatus, "skills-copilot.task-cockpit.status", "Task Cockpit status AX identifier should stay stable.")
        try expectEqual(AppAccessibilityID.taskCockpitCancelButton, "skills-copilot.task-cockpit.cancel", "Task Cockpit cancel AX identifier should stay stable.")
        try expectEqual(AppAccessibilityID.taskCockpitRetryButton, "skills-copilot.task-cockpit.retry", "Task Cockpit retry AX identifier should stay stable.")
        try expectEqual(AppAccessibilityID.taskCockpitStageProgress, "skills-copilot.task-cockpit.stage-progress", "Task Cockpit staged progress AX identifier should stay stable.")
        try expectEqual(AppAccessibilityID.taskCockpitResult, "skills-copilot.task-cockpit.result", "Task Cockpit result AX identifier should stay stable.")
    }

    private func configuredMainWindowWinsReopenScoring() throws {
        let configured = MainWindowModel.mainWindowScore(
            identifierRawValue: MainWindowModel.windowIdentifierRawValue,
            title: UIStrings.appWindowTitle,
            canBecomeMain: true
        )
        let titledOnly = MainWindowModel.mainWindowScore(
            identifierRawValue: nil,
            title: UIStrings.appWindowTitle,
            canBecomeMain: true
        )
        let other = MainWindowModel.mainWindowScore(
            identifierRawValue: "other-window",
            title: "Other",
            canBecomeMain: true
        )

        try expectEqual(configured > titledOnly, true, "Configured main window should win over a title-only window.")
        try expectEqual(titledOnly > other, true, "Titled main window should win over unrelated app windows.")
    }

    private func onboardingStorageKeyIsVersioned() throws {
        try expectEqual(
            FirstRunOnboardingModel.completionStorageKey,
            "agentCopilot.onboarding.completed.v1",
            "Onboarding completion should use an explicit versioned app-local key."
        )
    }
}
