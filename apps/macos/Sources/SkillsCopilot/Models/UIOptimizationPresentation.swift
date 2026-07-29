import Foundation

enum UIOptimizationPresentation {
    static let unifiedToolbar = UnifiedToolbarPresentation()
    static let listPage = ListPagePresentation()
    static let sidebarShell = SidebarShellPresentation()
    static let sidebarSelection = SidebarSelectionPresentation()
    static let surfaceCornerRadius = sidebarSelection.rowCornerRadius
    static let sessionList = SidebarSecondaryListPresentation()
    static let configList = SidebarSecondaryListPresentation()
    static let skillList = SkillListPresentation()
    static let detailHeader = DetailHeaderPresentation()
    static let detailFeedback = DetailFeedbackPresentation()
    static let configEditor = ConfigEditorPresentation()
    static let settings = SettingsPresentation()
    static let workflowSheet = WorkflowSheetPresentation()
    static let taskPreflight = TaskPreflightPresentation()
    static let skillManager = SkillManagerPresentation()
}

enum UnifiedToolbarSearchPlacement: Equatable {
    case globalTrailing
}

enum ListPageFilterStyle: Equatable {
    case capsule
}

enum ListPageSearchScope: Equatable {
    case localList
}

enum ListPageRowStyle: Equatable {
    case whiteCard
}

enum SettingsNavigationStyle: Equatable {
    case sidebar
}

enum SettingsWindowControlPolicy: Equatable {
    case closeOnly
}

enum WorkflowSheetTitlebarStyle: Equatable {
    case liquidGlass
}

enum WorkflowSheetCloseActionPlacement: Equatable {
    case trailingTitlebar
}

enum WorkflowSheetFeedbackStyle: Equatable {
    case inlineTintedBanner
}

enum WorkflowSheetColumnLayout: Equatable {
    case twoColumn
}

enum TaskPreflightSheetContentLayout: Equatable {
    case editorWithHistory
}

enum SkillManagerSheetContentLayout: Equatable {
    case controlsWithResults
}

struct UnifiedToolbarPresentation: Equatable {
    let spansEntireWindow = true
    let searchPlacement = UnifiedToolbarSearchPlacement.globalTrailing
    let collapsesAtScrollEdge = true
    let settingsActionUsesSystemSettingsLink = true
    let refreshControlWidth = 86
    let refreshHorizontalPadding = 6
    let refreshStatusSlotWidth = 12
    let reservesRefreshStatusSlot = true
    let minimumGlobalSearchWidth = 180
    let idealGlobalSearchWidth = 220
}

struct ListPagePresentation: Equatable {
    let filterStyle = ListPageFilterStyle.capsule
    let searchScope = ListPageSearchScope.localList
    let rowStyle = ListPageRowStyle.whiteCard
    let minimumCardRowHeight = 58
    let cardRowSpacing = 8
    let cardCornerRadius = 8
    let cardHorizontalInset = 12
    let localSearchCornerRadius = 10
}

struct SidebarShellPresentation: Equatable {
    let width = 260
    let compactWidth = 260
    let navigationCardHorizontalInset = -8
    let footerTopSpacing = 10
}

struct SidebarSelectionPresentation: Equatable {
    let usesSaturatedAccentBackground = false
    let usesWhiteSelectedText = false
    let accentLineWidth = 3
    let rowCornerRadius = 7
}

struct SidebarSecondaryListPresentation: Equatable {
    let minimumSearchWidth = 220
    let compactRowMinHeight = 40
    let compactRowMaxHeight = 44
    let usesSingleLineFilterToolbar = true
    let refreshUsesIconOnly = true
}

struct SkillListPresentation: Equatable {
    let minimumPrimaryColumnWidth = 220
    let idealPrimaryColumnWidth = 240
    let maximumPrimaryColumnWidth = 320
    let minimumSecondaryColumnWidth = MainWindowModel.minimumCompactSecondaryWidth
    let idealSecondaryColumnWidth = 360
    let maximumSecondaryColumnWidth = 440
    let minimumSearchWidth = 220
    let compactRowMinHeight = 36
    let compactRowMaxHeight = 40
    let usesSingleLineFilterToolbar = true
    let filterControlWidth = 72
    let filterControlHeight = 28
    let filterControlSpacing = 4
    let filterToolbarVerticalPadding = 4
    let sortDirectionButtonWidth = 28

    func emptyFilteredMessage(
        agentFilter: SkillAgentFilter,
        hasActiveProjectContext: Bool,
        hasActiveSearchOrFilter: Bool
    ) -> String {
        if hasActiveSearchOrFilter {
            if agentFilter == .codex {
                return UIStrings.noCodexSkillsMessage
            }
            if agentFilter == .openclaw {
                return UIStrings.noOpenClawWorkspaceSkillsMessage
            }
            return UIStrings.noSkillsMatchSearch
        }

        if agentFilter == .codex, !hasActiveProjectContext {
            return UIStrings.noCodexProjectMessage
        }
        if agentFilter == .openclaw {
            return UIStrings.noOpenClawWorkspaceSkillsMessage
        }
        if agentFilter == .all {
            return hasActiveProjectContext ? UIStrings.noSkillsInCatalog : UIStrings.noProjectSkillsMessage
        }
        return UIStrings.noAgentSkillsMessage(agentFilter.title)
    }
}

enum SkillRowMetadataPresentation {
    static func metadataText(for skill: SkillRecord) -> String {
        let packageContext = skill.pluginPackageSummary
            ?? skill.packageVersion.map { "v\($0)" }
            ?? skill.sourceKind
        if DisplayText.isToolGlobal(skill) {
            return [
                DisplayText.scope(for: skill),
                UIStrings.readOnlyPreview,
                packageContext ?? skill.provenance.label
            ].joined(separator: " · ")
        }
        if skill.agent == "hermes", DisplayText.isReadOnlyPreview(skill) {
            return "\(DisplayText.scope(for: skill)) · \(skill.provenance.label)"
        }
        if DisplayText.isReadOnlyPreview(skill) {
            return [
                DisplayText.scope(for: skill),
                UIStrings.readOnly,
                packageContext ?? skill.provenance.label
            ].joined(separator: " · ")
        }
        var parts = [
            DisplayText.scope(for: skill),
            DisplayText.state(skill.state, enabled: skill.enabled)
        ]
        parts.append(packageContext ?? skill.provenance.label)
        return parts.joined(separator: " · ")
    }

    static func skillIDsRequiringVisibleMetadata(in skills: [SkillRecord]) -> Set<SkillRecord.ID> {
        guard skills.count > 1 else { return Set(skills.map(\.id)) }

        let signatures = skills.map { metadataText(for: $0) }
        let counts = Dictionary(grouping: signatures, by: { $0 }).mapValues(\.count)
        let ranked = counts.sorted {
            if $0.value != $1.value {
                return $0.value > $1.value
            }
            return $0.key.localizedCaseInsensitiveCompare($1.key) == .orderedAscending
        }
        guard let dominant = ranked.first, dominant.value > 1 else {
            return Set(skills.map(\.id))
        }
        if ranked.count > 1, ranked[1].value == dominant.value {
            return Set(skills.map(\.id))
        }

        return Set(skills.compactMap { skill in
            metadataText(for: skill) == dominant.key ? nil : skill.id
        })
    }
}

struct DetailHeaderPresentation: Equatable {
    let height = 48
    let definitionUsesMonospacedFont = true
    let primaryToggleLivesInMenu = true
    let metadataLabelWidth = 82
    let metadataRowHeight = 30
}

struct DetailFeedbackPresentation: Equatable {
    let usesOverlayToast = false
    let maximumWidth = 420
    let cornerRadius = 8
}

struct ConfigEditorPresentation: Equatable {
    let usesSingleCodeCard = true
    let showsLineNumbers = true
    let usesCompactToolbarActions = true
    let primarySaveButtonVisible = false
    let autosaveEnabled = true
    let autosaveDelayNanoseconds: UInt64 = 900_000_000
    let codeCardMinHeight = 320
    let lineNumberGutterWidth = 42
}

struct SettingsPresentation: Equatable {
    let navigationStyle = SettingsNavigationStyle.sidebar
    let usesDedicatedSettingsScene = true
    let windowControlPolicy = SettingsWindowControlPolicy.closeOnly
    let primarySaveButtonsVisible = false
    let sidebarWidth = 190
    let minimumWidth = 760
    let idealWidth = 860
    let minimumHeight = 620
    let idealHeight = 680
    let usesUnifiedSectionHeaders = true
    let sectionCornerRadius = 8
    let providerObservabilityAutoLoadsAtStartup = true
    let providerObservabilityHasLocalBuildAction = false
    let providerObservabilityHidesRawLogList = true
    let providerObservabilitySummaryMetricCount = 5
    let providerObservabilityChartRowLimit = 5
    let providerObservabilityUsesScopedScroll = true
    let providerObservabilityDisablesSelectionOverlay = true
    let providerObservabilityAvoidsAdaptiveGrids = true
}

struct WorkflowSheetPresentation: Equatable {
    let titlebarStyle = WorkflowSheetTitlebarStyle.liquidGlass
    let closeActionPlacement = WorkflowSheetCloseActionPlacement.trailingTitlebar
    let feedbackStyle = WorkflowSheetFeedbackStyle.inlineTintedBanner
    let columnLayout = WorkflowSheetColumnLayout.twoColumn
    let titlebarHeight = 58
    let columnSpacing = 14
    let secondaryColumnWidth = 320
}

struct TaskPreflightPresentation: Equatable {
    let sheetContentLayout = TaskPreflightSheetContentLayout.editorWithHistory
    let sheetMinimumWidth = 950
    let sheetIdealWidth = 1_020
    let sheetMinimumHeight = 620
    let historyColumnWidth = 270
    let fixedAgentChipWidth = 0
    let agentGridColumnCount = 3
    let showsProviderUnavailableGate = true
}

struct SkillManagerPresentation: Equatable {
    let sheetContentLayout = SkillManagerSheetContentLayout.controlsWithResults
    let sheetMinimumWidth = 900
    let sheetIdealWidth = 980
    let sheetMinimumHeight = 680
    let sheetIdealHeight = 760
    let usesSegmentedWorkflows = true
    let targetsSummaryIsPinned = true
    let toolUnavailableDisablesExternalMutations = true
    let usesSurfaceLocalFeedback = true
}

struct CompactMetadataRow: Identifiable, Hashable {
    let label: String
    let value: String
    var systemImage: String? = nil
    var isCopyable = false

    var id: String {
        "\(label)-\(value)-\(systemImage ?? "")-\(isCopyable)"
    }
}
