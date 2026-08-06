import CoreGraphics
import Testing
@testable import SkillsCopilot

@Suite("MainWindowModelTests")
struct MainWindowModelTests {
    @Test("MainWindowModelTests")
    func run() throws {
        try mainWindowConfigurationIsStable()
        try windowChromeTracksTheResponsiveLayout()
        try compactWorkspaceStateIsModeIndependent()
        try compactRevealControlFollowsWorkspaceHover()
        try compactDetailKeepsReadableOverlayAndSelectionContext()
        try taskCockpitAccessibilityIdentifiersAreStable()
        try onboardingStorageKeyIsVersioned()
    }

    private func mainWindowConfigurationIsStable() throws {
        try expectEqual(AppAccessibilityID.mainWindow, "skills-copilot.main-window", "Main window accessibility identifier should stay stable.")
        try expectEqual(AppAccessibilityID.mainContent, "skills-copilot.main-content", "Main content accessibility identifier should stay stable.")
        try expectEqual(MainWindowModel.windowIdentifierRawValue, AppAccessibilityID.mainWindow, "Window identifier should match the AX identifier.")
        try expectEqual(MainWindowModel.mainSceneIdentifier, "main", "The single main Window scene should keep a stable identifier.")
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
            MainWindowModel.maximumReadableDetailWidth,
            800,
            "Wide detail surfaces should use the available workspace without making prose unbounded."
        )
        try expectEqual(
            MainWindowModel.maximumReadableDetailWidth <= 840,
            true,
            "Skill detail prose should stay within the readable line-width budget."
        )
        try expectEqual(
            MainWindowModel.minimumCompactDetailWidth,
            560,
            "Compact detail needs enough width for horizontal tabs and long-form content."
        )
        try expectEqual(
            MainWindowModel.minimumCompactVisibleListContextWidth,
            180,
            "A dismissible compact detail overlay should retain a narrow strip of list context."
        )
    }

    private func windowChromeTracksTheResponsiveLayout() throws {
        try expectEqual(
            MainWindowModel.windowChromeLayoutMode(width: CGFloat(MainWindowModel.minimumWidth)),
            .compact,
            "The minimum supported width should use compact titlebar controls."
        )
        try expectEqual(
            MainWindowModel.windowChromeLayoutMode(width: CGFloat(MainWindowModel.compactLayoutBreakpoint)),
            .regular,
            "The regular three-column breakpoint should restore the full titlebar controls."
        )
    }

    private func compactWorkspaceStateIsModeIndependent() throws {
        try expectEqual(
            MainWindowModel.compactWorkspaceLayer(selection: nil, showsDetail: true),
            .listOnly,
            "A compact workspace without a selected row should show the full secondary list."
        )

        let selections: [SidebarSelection] = [
            .skill("skill-alpha"),
            .session("session-alpha"),
            .configOverview,
        ]
        for selection in selections {
            try expectEqual(
                MainWindowModel.compactWorkspaceLayer(selection: selection, showsDetail: true),
                .detailOverlay,
                "Every selected compact workspace mode should use the same detail overlay."
            )
            try expectEqual(
                MainWindowModel.compactWorkspaceLayer(selection: selection, showsDetail: false),
                .detailRevealControl,
                "Every dismissed compact workspace detail should expose the same reveal control."
            )
        }
    }

    private func compactDetailKeepsReadableOverlayAndSelectionContext() throws {
        let compactSidebarWidth = CGFloat(UIOptimizationPresentation.sidebarShell.compactWidth)
        let testedWindowWidths = [
            CGFloat(MainWindowModel.minimumWidth),
            CGFloat(MainWindowModel.compactLayoutBreakpoint - 1),
        ]

        for windowWidth in testedWindowWidths {
            let availableWorkspaceWidth = windowWidth - compactSidebarWidth
            let detailWidth = MainWindowModel.compactDetailWidth(
                availableWidth: availableWorkspaceWidth
            )
            let visibleSecondaryWidth = MainWindowModel.compactVisibleSecondaryWidth(
                availableWidth: availableWorkspaceWidth
            )
            let secondaryContentWidth = MainWindowModel.compactSecondaryContentWidth(
                availableWidth: availableWorkspaceWidth
            )

            try expectEqual(
                secondaryContentWidth,
                availableWorkspaceWidth,
                "Compact detail should overlay the secondary list instead of compressing its content width."
            )
            try expectEqual(
                visibleSecondaryWidth >= MainWindowModel.minimumCompactVisibleListContextWidth,
                true,
                "Compact detail should retain enough uncovered list context to make dismissal understandable."
            )
            try expectEqual(
                detailWidth >= MainWindowModel.minimumCompactDetailWidth,
                true,
                "Compact detail should retain a readable minimum width for tabs and prose."
            )
            try expectEqual(
                detailWidth <= CGFloat(MainWindowModel.maximumReadableDetailWidth),
                true,
                "Compact detail should remain within the maximum readable width."
            )
            try expectEqual(
                detailWidth <= availableWorkspaceWidth,
                true,
                "Compact detail should never exceed the workspace bounds."
            )
        }
    }

    private func compactRevealControlFollowsWorkspaceHover() throws {
        try expectEqual(
            MainWindowModel.showsCompactDetailRevealControl(
                layer: .detailRevealControl,
                isWorkspaceHovered: false,
                isApplicationActive: true
            ),
            false,
            "The compact detail reveal control should not remain visible after the pointer leaves the workspace."
        )
        try expectEqual(
            MainWindowModel.showsCompactDetailRevealControl(
                layer: .detailRevealControl,
                isWorkspaceHovered: true,
                isApplicationActive: true
            ),
            true,
            "Hovering the compact workspace should reveal the detail control after detail was dismissed."
        )
        try expectEqual(
            MainWindowModel.showsCompactDetailRevealControl(
                layer: .detailOverlay,
                isWorkspaceHovered: true,
                isApplicationActive: true
            ),
            false,
            "The reveal control should stay hidden while detail is already visible."
        )
        try expectEqual(
            MainWindowModel.showsCompactDetailRevealControl(
                layer: .listOnly,
                isWorkspaceHovered: true,
                isApplicationActive: true
            ),
            false,
            "The reveal control should stay hidden when no selected detail exists."
        )
        try expectEqual(
            MainWindowModel.showsCompactDetailRevealControl(
                layer: .detailRevealControl,
                isWorkspaceHovered: true,
                isApplicationActive: false
            ),
            false,
            "The reveal control should hide immediately when Agent Copilot is not active."
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

    private func onboardingStorageKeyIsVersioned() throws {
        try expectEqual(
            FirstRunOnboardingModel.completionStorageKey,
            "agentCopilot.onboarding.completed.v1",
            "Onboarding completion should use an explicit versioned app-local key."
        )
    }
}
