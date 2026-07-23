import Foundation
@testable import SkillsCopilot

struct UIOptimizationModelTests {
    func run() throws {
        try sidebarSelectionUsesNativeMutedTreatment()
        try listPagesUseUnifiedToolbarAndWhiteCardRows()
        try secondarySidebarListsUseGlobalTreatment()
        try skillListDensityMatchesOptimizationPlan()
        try emptyAgentSkillListsExplainAgentContext()
        try detailHeaderUsesCompactCopyableMetadata()
        try detailFeedbackUsesInlineToast()
        try configEditorUsesAutosaveCodeCardPresentation()
        try settingsWindowUsesSidebarAndCloseOnlyControls()
        try settingsProviderObservabilityUsesBoundedScopedRendering()
        try modalWorkflowsUseSharedSheetChromeAndColumns()
        try settingsPreflightAndManagerSurfacesHaveStablePresentation()
        try skillManagerPreviewMetadataIsCompactAndActionSafe()
    }

    private func sidebarSelectionUsesNativeMutedTreatment() throws {
        try expectEqual(
            UIOptimizationPresentation.sidebarSelection.usesSaturatedAccentBackground,
            false,
            "Sidebar selection should avoid saturated accent backgrounds."
        )
        try expectEqual(
            UIOptimizationPresentation.sidebarSelection.usesWhiteSelectedText,
            false,
            "Sidebar selected rows should keep readable primary text instead of forcing white text."
        )
        try expectEqual(
            UIOptimizationPresentation.sidebarSelection.accentLineWidth,
            3,
            "Sidebar selected rows should use a 3pt brand accent line."
        )
    }

    private func listPagesUseUnifiedToolbarAndWhiteCardRows() throws {
        try expectEqual(
            UIOptimizationPresentation.unifiedToolbar.spansEntireWindow,
            true,
            "List pages should use one window-level toolbar instead of stacked gray bars."
        )
        try expectEqual(
            UIOptimizationPresentation.unifiedToolbar.searchPlacement,
            .globalTrailing,
            "The global search field should live in the trailing window toolbar."
        )
        try expectEqual(
            UIOptimizationPresentation.unifiedToolbar.collapsesAtScrollEdge,
            true,
            "The toolbar should allow the system Liquid Glass scroll-edge treatment to collapse or compact it."
        )
        try expectEqual(
            UIOptimizationPresentation.unifiedToolbar.settingsActionUsesSystemSettingsLink,
            true,
            "The toolbar settings button should use the system Settings scene entry point."
        )
        try expectEqual(
            UIOptimizationPresentation.listPage.filterStyle,
            .capsule,
            "Skill and session filters should render as capsule controls under the content title."
        )
        try expectEqual(
            UIOptimizationPresentation.listPage.searchScope,
            .localList,
            "The search field below the list title should be visually and semantically scoped to the visible list."
        )
        try expectEqual(
            UIOptimizationPresentation.listPage.rowStyle,
            .whiteCard,
            "Skill and session rows should use spacious white card rows instead of custom translucent chrome."
        )
        try expectEqual(
            UIOptimizationPresentation.listPage.minimumCardRowHeight,
            58,
            "Card rows need enough vertical space for icon, title, status, and secondary metadata."
        )
        try expectEqual(
            UIOptimizationPresentation.listPage.cardRowSpacing,
            8,
            "Cards should have visible breathing room between rows."
        )
        try expectEqual(
            UIOptimizationPresentation.sidebarShell.width,
            260,
            "The primary sidebar should use a stable full-height width for repository and navigation context."
        )
    }

    private func secondarySidebarListsUseGlobalTreatment() throws {
        try expectEqual(
            UIOptimizationPresentation.sessionList.usesSingleLineFilterToolbar,
            true,
            "Session sidebar filters, search, and refresh should fit the unified toolbar model."
        )
        try expectEqual(
            UIOptimizationPresentation.sessionList.refreshUsesIconOnly,
            true,
            "Session refresh should use an icon-only toolbar action."
        )
        try expectEqual(
            UIOptimizationPresentation.configList.usesSingleLineFilterToolbar,
            true,
            "Config sidebar scope, search, and refresh should fit the unified toolbar model."
        )
        try expectEqual(
            UIOptimizationPresentation.configList.minimumSearchWidth,
            220,
            "Config search should keep the same minimum width as the optimized skill/session sidebars."
        )
        try expectEqual(
            UIOptimizationPresentation.configList.compactRowMaxHeight,
            44,
            "Config sidebar rows should stay in the compact global row-height range."
        )
    }

    private func skillListDensityMatchesOptimizationPlan() throws {
        try expectEqual(
            UIOptimizationPresentation.skillList.minimumSecondaryColumnWidth,
            360,
            "The middle skill list column should not collapse below the optimized minimum width."
        )
        try expectEqual(
            UIOptimizationPresentation.skillList.minimumSearchWidth,
            220,
            "Skill search should retain the proposed minimum width."
        )
        try expectEqual(
            UIOptimizationPresentation.skillList.compactRowMinHeight,
            36,
            "Compact skill rows should start at the proposed 36pt height."
        )
        try expectEqual(
            UIOptimizationPresentation.skillList.compactRowMaxHeight,
            40,
            "Compact skill rows should cap at the proposed 40pt height."
        )
        try expectEqual(
            UIOptimizationPresentation.skillList.usesSingleLineFilterToolbar,
            true,
            "Skill filters should be modeled as a single-line toolbar at regular widths."
        )
    }

    private func emptyAgentSkillListsExplainAgentContext() throws {
        let agentEmptyMessage = UIOptimizationPresentation.skillList.emptyFilteredMessage(
            agentFilter: .claudeCode,
            hasActiveProjectContext: false,
            hasActiveSearchOrFilter: false
        )

        try expectFalse(
            agentEmptyMessage == UIStrings.noSkillsMatchSearch,
            "An empty selected agent should not look like a failed search when no search or filters are active."
        )
        try expectEqual(
            agentEmptyMessage.contains(UIStrings.claudeCode),
            true,
            "The empty message should name the selected agent so users know the data is hidden by agent context."
        )

        try expectEqual(
            UIOptimizationPresentation.skillList.emptyFilteredMessage(
                agentFilter: .claudeCode,
                hasActiveProjectContext: false,
                hasActiveSearchOrFilter: true
            ),
            UIStrings.noSkillsMatchSearch,
            "Active search or filter criteria should still use the filtered-empty copy."
        )

        let openClawEmptyMessage = UIOptimizationPresentation.skillList.emptyFilteredMessage(
            agentFilter: .openclaw,
            hasActiveProjectContext: true,
            hasActiveSearchOrFilter: false
        )
        try expectFalse(
            openClawEmptyMessage.contains("<workspace>"),
            "OpenClaw empty copy should not expose raw placeholder tokens to users."
        )
    }

    private func detailHeaderUsesCompactCopyableMetadata() throws {
        try expectEqual(
            UIOptimizationPresentation.detailHeader.height,
            48,
            "The detail header should use the proposed compact 44-48pt range."
        )
        try expectEqual(
            UIOptimizationPresentation.detailHeader.definitionUsesMonospacedFont,
            true,
            "Definition hashes should use monospaced presentation."
        )
        try expectEqual(
            UIOptimizationPresentation.detailHeader.primaryToggleLivesInMenu,
            true,
            "Risky enable/disable actions should move out of the prominent blue primary button slot."
        )
    }

    private func detailFeedbackUsesInlineToast() throws {
        try expectEqual(
            UIOptimizationPresentation.detailFeedback.usesOverlayToast,
            false,
            "Detail feedback should render inline so it does not cover detail content."
        )
        try expectEqual(
            UIOptimizationPresentation.detailFeedback.maximumWidth,
            420,
            "Detail feedback toasts should keep a compact readable width."
        )
    }

    private func configEditorUsesAutosaveCodeCardPresentation() throws {
        try expectEqual(
            UIOptimizationPresentation.configEditor.usesSingleCodeCard,
            true,
            "Config editing should use one focused code-card surface instead of separate header, editor, and footer bars."
        )
        try expectEqual(
            UIOptimizationPresentation.configEditor.showsLineNumbers,
            true,
            "Config JSON viewers and editors should expose line numbers."
        )
        try expectEqual(
            UIOptimizationPresentation.configEditor.usesCompactToolbarActions,
            true,
            "Reload, format, and reveal/edit controls should live in a compact card toolbar."
        )
        try expectEqual(
            UIOptimizationPresentation.configEditor.primarySaveButtonVisible,
            false,
            "Config editing should not depend on a persistent large Save button under the editor."
        )
        try expectEqual(
            UIOptimizationPresentation.configEditor.autosaveEnabled,
            true,
            "Editable JSON config changes should autosave through the verified service save flow."
        )
    }

    private func settingsWindowUsesSidebarAndCloseOnlyControls() throws {
        try expectEqual(
            UIOptimizationPresentation.settings.navigationStyle,
            .sidebar,
            "Settings should use a macOS sidebar list instead of top tab pages."
        )
        try expectEqual(
            UIOptimizationPresentation.settings.usesDedicatedSettingsScene,
            true,
            "Settings should remain a separate modeless Settings scene."
        )
        try expectEqual(
            UIOptimizationPresentation.settings.windowControlPolicy,
            .closeOnly,
            "Settings windows should disable minimize and zoom controls."
        )
        try expectEqual(
            UIOptimizationPresentation.settings.primarySaveButtonsVisible,
            false,
            "Settings changes should not depend on large persistent Save or Done buttons."
        )
        try expectEqual(
            UIOptimizationPresentation.settings.sidebarWidth,
            190,
            "Settings sidebar should reserve stable width for appearance, provider, monitoring, and service categories."
        )
    }

    private func settingsProviderObservabilityUsesBoundedScopedRendering() throws {
        try expectEqual(
            UIOptimizationPresentation.settings.providerObservabilityAutoLoadsAtStartup,
            true,
            "Provider observability should preload during app startup so Settings can display it without a local build action."
        )
        try expectEqual(
            UIOptimizationPresentation.settings.providerObservabilityHasLocalBuildAction,
            false,
            "Provider observability settings should not expose a Generate/Build action."
        )
        try expectEqual(
            UIOptimizationPresentation.settings.providerObservabilityHidesRawLogList,
            true,
            "Provider observability settings should hide raw log rows and present a lightweight dashboard instead."
        )
        try expectEqual(
            UIOptimizationPresentation.settings.providerObservabilitySummaryMetricCount,
            5,
            "Provider observability settings should keep the first row to five concise summary metrics."
        )
        try expectEqual(
            UIOptimizationPresentation.settings.providerObservabilityChartRowLimit,
            5,
            "Provider observability charts should cap row counts so scrolling remains responsive."
        )
        try expectEqual(
            UIOptimizationPresentation.settings.providerObservabilityUsesScopedScroll,
            true,
            "Provider observability settings should own a scoped scroll surface instead of relying on the whole settings pane to relayout heavy content."
        )
        try expectEqual(
            UIOptimizationPresentation.settings.providerObservabilityDisablesSelectionOverlay,
            true,
            "Provider observability settings should disable selectable text overlays that can invalidate AppKit text layout while scrolling."
        )
        try expectEqual(
            UIOptimizationPresentation.settings.providerObservabilityAvoidsAdaptiveGrids,
            true,
            "Provider observability settings should avoid adaptive Grid/LazyVGrid layouts that can loop with AppKit-backed text while scrolling."
        )
    }

    private func modalWorkflowsUseSharedSheetChromeAndColumns() throws {
        try expectEqual(
            UIOptimizationPresentation.workflowSheet.titlebarStyle,
            .liquidGlass,
            "Modal workflows should share a compact Liquid Glass titlebar instead of bespoke red or gray header bands."
        )
        try expectEqual(
            UIOptimizationPresentation.workflowSheet.closeActionPlacement,
            .trailingTitlebar,
            "Workflow sheets should expose Done/Close in the trailing titlebar."
        )
        try expectEqual(
            UIOptimizationPresentation.workflowSheet.feedbackStyle,
            .inlineTintedBanner,
            "Workflow errors and warnings should use scoped lightweight banners."
        )
        try expectEqual(
            UIOptimizationPresentation.workflowSheet.columnLayout,
            .twoColumn,
            "Workflow sheets should keep input/options visually separate from history, lists, and previews."
        )
        try expectEqual(
            UIOptimizationPresentation.taskPreflight.sheetContentLayout,
            .editorWithHistory,
            "Task Preflight should render the editor on the left and history on the right."
        )
        try expectEqual(
            UIOptimizationPresentation.skillManager.sheetContentLayout,
            .controlsWithResults,
            "Skill Manager should render controls on the left and search/installed/local results on the right."
        )
    }

    private func settingsPreflightAndManagerSurfacesHaveStablePresentation() throws {
        try expectEqual(
            UIOptimizationPresentation.settings.minimumWidth,
            760,
            "Settings should keep the existing stable minimum window width."
        )
        try expectEqual(
            UIOptimizationPresentation.settings.usesUnifiedSectionHeaders,
            true,
            "Settings pages should share compact header and section presentation."
        )
        try expectEqual(
            UIOptimizationPresentation.taskPreflight.sheetMinimumWidth,
            950,
            "Task Preflight should retain the optimized two-column sheet width."
        )
        try expectEqual(
            UIOptimizationPresentation.taskPreflight.historyColumnWidth,
            270,
            "Task Preflight history should remain a stable right-side column."
        )
        try expectEqual(
            UIOptimizationPresentation.taskPreflight.fixedAgentChipWidth,
            0,
            "Task Preflight agent chips should use adaptive width instead of the old 96pt fixed width."
        )
        try expectEqual(
            UIOptimizationPresentation.taskPreflight.showsProviderUnavailableGate,
            true,
            "Task Preflight should explicitly gate provider-unavailable states before build actions."
        )
        try expectEqual(
            UIOptimizationPresentation.skillManager.usesSegmentedWorkflows,
            true,
            "Skill Manager should separate search/install, installed/update, and local library workflows."
        )
        try expectEqual(
            UIOptimizationPresentation.skillManager.targetsSummaryIsPinned,
            true,
            "Skill Manager targets and safety controls should stay above workflow content."
        )
        try expectEqual(
            UIOptimizationPresentation.skillManager.toolUnavailableDisablesExternalMutations,
            true,
            "External manager unavailable states should disable search/install/remove/update controls."
        )
        try expectEqual(
            UIOptimizationPresentation.skillManager.usesSurfaceLocalFeedback,
            true,
            "Skill Manager errors should stay scoped to the sheet instead of leaking into detail empty states."
        )
    }

    private func skillManagerPreviewMetadataIsCompactAndActionSafe() throws {
        let preview = SkillManagerCommandPreview(
            toolId: "npx-skills",
            operation: "install",
            command: ["/usr/local/bin/npx", "skills", "add", "demo/agent-skills"],
            cwd: "/tmp/project",
            env: [],
            requiresConfirmation: true,
            confirmed: false,
            networkRequired: true,
            networkAllowed: false,
            willRun: false,
            previewToken: "skill-manager:test",
            summary: "Install preview",
            risks: ["External manager writes selected targets."],
            source: "demo/agent-skills",
            skills: []
        )

        let rows = preview.compactMetadataRows
        try expectEqual(
            rows.map { $0.label },
            ["CWD", "Confirmed", "Network"],
            "Preview metadata should use a compact key-value row order."
        )
        try expectEqual(
            rows.filter { $0.isCopyable }.map { $0.label },
            ["CWD"],
            "Opaque authorization capabilities must never expose copy affordances."
        )
        try expectEqual(
            preview.requiresExplicitApplyConfirmation,
            true,
            "Write-capable manager operations should remain explicit-confirm before apply."
        )
    }
}
