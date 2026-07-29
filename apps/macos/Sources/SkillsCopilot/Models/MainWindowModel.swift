import CoreGraphics

enum AppAccessibilityID {
    static let mainWindow = "skills-copilot.main-window"
    static let mainContent = "skills-copilot.main-content"
    static let taskCockpitPanel = "skills-copilot.task-cockpit.panel"
    static let taskCockpitInput = "skills-copilot.task-cockpit.input"
    static let taskCockpitInputStatus = "skills-copilot.task-cockpit.input.status"
    static let taskCockpitBuildButton = "skills-copilot.task-cockpit.build"
    static let taskCockpitStatus = "skills-copilot.task-cockpit.status"
    static let taskCockpitCancelButton = "skills-copilot.task-cockpit.cancel"
    static let taskCockpitRetryButton = "skills-copilot.task-cockpit.retry"
    static let taskCockpitStageProgress = "skills-copilot.task-cockpit.stage-progress"
    static let taskCockpitResult = "skills-copilot.task-cockpit.result"
}

enum CompactWorkspaceLayer: Equatable {
    case listOnly
    case detailOverlay
    case detailRevealControl
}

enum WindowChromeLayoutMode: Equatable {
    case regular
    case compact
}

enum MainWindowModel {
    static let mainSceneIdentifier = "main"
    static let windowIdentifierRawValue = AppAccessibilityID.mainWindow
    static let autosaveName = "SkillsCopilot.MainWindow"
    static let minimumWidth = 1024
    static let minimumHeight = 600
    static let compactLayoutBreakpoint = 1180
    static let maximumReadableDetailWidth = 800
    static let minimumCompactSecondaryWidth: CGFloat = 360
    static let minimumCompactVisibleListContextWidth: CGFloat = 180
    static let minimumCompactDetailWidth: CGFloat = 560
    static let compactDetailPreferredFraction: CGFloat = 0.72

    static func usesCompactLayout(width: CGFloat) -> Bool {
        width < CGFloat(compactLayoutBreakpoint)
    }

    static func windowChromeLayoutMode(width: CGFloat) -> WindowChromeLayoutMode {
        usesCompactLayout(width: width) ? .compact : .regular
    }

    static func compactWorkspaceLayer(
        selection: SidebarSelection?,
        showsDetail: Bool
    ) -> CompactWorkspaceLayer {
        guard selection != nil else { return .listOnly }
        return showsDetail ? .detailOverlay : .detailRevealControl
    }

    static func showsCompactDetailRevealControl(
        layer: CompactWorkspaceLayer,
        isWorkspaceHovered: Bool,
        isApplicationActive: Bool
    ) -> Bool {
        layer == .detailRevealControl
            && isWorkspaceHovered
            && isApplicationActive
    }

    static func compactDetailWidth(availableWidth: CGFloat) -> CGFloat {
        let normalizedWidth = max(0, availableWidth)
        let boundedMinimumWidth = min(minimumCompactDetailWidth, normalizedWidth)
        let preferredWidth = min(
            CGFloat(maximumReadableDetailWidth),
            max(boundedMinimumWidth, normalizedWidth * compactDetailPreferredFraction)
        )
        let maximumWidthPreservingListContext = max(
            0,
            normalizedWidth - minimumCompactVisibleListContextWidth
        )
        guard maximumWidthPreservingListContext >= boundedMinimumWidth else {
            return min(preferredWidth, normalizedWidth)
        }
        return min(preferredWidth, maximumWidthPreservingListContext)
    }

    static func compactSecondaryContentWidth(availableWidth: CGFloat) -> CGFloat {
        max(0, availableWidth)
    }

    static func compactVisibleSecondaryWidth(availableWidth: CGFloat) -> CGFloat {
        let normalizedWidth = max(0, availableWidth)
        return max(0, normalizedWidth - compactDetailWidth(availableWidth: normalizedWidth))
    }

}
