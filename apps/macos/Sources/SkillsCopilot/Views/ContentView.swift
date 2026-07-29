import AppKit
import SwiftUI

struct ContentView: View {
    @EnvironmentObject private var store: SkillStore
    @Environment(\.accessibilityReduceMotion) private var reduceMotion
    @State private var globalSearchText = ""
    @State private var isGlobalSearchFocused = false
    @State private var showsGlobalSearchResults = false
    @State private var columnVisibility: NavigationSplitViewVisibility = .all
    @State private var showsCompactDetail = true
    @State private var showsFirstRunOnboarding = false
    @State private var windowChromeLayoutMode = WindowChromeLayoutMode.compact
    @AppStorage(FirstRunOnboardingModel.completionStorageKey)
    private var hasCompletedOnboarding = false

    var body: some View {
        GeometryReader { proxy in
            let layoutMode = MainWindowModel.windowChromeLayoutMode(width: proxy.size.width)

            ZStack(alignment: .topTrailing) {
                appShell
                    .opacity(store.startupLoadingState == nil ? 1 : 0)
                    .allowsHitTesting(store.startupLoadingState == nil)
                    .accessibilityHidden(store.startupLoadingState != nil)

                if let state = store.startupLoadingState {
                    AppStartupLoadingView(state: state)
                        .transition(.opacity)
                }

                if store.startupLoadingState == nil, store.isProjectUpdating {
                    ProjectTransitionLoadingView(
                        projectName: store.projectTransitionName ?? UIStrings.projectSelectedSource
                    )
                    .transition(.opacity)
                    .zIndex(9)
                }

                if shouldShowGlobalSearchResultsOverlay {
                    globalSearchResultsOverlay(layoutMode: layoutMode)
                        .transition(.opacity.combined(with: .move(edge: .top)))
                        .zIndex(8)
                }

                pinnedWindowChromeControls
            }
            .onAppear {
                windowChromeLayoutMode = layoutMode
            }
            .onChange(of: proxy.size.width) { width in
                windowChromeLayoutMode = MainWindowModel.windowChromeLayoutMode(width: width)
            }
        }
        .task {
            await store.loadAppStartupDataIfNeeded()
            if !hasCompletedOnboarding {
                showsFirstRunOnboarding = true
            }
        }
        .onChange(of: hasCompletedOnboarding) { completed in
            if !completed, store.hasCompletedStartupLoad {
                showsFirstRunOnboarding = true
            }
        }
        .onChange(of: trimmedGlobalSearchText) { query in
            store.updateAppSearch(query: query)
        }
        .onChange(of: isGlobalSearchFocused) { focused in
            guard focused, !trimmedGlobalSearchText.isEmpty else { return }
            store.updateAppSearch(query: trimmedGlobalSearchText)
        }
        .onChange(of: store.selectedAgentLocalSessionRefreshKey) { _ in
            guard !trimmedGlobalSearchText.isEmpty else { return }
            store.updateAppSearch(query: trimmedGlobalSearchText)
        }
        .transaction { transaction in
            if reduceMotion {
                transaction.animation = nil
            }
        }
        .accessibilityIdentifier(AppAccessibilityID.mainContent)
        .accessibilityLabel(UIStrings.appWindowTitle)
        .sheet(isPresented: $showsFirstRunOnboarding) {
            FirstRunOnboardingView(hasCompletedOnboarding: $hasCompletedOnboarding)
                .environmentObject(store)
        }
    }

    private var pinnedWindowChromeControls: some View {
        WindowChromeTitlebarAccessory(
            layout: WindowChromeTitlebarAccessoryLayout(
                contentWidth: WindowChromeToolbarMetrics.totalWidth(
                    for: windowChromeLayoutMode
                ),
                contentHeight: WindowChromeToolbarMetrics.controlHeight,
                trailingPadding: WindowChromeToolbarMetrics.toolbarTrailingPadding
            )
        ) {
            WindowChromeToolbarControls(
                layoutMode: windowChromeLayoutMode,
                text: $globalSearchText,
                isSearchFocused: $isGlobalSearchFocused,
                showsSearchResults: $showsGlobalSearchResults,
                onSubmit: selectFirstGlobalSearchResult
            )
            .environmentObject(store)
        }
        .frame(width: 0, height: 0)
        .accessibilityHidden(true)
        .zIndex(10)
    }

    private var appShell: some View {
        navigationShell
    }

    private var navigationShell: some View {
        GeometryReader { proxy in
            if MainWindowModel.usesCompactLayout(width: proxy.size.width) {
                compactNavigationShell
            } else {
                regularNavigationShell
            }
        }
        .task(id: store.selectedAgentLocalSessionRefreshKey) {
            guard store.hasCompletedStartupLoad else { return }
            await store.refreshSelectedAgentLocalSessionsIfNeeded()
        }
        .onChange(of: store.selectedSkillID) { _ in
            guard store.hasCompletedStartupLoad else { return }
            Task { await store.loadSelectedDetail() }
        }
        .onChange(of: store.selectedSidebarSelection) { selection in
            if selection != nil {
                showsCompactDetail = true
            }
        }
    }

    private var regularNavigationShell: some View {
        NavigationSplitView(columnVisibility: $columnVisibility) {
            SidebarView()
                .navigationSplitViewColumnWidth(
                    min: CGFloat(UIOptimizationPresentation.sidebarShell.width),
                    ideal: CGFloat(UIOptimizationPresentation.sidebarShell.width),
                    max: CGFloat(UIOptimizationPresentation.sidebarShell.width)
                )
        } content: {
            SecondarySidebarView(columnVisibility: columnVisibility)
                .navigationSplitViewColumnWidth(
                    min: CGFloat(UIOptimizationPresentation.skillList.minimumSecondaryColumnWidth),
                    ideal: CGFloat(UIOptimizationPresentation.skillList.idealSecondaryColumnWidth),
                    max: CGFloat(UIOptimizationPresentation.skillList.maximumSecondaryColumnWidth)
                )
        } detail: {
            DetailView(skill: store.selectedSkill)
        }
    }

    private var compactNavigationShell: some View {
        NavigationSplitView(columnVisibility: $columnVisibility) {
            SidebarView()
                .navigationSplitViewColumnWidth(
                    min: CGFloat(UIOptimizationPresentation.sidebarShell.compactWidth),
                    ideal: CGFloat(UIOptimizationPresentation.sidebarShell.compactWidth),
                    max: CGFloat(UIOptimizationPresentation.sidebarShell.width)
                )
        } detail: {
            CompactWorkspaceView(
                columnVisibility: columnVisibility,
                showsDetail: $showsCompactDetail
            )
        }
    }

    private var trimmedGlobalSearchText: String {
        globalSearchText.trimmingCharacters(in: .whitespacesAndNewlines)
    }

    private var shouldShowGlobalSearchResultsOverlay: Bool {
        store.startupLoadingState == nil && showsGlobalSearchResults && !trimmedGlobalSearchText.isEmpty
    }

    private func globalSearchResultsOverlay(layoutMode: WindowChromeLayoutMode) -> some View {
        GlobalSearchResultsOverlay(
            query: trimmedGlobalSearchText,
            results: globalSearchResults,
            kindCounts: store.appSearchResult.kindCounts,
            isLoading: store.isSearchingApp,
            fallbackReason: store.appSearchResult.fallbackReason,
            onViewAll: showAllGlobalSearchResults
        ) { result in
            showsGlobalSearchResults = false
            isGlobalSearchFocused = false
            selectGlobalSearchResult(result)
        }
        .padding(.top, WindowChromeToolbarMetrics.searchResultsTopPadding)
        .padding(
            .trailing,
            WindowChromeToolbarMetrics.searchResultsTrailingPadding(for: layoutMode)
        )
        .accessibilitySortPriority(2)
    }

    private var globalSearchResults: [AppSearchItem] {
        guard !trimmedGlobalSearchText.isEmpty,
              store.appSearchResult.query == trimmedGlobalSearchText
        else { return [] }
        return store.appSearchResult.items
    }

    private func selectFirstGlobalSearchResult() {
        guard let result = globalSearchResults.first else { return }
        selectGlobalSearchResult(result)
    }

    private func selectGlobalSearchResult(_ result: AppSearchItem) {
        Task { @MainActor in
            await store.selectAppSearchItem(result)
            globalSearchText = ""
            showsGlobalSearchResults = false
        }
    }

    private func showAllGlobalSearchResults(_ kind: AppSearchItemKind) {
        let query = trimmedGlobalSearchText
        Task { @MainActor in
            await store.showAllAppSearchResults(kind: kind, query: query)
            globalSearchText = ""
            showsGlobalSearchResults = false
            isGlobalSearchFocused = false
        }
    }
}

private struct CompactWorkspaceView: View {
    @EnvironmentObject private var store: SkillStore
    @State private var isWorkspaceHovered = false
    @State private var isApplicationActive = NSApplication.shared.isActive
    let columnVisibility: NavigationSplitViewVisibility
    @Binding var showsDetail: Bool

    var body: some View {
        GeometryReader { proxy in
            let layer = MainWindowModel.compactWorkspaceLayer(
                selection: store.selectedSidebarSelection,
                showsDetail: showsDetail
            )
            let availableWidth = proxy.size.width
            let detailWidth = detailWidth(availableWidth: availableWidth)
            let secondaryWidth = MainWindowModel.compactSecondaryContentWidth(
                availableWidth: availableWidth
            )

            ZStack(alignment: .trailing) {
                SecondarySidebarView(columnVisibility: columnVisibility)
                    .frame(width: secondaryWidth)
                    .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .leading)
                    .contentShape(Rectangle())
                    .onHover { isHovering in
                        isWorkspaceHovered = isHovering
                    }
                    .clipped()

                switch layer {
                case .detailOverlay:
                    detailOverlay(width: detailWidth)
                        .transition(.move(edge: .trailing).combined(with: .opacity))
                        .zIndex(2)
                case .detailRevealControl:
                    if MainWindowModel.showsCompactDetailRevealControl(
                        layer: layer,
                        isWorkspaceHovered: isWorkspaceHovered,
                        isApplicationActive: isApplicationActive
                    ) {
                        showDetailButton
                            .padding(12)
                            .transition(.opacity.combined(with: .scale(scale: 0.96)))
                            .zIndex(1)
                    }
                case .listOnly:
                    EmptyView()
                }
            }
        }
        .animation(.easeInOut(duration: 0.18), value: showsDetail)
        .animation(.easeOut(duration: 0.12), value: isWorkspaceHovered)
        .onReceive(
            NotificationCenter.default.publisher(
                for: NSApplication.didResignActiveNotification
            )
        ) { _ in
            isApplicationActive = false
        }
        .onReceive(
            NotificationCenter.default.publisher(
                for: NSApplication.didBecomeActiveNotification
            )
        ) { _ in
            isApplicationActive = true
        }
    }

    private func detailOverlay(width: CGFloat) -> some View {
        VStack(spacing: 0) {
            HStack(spacing: 10) {
                Label(UIStrings.detailSection, systemImage: "sidebar.trailing")
                    .font(.caption.weight(.semibold))
                    .foregroundStyle(.secondary)

                Spacer(minLength: 8)

                Button {
                    showsDetail = false
                } label: {
                    Label(
                        UIStrings.text("detail.compact.close", "Close Details"),
                        systemImage: "xmark"
                    )
                    .labelStyle(.iconOnly)
                    .frame(width: 32, height: 32)
                    .contentShape(Rectangle())
                }
                .buttonStyle(.plain)
                .help(UIStrings.text("detail.compact.close", "Close Details"))
                .accessibilityLabel(UIStrings.text("detail.compact.close", "Close Details"))
            }
            .padding(.leading, 14)
            .padding(.trailing, 8)
            .frame(height: 42)
            .background(.bar)

            Divider()

            DetailView(skill: store.selectedSkill)
        }
        .frame(width: width)
        .frame(maxHeight: .infinity)
        .background(Color(nsColor: .windowBackgroundColor))
        .overlay(alignment: .leading) {
            Rectangle()
                .fill(Color.secondary.opacity(0.18))
                .frame(width: 1)
        }
        .shadow(color: .black.opacity(0.14), radius: 18, x: -6, y: 0)
        .accessibilityElement(children: .contain)
        .accessibilityLabel(UIStrings.text("detail.compact.overlay", "Selected item details"))
    }

    private var showDetailButton: some View {
        Button {
            showsDetail = true
        } label: {
            Label(
                UIStrings.text("detail.compact.show", "Show Details"),
                systemImage: "sidebar.trailing"
            )
        }
        .buttonStyle(.bordered)
        .controlSize(.small)
        .help(UIStrings.text("detail.compact.show", "Show Details"))
        .accessibilityLabel(UIStrings.text("detail.compact.show", "Show Details"))
    }

    private func detailWidth(availableWidth: CGFloat) -> CGFloat {
        MainWindowModel.compactDetailWidth(availableWidth: availableWidth)
    }
}

private enum WindowChromeToolbarMetrics {
    static let controlHeight: CGFloat = 34
    static let agentWidth: CGFloat = 146
    static let projectWidth: CGFloat = 210
    static let compactAgentWidth: CGFloat = 132
    static let compactProjectWidth: CGFloat = 34
    static let trailingSpacing: CGFloat = 6
    static let iconButtonWidth: CGFloat = 34
    static let refreshWidth = CGFloat(UIOptimizationPresentation.unifiedToolbar.refreshControlWidth)
    static let refreshHorizontalPadding = CGFloat(
        UIOptimizationPresentation.unifiedToolbar.refreshHorizontalPadding
    )
    static let refreshStatusSlotWidth = CGFloat(
        UIOptimizationPresentation.unifiedToolbar.refreshStatusSlotWidth
    )
    static let toolbarTrailingPadding: CGFloat = 12
    static let regularSearchWidth = CGFloat(UIOptimizationPresentation.unifiedToolbar.idealGlobalSearchWidth)
    static let compactSearchWidth = CGFloat(UIOptimizationPresentation.unifiedToolbar.minimumGlobalSearchWidth)
    static let searchResultsWidth: CGFloat = 460
    static let searchResultsMinHeight: CGFloat = 180
    static let searchResultsMaxHeight: CGFloat = 340
    static let searchResultsTopPadding: CGFloat = 10

    static func agentWidth(for layoutMode: WindowChromeLayoutMode) -> CGFloat {
        layoutMode == .compact ? compactAgentWidth : agentWidth
    }

    static func projectWidth(for layoutMode: WindowChromeLayoutMode) -> CGFloat {
        layoutMode == .compact ? compactProjectWidth : projectWidth
    }

    static func searchWidth(for layoutMode: WindowChromeLayoutMode) -> CGFloat {
        switch layoutMode {
        case .regular:
            regularSearchWidth
        case .compact:
            compactSearchWidth
        }
    }

    static func totalWidth(for layoutMode: WindowChromeLayoutMode) -> CGFloat {
        agentWidth(for: layoutMode)
            + projectWidth(for: layoutMode)
            + searchWidth(for: layoutMode)
            + refreshWidth
            + iconButtonWidth
            + 8 * 2
            + trailingSpacing * 2
    }

    static func searchResultsTrailingPadding(for layoutMode: WindowChromeLayoutMode) -> CGFloat {
        switch layoutMode {
        case .regular:
            return toolbarTrailingPadding
                + refreshWidth
                + iconButtonWidth
                + trailingSpacing * 2
        case .compact:
            return toolbarTrailingPadding
                + refreshWidth
                + iconButtonWidth
                + trailingSpacing * 2
        }
    }
}

private struct WindowChromeToolbarControls: View {
    let layoutMode: WindowChromeLayoutMode
    @Binding var text: String
    @Binding var isSearchFocused: Bool
    @Binding var showsSearchResults: Bool
    let onSubmit: () -> Void

    var body: some View {
        HStack(spacing: 8) {
            TitlebarAgentSelectorControl()
                .frame(width: agentWidth, height: controlHeight, alignment: .leading)

            TitlebarProjectPickerControl(isCompact: layoutMode == .compact)
                .frame(width: projectWidth, height: controlHeight, alignment: .leading)

            WindowChromeTrailingControls(
                layoutMode: layoutMode,
                text: $text,
                isSearchFocused: $isSearchFocused,
                showsSearchResults: $showsSearchResults,
                onSubmit: onSubmit
            )
        }
        .frame(height: controlHeight, alignment: .leading)
        .fixedSize(horizontal: true, vertical: false)
    }

    private var controlHeight: CGFloat { WindowChromeToolbarMetrics.controlHeight }
    private var agentWidth: CGFloat {
        WindowChromeToolbarMetrics.agentWidth(for: layoutMode)
    }
    private var projectWidth: CGFloat {
        WindowChromeToolbarMetrics.projectWidth(for: layoutMode)
    }
}

private struct TitlebarAgentSelectorControl: View {
    @EnvironmentObject private var store: SkillStore
    @State private var isPopoverPresented = false

    var body: some View {
        Button {
            isPopoverPresented.toggle()
        } label: {
            TitlebarAgentSelectorLabel(
                filter: store.agentFilter,
                title: shortTitle(for: store.agentFilter)
            )
        }
        .buttonStyle(.plain)
        .popover(isPresented: $isPopoverPresented, arrowEdge: .top) {
            VStack(alignment: .leading, spacing: 4) {
                ForEach(SkillAgentFilter.managementCases) { filter in
                    Button {
                        store.agentFilter = filter
                        isPopoverPresented = false
                    } label: {
                        Label(shortTitle(for: filter), systemImage: filter == store.agentFilter ? "checkmark" : systemImage(for: filter))
                            .frame(maxWidth: .infinity, alignment: .leading)
                    }
                    .buttonStyle(.plain)
                    .padding(.horizontal, 8)
                    .padding(.vertical, 6)
                    .contentShape(RoundedRectangle(cornerRadius: 8))
                }
            }
            .padding(8)
            .frame(width: 180, alignment: .leading)
        }
        .help("\(UIStrings.text("help.agentSelector", "Select the agent workspace.")) \(store.agentFilter.title)")
        .accessibilityLabel(UIStrings.agent)
        .accessibilityValue(store.agentFilter.title)
    }

    private func shortTitle(for filter: SkillAgentFilter) -> String {
        switch filter {
        case .claudeCode:
            return UIStrings.claudeCode
        case .codex:
            return UIStrings.codex
        case .opencode:
            return UIStrings.opencode
        case .pi:
            return UIStrings.pi
        case .hermes:
            return UIStrings.hermes
        case .openclaw:
            return UIStrings.openclaw
        case .all:
            return UIStrings.text("filter.all", "All")
        }
    }

    private func systemImage(for filter: SkillAgentFilter) -> String {
        switch filter {
        case .claudeCode:
            return "sparkle"
        case .codex:
            return "terminal"
        case .opencode:
            return "curlybraces"
        case .pi:
            return "pi"
        case .hermes:
            return "bolt"
        case .openclaw:
            return "shippingbox"
        case .all:
            return "square.grid.2x2"
        }
    }
}

private struct TitlebarAgentSelectorLabel: View {
    let filter: SkillAgentFilter
    let title: String

    var body: some View {
        HStack(spacing: 8) {
            TitlebarAgentIconBadge(filter: filter, size: 24)

            Text(title)
                .font(.headline.weight(.semibold))
                .foregroundStyle(.primary)
                .lineLimit(1)
                .minimumScaleFactor(0.75)

            Image(systemName: "chevron.up.chevron.down")
                .font(.system(size: 10, weight: .semibold))
                .foregroundStyle(.secondary)
                .accessibilityHidden(true)
        }
        .padding(.leading, 12)
        .padding(.trailing, 10)
        .frame(minWidth: 118, maxWidth: .infinity, minHeight: 32, maxHeight: 32, alignment: .leading)
        .titlebarChromeControlCapsule()
        .contentShape(Capsule())
        .accessibilityElement(children: .combine)
        .accessibilityLabel(title)
    }
}

private struct TitlebarProjectPickerControl: View {
    @EnvironmentObject private var store: SkillStore
    @State private var isPopoverPresented = false
    let isCompact: Bool

    var body: some View {
        Button {
            isPopoverPresented.toggle()
        } label: {
            TitlebarProjectPickerLabel(
                title: projectTitle,
                systemImage: statusImage ?? "folder.badge.plus",
                isWarning: statusImage == "exclamationmark.triangle.fill",
                isCompact: isCompact
            )
        }
        .buttonStyle(.plain)
        .disabled(store.isRefreshBusy)
        .popover(isPresented: $isPopoverPresented, arrowEdge: .top) {
            VStack(alignment: .leading, spacing: 6) {
                Button {
                    isPopoverPresented = false
                    chooseProject()
                } label: {
                    Label(UIStrings.chooseProject, systemImage: "folder.badge.plus")
                        .frame(maxWidth: .infinity, alignment: .leading)
                }
                .buttonStyle(.plain)
                .padding(.horizontal, 8)
                .padding(.vertical, 6)

                if !store.recentProjectContexts.isEmpty {
                    Divider()
                    HStack(spacing: 8) {
                        Text(UIStrings.recentProjects)
                            .font(.caption.weight(.semibold))
                            .foregroundStyle(.secondary)

                        Spacer(minLength: 8)

                        Button(role: .destructive) {
                            Task { await store.clearRecentProjects() }
                        } label: {
                            Text(UIStrings.clearRecentProjectsCompact)
                                .font(.caption)
                        }
                        .buttonStyle(.plain)
                        .help(UIStrings.clearRecentProjects)
                        .accessibilityLabel(UIStrings.clearRecentProjects)
                    }
                    .padding(.horizontal, 8)

                    ForEach(store.recentProjectContexts) { context in
                        HStack(spacing: 4) {
                            Button {
                                isPopoverPresented = false
                                Task {
                                    await store.setProject(
                                        rootPath: context.rootPath,
                                        currentCWD: context.currentCWD,
                                        name: context.name
                                    )
                                }
                            } label: {
                                HStack(spacing: 8) {
                                    VStack(alignment: .leading, spacing: 1) {
                                        Text(context.name)
                                            .lineLimit(1)
                                        Text(recentProjectPath(context))
                                            .font(.caption2)
                                            .foregroundStyle(.secondary)
                                            .lineLimit(1)
                                            .truncationMode(.middle)
                                    }
                                    .frame(maxWidth: .infinity, alignment: .leading)

                                    if context.isActive {
                                        Label(UIStrings.currentProject, systemImage: "checkmark")
                                            .font(.caption2.weight(.semibold))
                                            .foregroundStyle(Color.accentColor)
                                            .fixedSize()
                                    }
                                }
                                .frame(maxWidth: .infinity, alignment: .leading)
                            }
                            .buttonStyle(.plain)
                            .padding(.vertical, 6)
                            .accessibilityValue(context.isActive ? UIStrings.currentProject : "")

                            if !context.isActive {
                                Button(role: .destructive) {
                                    Task { await store.removeRecentProject(id: context.id) }
                                } label: {
                                    Image(systemName: "trash")
                                        .frame(width: 24, height: 24)
                                        .contentShape(Rectangle())
                                }
                                .buttonStyle(.plain)
                                .foregroundStyle(.secondary)
                                .help(UIStrings.removeRecentProject(context.name, path: recentProjectPath(context)))
                                .accessibilityLabel(UIStrings.removeRecentProject(context.name, path: recentProjectPath(context)))
                            }
                        }
                        .padding(.horizontal, 8)
                        .background(
                            context.isActive ? Color.accentColor.opacity(0.08) : Color.clear,
                            in: RoundedRectangle(cornerRadius: 8)
                        )
                    }
                }

                if store.activeProjectContext != nil {
                    Divider()

                    Button {
                        isPopoverPresented = false
                        revealActiveProject()
                    } label: {
                        Label(UIStrings.revealInFinder, systemImage: "arrow.up.forward.app")
                            .frame(maxWidth: .infinity, alignment: .leading)
                    }
                    .buttonStyle(.plain)
                    .padding(.horizontal, 8)
                    .padding(.vertical, 6)

                    Button(role: .destructive) {
                        isPopoverPresented = false
                        Task { await store.clearProject() }
                    } label: {
                        Label(UIStrings.clearProject, systemImage: "xmark.circle")
                            .frame(maxWidth: .infinity, alignment: .leading)
                    }
                    .buttonStyle(.plain)
                    .padding(.horizontal, 8)
                    .padding(.vertical, 6)
                }
            }
            .padding(8)
            .frame(width: 300, alignment: .leading)
        }
        .help(projectHelp)
        .accessibilityLabel(UIStrings.text("project.chooseMenu", "Project"))
        .accessibilityValue(projectTitle)
    }

    private var projectTitle: String {
        guard let project = store.activeProjectContext else {
            return UIStrings.toolbarNoProjectSelected
        }
        let trimmedName = project.name.trimmingCharacters(in: .whitespacesAndNewlines)
        if !trimmedName.isEmpty {
            return trimmedName
        }
        let lastPathComponent = URL(fileURLWithPath: project.rootPath).lastPathComponent
        return lastPathComponent.isEmpty ? UIStrings.text("project.selected", "Selected project") : lastPathComponent
    }

    private var projectHelp: String {
        if let validationMessage = store.projectValidationMessage {
            return "\(projectTitle): \(validationMessage)"
        }
        if let rootPath = store.activeProjectContext?.rootPath, !rootPath.isEmpty {
            return "\(projectTitle), \(DisplayText.privacyPath(rootPath, privacyModeEnabled: true))"
        }
        return "\(projectTitle), \(UIStrings.projectGlobalRootsOnly)"
    }

    private var statusImage: String? {
        if store.projectValidationMessage != nil {
            return "exclamationmark.triangle.fill"
        }
        if store.agentFilter == .openclaw {
            return "folder.badge.questionmark"
        }
        return nil
    }

    private func chooseProject() {
        let panel = NSOpenPanel()
        panel.canChooseFiles = false
        panel.canChooseDirectories = true
        panel.allowsMultipleSelection = false
        panel.canCreateDirectories = false
        panel.prompt = UIStrings.chooseProject

        if panel.runModal() == .OK, let url = panel.url {
            Task {
                await store.setProject(
                    rootPath: url.path,
                    currentCWD: url.path,
                    name: url.lastPathComponent
                )
            }
        }
    }

    private func revealActiveProject() {
        guard let rootPath = store.activeProjectContext?.rootPath else { return }
        NSWorkspace.shared.activateFileViewerSelecting([URL(fileURLWithPath: rootPath)])
    }

    private func recentProjectPath(_ context: ProjectContext) -> String {
        DisplayText.privacyPath(context.rootPath, privacyModeEnabled: true)
    }
}

private struct TitlebarProjectPickerLabel: View {
    let title: String
    let systemImage: String
    let isWarning: Bool
    let isCompact: Bool

    var body: some View {
        if isCompact {
            collapsedLabel
        } else {
            ViewThatFits(in: .horizontal) {
                expandedLabel
                collapsedLabel
            }
        }
    }

    private var expandedLabel: some View {
        HStack(spacing: 8) {
            icon

            Text(title)
                .font(.headline.weight(.semibold))
                .foregroundStyle(.primary)
                .lineLimit(1)
                .truncationMode(.middle)

            Image(systemName: "chevron.up.chevron.down")
                .font(.system(size: 10, weight: .semibold))
                .foregroundStyle(.secondary)
                .accessibilityHidden(true)
        }
        .padding(.leading, 11)
        .padding(.trailing, 10)
        .frame(minWidth: 120, maxWidth: .infinity, minHeight: 32, maxHeight: 32, alignment: .leading)
        .titlebarChromeControlCapsule()
        .contentShape(Capsule())
        .accessibilityElement(children: .combine)
        .accessibilityLabel(title)
    }

    private var collapsedLabel: some View {
        icon
            .frame(width: 32, height: 32)
            .titlebarChromeControlCircle()
            .contentShape(Circle())
            .accessibilityLabel(title)
    }

    private var icon: some View {
        Image(systemName: systemImage)
            .font(.system(size: 15, weight: .semibold))
            .foregroundStyle(isWarning ? Color.orange : Color.accentColor)
            .frame(width: 18, height: 18)
            .accessibilityHidden(true)
    }
}

private struct TitlebarAgentIconBadge: View {
    let filter: SkillAgentFilter
    var size: CGFloat = 28

    var body: some View {
        ZStack {
            if let image = AgentIconProvider.image(for: filter) {
                Image(nsImage: image)
                    .resizable()
                    .scaledToFit()
                    .frame(width: imageSize, height: imageSize)
                    .clipShape(RoundedRectangle(cornerRadius: imageCornerRadius))
                    .accessibilityLabel(DisplayText.agent(filter.rawValue))
            } else {
                Image(systemName: fallbackSystemImage)
                    .font(.system(size: fallbackIconSize, weight: .semibold))
                    .foregroundStyle(Color.accentColor)
                    .accessibilityLabel(DisplayText.agent(filter.rawValue))
            }
        }
        .frame(width: size, height: size)
        .background(Color.agentCopilotPanelBackground, in: RoundedRectangle(cornerRadius: badgeCornerRadius))
    }

    private var imageSize: CGFloat {
        max(18, size - 4)
    }

    private var imageCornerRadius: CGFloat {
        max(5, size * 0.18)
    }

    private var fallbackIconSize: CGFloat {
        max(16, size * 0.58)
    }

    private var badgeCornerRadius: CGFloat {
        max(8, size * 0.28)
    }

    private var fallbackSystemImage: String {
        switch filter {
        case .claudeCode:
            return "sparkles"
        case .codex:
            return "chevron.left.forwardslash.chevron.right"
        case .opencode:
            return "curlybraces"
        case .pi:
            return "p.circle"
        case .hermes:
            return "h.circle"
        case .openclaw:
            return "pawprint"
        case .all:
            return "square.grid.2x2"
        }
    }
}

private struct GlobalSearchSuggestionRow: View {
    let result: AppSearchItem

    var body: some View {
        HStack(spacing: 10) {
            Image(systemName: result.kind.systemImage)
                .font(.system(size: 14, weight: .semibold))
                .foregroundStyle(Color.accentColor)
                .frame(width: 18)

            VStack(alignment: .leading, spacing: 2) {
                Text(result.title)
                    .font(.body.weight(.semibold))
                    .lineLimit(1)

                Text(result.subtitle)
                    .font(.caption)
                    .foregroundStyle(.secondary)
                    .lineLimit(1)
            }
        }
        .accessibilityElement(children: .combine)
        .accessibilityLabel("\(result.kind.title), \(result.title)")
    }
}

private struct WindowChromeTrailingControls: View {
    let layoutMode: WindowChromeLayoutMode
    @Binding var text: String
    @Binding var isSearchFocused: Bool
    @Binding var showsSearchResults: Bool
    let onSubmit: () -> Void

    var body: some View {
        controls
    }

    private var controls: some View {
        HStack(alignment: .center, spacing: 6) {
            GlobalWindowSearchControl(
                text: $text,
                isSearchFocused: $isSearchFocused,
                showsResults: $showsSearchResults,
                width: WindowChromeToolbarMetrics.searchWidth(for: layoutMode),
                onSubmit: onSubmit
            )

            WindowChromeRefreshControl()
            WindowChromeSettingsControl()
        }
        .fixedSize()
        .frame(height: WindowChromeToolbarMetrics.controlHeight, alignment: .center)
    }
}

private struct WindowChromeRefreshControl: View {
    @EnvironmentObject private var store: SkillStore

    var body: some View {
        Button {
            Task { await store.refresh() }
        } label: {
            HStack(spacing: 4) {
                if store.isRefreshBusy {
                    ProgressView()
                        .controlSize(.small)
                        .frame(width: 16, height: 16)
                } else {
                    Image(systemName: "arrow.clockwise")
                        .font(.system(size: 14, weight: .semibold))
                        .frame(width: 16, height: 16)
                }

                ZStack {
                    Image(systemName: "exclamationmark.circle.fill")
                        .font(.system(size: 11, weight: .semibold))
                        .foregroundStyle(.orange)
                        .opacity(store.hasPendingFileSystemChanges ? 1 : 0)
                        .accessibilityHidden(true)
                }
                .frame(
                    width: WindowChromeToolbarMetrics.refreshStatusSlotWidth,
                    height: 16
                )

                Text(UIStrings.menuRefresh)
                    .font(.caption.weight(.semibold))
            }
            .padding(.horizontal, WindowChromeToolbarMetrics.refreshHorizontalPadding)
            .frame(
                width: WindowChromeToolbarMetrics.refreshWidth,
                height: WindowChromeToolbarMetrics.controlHeight
            )
            .contentShape(Capsule())
        }
        .buttonStyle(.plain)
        .fixedSize()
        .windowChromeGlassCapsule()
        .disabled(store.isRefreshBusy)
        .help("\(UIStrings.menuRefresh) — \(store.watcherStatusMessage)")
        .accessibilityLabel(UIStrings.menuRefresh)
        .accessibilityValue(store.watcherStatusMessage)
    }
}

private struct GlobalWindowSearchControl: View {
    @Binding var text: String
    @Binding var isSearchFocused: Bool
    @Binding var showsResults: Bool
    let width: CGFloat
    let onSubmit: () -> Void

    private var trimmedText: String {
        text.trimmingCharacters(in: .whitespacesAndNewlines)
    }

    var body: some View {
        searchField
        .accessibilityLabel(UIStrings.text("toolbar.globalSearch", "Search all"))
        .help(UIStrings.text(
            "toolbar.globalSearch.help",
            "Search skills, sessions, and configuration across the current workspace."
        ))
    }

    @ViewBuilder
    private var searchField: some View {
        HStack(spacing: 8) {
            WindowChromeSearchTextField(
                text: $text,
                placeholder: UIStrings.text("toolbar.globalSearch", "Search all"),
                onSubmit: onSubmit
            ) { focused in
                isSearchFocused = focused
                if focused {
                    showsResults = !trimmedText.isEmpty
                } else {
                    DispatchQueue.main.asyncAfter(deadline: .now() + 0.2) {
                        if !isSearchFocused {
                            showsResults = false
                        }
                    }
                }
            }
                .frame(maxWidth: .infinity, minHeight: 22, alignment: .leading)
                .onChange(of: text) { _ in
                    showsResults = !trimmedText.isEmpty
                }

            Image(systemName: "magnifyingglass")
                .font(.system(size: 15, weight: .semibold))
                .foregroundStyle(.secondary)
                .accessibilityHidden(true)
        }
        .padding(.leading, 14)
        .padding(.trailing, 12)
        .frame(width: width, height: WindowChromeToolbarMetrics.controlHeight, alignment: .center)
        .windowChromeGlassCapsule()
        .contentShape(Capsule())
    }
}

private struct WindowChromeSearchTextField: NSViewRepresentable {
    @Binding var text: String
    let placeholder: String
    let onSubmit: () -> Void
    let onFocusChange: (Bool) -> Void

    func makeCoordinator() -> Coordinator {
        Coordinator(
            text: $text,
            onSubmit: onSubmit,
            onFocusChange: onFocusChange
        )
    }

    func makeNSView(context: Context) -> FirstMouseNSTextField {
        let textField = FirstMouseNSTextField()
        textField.delegate = context.coordinator
        textField.isEnabled = true
        textField.isEditable = true
        textField.isSelectable = true
        textField.isBordered = false
        textField.isBezeled = false
        textField.drawsBackground = false
        textField.focusRingType = .none
        textField.font = .systemFont(ofSize: NSFont.systemFontSize)
        textField.textColor = .labelColor
        textField.placeholderString = placeholder
        textField.lineBreakMode = .byTruncatingTail
        textField.cell?.usesSingleLineMode = true
        textField.cell?.wraps = false
        textField.cell?.isScrollable = true
        textField.setAccessibilityLabel(placeholder)
        return textField
    }

    func updateNSView(_ nsView: FirstMouseNSTextField, context: Context) {
        context.coordinator.text = $text
        context.coordinator.onSubmit = onSubmit
        context.coordinator.onFocusChange = onFocusChange
        nsView.placeholderString = placeholder
        nsView.setAccessibilityLabel(placeholder)
        if nsView.stringValue != text {
            nsView.stringValue = text
        }
    }

    final class Coordinator: NSObject, NSTextFieldDelegate {
        var text: Binding<String>
        var onSubmit: () -> Void
        var onFocusChange: (Bool) -> Void

        init(
            text: Binding<String>,
            onSubmit: @escaping () -> Void,
            onFocusChange: @escaping (Bool) -> Void
        ) {
            self.text = text
            self.onSubmit = onSubmit
            self.onFocusChange = onFocusChange
        }

        func controlTextDidBeginEditing(_ obj: Notification) {
            onFocusChange(true)
        }

        func controlTextDidChange(_ obj: Notification) {
            guard let textField = obj.object as? NSTextField else { return }
            text.wrappedValue = textField.stringValue
        }

        func controlTextDidEndEditing(_ obj: Notification) {
            onFocusChange(false)
        }

        func control(
            _ control: NSControl,
            textView: NSTextView,
            doCommandBy commandSelector: Selector
        ) -> Bool {
            guard commandSelector == #selector(NSResponder.insertNewline(_:)) else {
                return false
            }
            onSubmit()
            return true
        }
    }
}

private final class FirstMouseNSTextField: NSTextField {
    override var acceptsFirstResponder: Bool {
        true
    }

    override func acceptsFirstMouse(for event: NSEvent?) -> Bool {
        true
    }

    override func mouseDown(with event: NSEvent) {
        window?.makeFirstResponder(self)
        selectText(nil)
        super.mouseDown(with: event)
    }
}

private struct GlobalSearchResultsOverlay: View {
    let query: String
    let results: [AppSearchItem]
    let kindCounts: [AppSearchKindCount]
    let isLoading: Bool
    let fallbackReason: String?
    let onViewAll: (AppSearchItemKind) -> Void
    let onSelect: (AppSearchItem) -> Void

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            Text(UIStrings.text("toolbar.globalSearch.results", "Global results"))
                .font(.caption.weight(.semibold))
                .foregroundStyle(.secondary)
                .padding(.horizontal, 4)

            if isLoading && results.isEmpty {
                HStack(spacing: 8) {
                    ProgressView()
                        .controlSize(.small)
                    Text(UIStrings.text("toolbar.globalSearch.searching", "Searching..."))
                        .font(.callout)
                        .foregroundStyle(.secondary)
                }
                .padding(.horizontal, 4)
                .padding(.vertical, 8)
            } else if results.isEmpty {
                Text(UIStrings.text("toolbar.globalSearch.empty", "No matching resources."))
                    .font(.callout)
                    .foregroundStyle(.secondary)
                    .padding(.horizontal, 4)
                    .padding(.vertical, 8)
            } else {
                ScrollView {
                    LazyVStack(alignment: .leading, spacing: 10) {
                        ForEach(AppSearchItemKind.allCases, id: \.self) { kind in
                            let kindResults = results.filter { $0.kind == kind }
                            if !kindResults.isEmpty {
                                VStack(alignment: .leading, spacing: 4) {
                                    HStack(spacing: 6) {
                                        Image(systemName: kind.systemImage)
                                            .font(.caption.weight(.semibold))
                                            .foregroundStyle(.secondary)
                                            .frame(width: 14)
                                        Text("\(kind.title) \(count(for: kind))")
                                            .font(.caption.weight(.semibold))
                                            .foregroundStyle(.secondary)
                                        Spacer()
                                        Button(viewAllTitle(for: kind)) {
                                            onViewAll(kind)
                                        }
                                        .buttonStyle(.link)
                                        .controlSize(.small)
                                        .accessibilityIdentifier(viewAllAccessibilityIdentifier(for: kind))
                                        .accessibilityLabel(viewAllTitle(for: kind))
                                        .help(viewAllHelp(for: kind))
                                    }
                                    .padding(.horizontal, 8)

                                    ForEach(kindResults) { result in
                                        Button {
                                            onSelect(result)
                                        } label: {
                                            GlobalSearchSuggestionRow(result: result)
                                                .padding(.horizontal, 8)
                                                .padding(.vertical, 6)
                                                .frame(maxWidth: .infinity, alignment: .leading)
                                        }
                                        .buttonStyle(.plain)
                                        .contentShape(RoundedRectangle(cornerRadius: 8))
                                    }
                                }
                            }
                        }
                    }
                }
                .frame(
                    minHeight: WindowChromeToolbarMetrics.searchResultsMinHeight,
                    maxHeight: WindowChromeToolbarMetrics.searchResultsMaxHeight
                )
            }

            if let fallbackReason, !fallbackReason.isEmpty {
                Text(fallbackReason)
                    .font(.caption)
                    .foregroundStyle(.secondary)
                    .lineLimit(2)
                    .padding(.horizontal, 4)
            }
        }
        .padding(10)
        .frame(width: WindowChromeToolbarMetrics.searchResultsWidth, alignment: .leading)
        .background {
            RoundedRectangle(cornerRadius: CGFloat(UIOptimizationPresentation.surfaceCornerRadius))
                .fill(.regularMaterial)
        }
        .overlay {
            RoundedRectangle(cornerRadius: CGFloat(UIOptimizationPresentation.surfaceCornerRadius))
                .stroke(Color.primary.opacity(0.12), lineWidth: 1)
        }
        .shadow(color: .black.opacity(0.18), radius: 18, x: 0, y: 10)
    }

    private func count(for kind: AppSearchItemKind) -> Int {
        kindCounts.first(where: { $0.kind == kind })?.count
            ?? results.lazy.filter { $0.kind == kind }.count
    }

    private func viewAllTitle(for kind: AppSearchItemKind) -> String {
        let count = count(for: kind)
        switch kind {
        case .skill:
            return UIStrings.appSearchViewAllSkills(count)
        case .session:
            return UIStrings.appSearchViewAllSessions(count)
        case .configHistory:
            return UIStrings.appSearchViewAllConfigRecords(count)
        }
    }

    private func viewAllAccessibilityIdentifier(for kind: AppSearchItemKind) -> String {
        switch kind {
        case .skill:
            return "global-search.skills.view-all"
        case .session:
            return "global-search.sessions.view-all"
        case .configHistory:
            return "global-search.config-history.view-all"
        }
    }

    private func viewAllHelp(for kind: AppSearchItemKind) -> String {
        UIStrings.appSearchViewAllHelp(count(for: kind), kind: kind.title.lowercased())
    }
}

private struct AppStartupLoadingView: View {
    let state: AppStartupLoadingState

    var body: some View {
        VStack(spacing: 14) {
            Text(state.message)
                .font(.headline)
                .lineLimit(1)
                .minimumScaleFactor(0.8)

            ProgressView(value: state.progress)
                .progressViewStyle(.linear)
                .frame(width: 320)
        }
        .padding(32)
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .background(Color.agentCopilotWindowBackground)
        .accessibilityElement(children: .combine)
        .accessibilityLabel(state.message)
        .accessibilityValue("\(Int((state.progress * 100).rounded()))%")
    }
}

private struct ProjectTransitionLoadingView: View {
    let projectName: String

    var body: some View {
        VStack(spacing: 12) {
            ProgressView()
                .controlSize(.large)
            Text(UIStrings.projectSwitching(projectName))
                .font(.headline)
                .lineLimit(2)
                .multilineTextAlignment(.center)
        }
        .padding(28)
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .background(Color.agentCopilotWindowBackground.opacity(0.94))
        .contentShape(Rectangle())
        .accessibilityElement(children: .combine)
        .accessibilityLabel(UIStrings.projectSwitching(projectName))
    }
}

private struct WindowChromeSettingsControl: View {
    var body: some View {
        if #available(macOS 14.0, *) {
            SettingsLink {
                settingsLabel
            }
            .buttonStyle(.plain)
            .windowChromeGlassCircle()
            .help(UIStrings.text("toolbar.settings", "Settings"))
            .accessibilityLabel(UIStrings.text("toolbar.settings", "Settings"))
        } else {
            Button(action: openSettingsFallback) {
                settingsLabel
            }
            .buttonStyle(.plain)
            .windowChromeGlassCircle()
            .help(UIStrings.text("toolbar.settings", "Settings"))
            .accessibilityLabel(UIStrings.text("toolbar.settings", "Settings"))
        }
    }

    private var settingsLabel: some View {
        Image(systemName: "gearshape")
            .font(.system(size: 16, weight: .semibold))
            .frame(
                width: WindowChromeToolbarMetrics.iconButtonWidth,
                height: WindowChromeToolbarMetrics.controlHeight
            )
            .contentShape(Circle())
    }

    private func openSettingsFallback() {
        if !NSApp.sendAction(Selector(("showSettingsWindow:")), to: nil, from: nil) {
            NSApp.sendAction(Selector(("showPreferencesWindow:")), to: nil, from: nil)
        }
    }
}

private extension View {
    @ViewBuilder
    func titlebarChromeControlCapsule() -> some View {
        background(Color(nsColor: .controlBackgroundColor).opacity(0.72), in: Capsule())
            .overlay(
                Capsule()
                    .stroke(Color.secondary.opacity(0.14), lineWidth: 1)
            )
    }

    @ViewBuilder
    func titlebarChromeControlCircle() -> some View {
        background(Color(nsColor: .controlBackgroundColor).opacity(0.72), in: Circle())
            .overlay(
                Circle()
                    .stroke(Color.secondary.opacity(0.14), lineWidth: 1)
            )
    }

    @ViewBuilder
    func windowChromeGlassCapsule() -> some View {
        background(Color(nsColor: .controlBackgroundColor).opacity(0.72), in: Capsule())
            .overlay(
                Capsule()
                    .stroke(Color.secondary.opacity(0.14), lineWidth: 1)
            )
    }

    @ViewBuilder
    func windowChromeGlassCircle() -> some View {
        background(Color(nsColor: .controlBackgroundColor).opacity(0.72), in: Circle())
            .overlay(
                Circle()
                    .stroke(Color.secondary.opacity(0.14), lineWidth: 1)
            )
    }
}
