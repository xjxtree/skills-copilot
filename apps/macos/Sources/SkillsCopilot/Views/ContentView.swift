import AppKit
import SwiftUI

struct ContentView: View {
    @EnvironmentObject private var store: SkillStore
    @Environment(\.accessibilityReduceMotion) private var reduceMotion
    @State private var globalSearchText = ""
    @State private var columnVisibility: NavigationSplitViewVisibility = .all

    var body: some View {
        ZStack(alignment: .topTrailing) {
            appShell
                .opacity(store.startupLoadingState == nil ? 1 : 0)
                .allowsHitTesting(store.startupLoadingState == nil)
                .accessibilityHidden(store.startupLoadingState != nil)

            if let state = store.startupLoadingState {
                AppStartupLoadingView(state: state)
                    .transition(.opacity)
            }

            pinnedWindowChromeControls
        }
        .task {
            await store.loadAppStartupDataIfNeeded()
        }
        .transaction { transaction in
            if reduceMotion {
                transaction.animation = nil
            }
        }
        .accessibilityIdentifier(AppAccessibilityID.mainContent)
        .accessibilityLabel(UIStrings.appWindowTitle)
    }

    private var appShell: some View {
        navigationShell
    }

    private var pinnedWindowChromeControls: some View {
        WindowChromeTitlebarAccessory {
            WindowChromeToolbarControls(
                text: $globalSearchText,
                results: globalSearchResults,
                onSelect: selectGlobalSearchResult,
                onSubmit: selectFirstGlobalSearchResult
            )
            .environmentObject(store)
        }
        .frame(width: 0, height: 0)
        .allowsHitTesting(false)
        .accessibilityHidden(true)
        .zIndex(10)
    }

    private var navigationShell: some View {
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
        .task(id: store.selectedAgentLocalSessionRefreshKey) {
            guard store.hasCompletedStartupLoad else { return }
            await store.refreshSelectedAgentLocalSessionsIfNeeded()
        }
        .onChange(of: store.selectedSkillID) { _ in
            guard store.hasCompletedStartupLoad else { return }
            Task { await store.loadSelectedDetail() }
        }
    }

    private var globalSearchResults: [GlobalSearchResourceResult] {
        let query = globalSearchText.trimmingCharacters(in: .whitespacesAndNewlines).lowercased()
        guard !query.isEmpty else { return [] }

        let skillResults = store.skills.lazy
            .filter { matchesSelectedAgent($0.agent) }
            .filter { globalSearchMatches(query: query, values: skillSearchValues($0)) }
            .prefix(6)
            .map { skill in
                GlobalSearchResourceResult(
                    kind: .skill,
                    target: .skill(skill.id),
                    title: skill.name,
                    subtitle: DisplayText.scope(for: skill),
                    agent: skill.agent
                )
            }

        let sessionResults = store.localSessionPreviewResult.sessionRows.lazy
            .filter { matchesSelectedAgent($0.agent) }
            .filter { globalSearchMatches(query: query, values: sessionSearchValues($0)) }
            .prefix(4)
            .map { session in
                GlobalSearchResourceResult(
                    kind: .session,
                    target: .session(session.id),
                    title: session.title,
                    subtitle: sessionSubtitle(session),
                    agent: session.agent
                )
            }

        let configResults = store.agentConfigSnapshots.lazy
            .filter { matchesSelectedAgent($0.agent) }
            .filter { globalSearchMatches(query: query, values: configSnapshotSearchValues($0)) }
            .prefix(4)
            .map { snapshot in
                let item = AgentConfigTimelineItem(snapshot: snapshot)
                return GlobalSearchResourceResult(
                    kind: .configHistory,
                    target: .configSnapshot(snapshot.id),
                    title: item.actionText,
                    subtitle: "\(item.scopeText) · \(item.targetSummary)",
                    agent: snapshot.agent
                )
            }

        return Array((Array(skillResults) + Array(sessionResults) + Array(configResults)).prefix(12))
    }

    private func selectFirstGlobalSearchResult() {
        guard let result = globalSearchResults.first else { return }
        selectGlobalSearchResult(result)
    }

    private func selectGlobalSearchResult(_ result: GlobalSearchResourceResult) {
        switch result.target {
        case .skill(let id):
            guard let skill = store.skills.first(where: { $0.id == id }) else { return }
            if let filter = agentFilter(for: skill.agent) {
                store.agentFilter = filter
            }
            store.sidebarContentMode = .skills
            store.searchText = ""
            store.stateFilter = .all
            store.skillScopeFilter = .all
            store.selectedDetailSection = .overview
            store.selectedSidebarSelection = .skill(skill.id)
        case .session(let id):
            guard let session = store.localSessionPreviewResult.sessionRows.first(where: { $0.id == id }) else { return }
            store.sidebarContentMode = .sessions
            store.localSessionScopeFilter = .all
            store.localSessionSearchText = ""
            store.selectLocalSession(session)
        case .configSnapshot(let id):
            guard let snapshot = store.agentConfigSnapshots.first(where: { $0.id == id }) else { return }
            if let filter = agentFilter(for: snapshot.agent) {
                store.agentFilter = filter
            }
            store.sidebarContentMode = .config
            store.configScopeFilter = .all
            store.configSidebarSearchText = ""
            store.selectConfigSnapshot(snapshot)
        }
        globalSearchText = ""
    }

    private func globalSearchMatches(query: String, values: [String]) -> Bool {
        values.contains { value in
            value.lowercased().contains(query)
        }
    }

    private func matchesSelectedAgent(_ agent: String?) -> Bool {
        switch store.agentFilter {
        case .all:
            return true
        case .claudeCode, .codex, .opencode, .pi, .hermes, .openclaw:
            return agent == store.agentFilter.rawValue
        }
    }

    private func skillSearchValues(_ skill: SkillRecord) -> [String] {
        [
            skill.name,
            skill.definitionId,
            skill.path,
            skill.displayPath,
            DisplayText.agent(skill.agent),
            DisplayText.scope(for: skill)
        ]
    }

    private func sessionSearchValues(_ session: LocalSessionPreviewRow) -> [String] {
        [
            session.title,
            session.redactedPath,
            session.projectRoot ?? "",
            session.excerpt,
            DisplayText.agent(session.agent ?? ""),
            DisplayText.scope(session.scope)
        ]
    }

    private func configSnapshotSearchValues(_ snapshot: ConfigSnapshotRecord) -> [String] {
        [
            snapshot.reason,
            snapshot.target,
            DisplayText.agent(snapshot.agent),
            DisplayText.scope(snapshot.scope),
            DisplayText.timestamp(snapshot.createdAt)
        ]
    }

    private func sessionSubtitle(_ session: LocalSessionPreviewRow) -> String {
        var parts: [String] = []
        parts.append(DisplayText.scope(session.scope))
        if let endedAt = session.endedAt ?? session.startedAt {
            parts.append(DisplayText.timestamp(endedAt))
        }
        return parts.joined(separator: " · ")
    }

    private func agentFilter(for agent: String?) -> SkillAgentFilter? {
        guard let agent else { return nil }
        return SkillAgentFilter.managementCases.first { $0.rawValue == agent }
    }
}

private struct WindowChromeTitlebarAccessory<Content: View>: NSViewRepresentable {
    let content: Content

    init(@ViewBuilder content: () -> Content) {
        self.content = content()
    }

    func makeCoordinator() -> Coordinator {
        Coordinator()
    }

    func makeNSView(context: Context) -> NSView {
        let view = NSView(frame: .zero)
        DispatchQueue.main.async {
            context.coordinator.installIfNeeded(in: view.window, content: content)
        }
        return view
    }

    func updateNSView(_ nsView: NSView, context: Context) {
        context.coordinator.update(content: content)
        DispatchQueue.main.async {
            context.coordinator.installIfNeeded(in: nsView.window, content: content)
        }
    }

    final class Coordinator {
        private weak var window: NSWindow?
        private var accessory: NSTitlebarAccessoryViewController?
        private var hostingView: FirstMouseNSHostingView<Content>?

        deinit {
            removeAccessory()
        }

        func installIfNeeded(in window: NSWindow?, content: Content) {
            guard let window else { return }
            guard self.window !== window else {
                update(content: content)
                return
            }

            removeAccessory()

            let hostingView = FirstMouseNSHostingView(rootView: content)
            hostingView.translatesAutoresizingMaskIntoConstraints = false
            hostingView.setContentHuggingPriority(.required, for: .horizontal)
            hostingView.setContentHuggingPriority(.required, for: .vertical)
            hostingView.setContentCompressionResistancePriority(.required, for: .horizontal)
            hostingView.setContentCompressionResistancePriority(.required, for: .vertical)

            let container = FirstMouseTitlebarAccessoryContainer(
                frame: NSRect(
                    x: 0,
                    y: 0,
                    width: WindowChromeToolbarMetrics.accessoryWidth,
                    height: WindowChromeToolbarMetrics.controlHeight
                )
            )
            container.addSubview(hostingView)

            NSLayoutConstraint.activate([
                container.widthAnchor.constraint(equalToConstant: WindowChromeToolbarMetrics.accessoryWidth),
                container.heightAnchor.constraint(equalToConstant: WindowChromeToolbarMetrics.controlHeight),
                hostingView.widthAnchor.constraint(equalToConstant: WindowChromeToolbarMetrics.totalWidth),
                hostingView.heightAnchor.constraint(equalToConstant: WindowChromeToolbarMetrics.controlHeight),
                hostingView.trailingAnchor.constraint(
                    equalTo: container.trailingAnchor,
                    constant: -WindowChromeToolbarMetrics.titlebarTrailingPadding
                ),
                hostingView.centerYAnchor.constraint(equalTo: container.centerYAnchor)
            ])

            let accessory = NSTitlebarAccessoryViewController()
            accessory.view = container
            accessory.layoutAttribute = .right
            window.addTitlebarAccessoryViewController(accessory)

            self.window = window
            self.accessory = accessory
            self.hostingView = hostingView
        }

        func update(content: Content) {
            hostingView?.rootView = content
        }

        private func removeAccessory() {
            if let accessory,
               let window,
               let index = window.titlebarAccessoryViewControllers.firstIndex(where: { $0 === accessory }) {
                window.removeTitlebarAccessoryViewController(at: index)
            }
            accessory = nil
            hostingView = nil
            window = nil
        }
    }
}

private final class FirstMouseTitlebarAccessoryContainer: NSView {
    override var intrinsicContentSize: NSSize {
        NSSize(
            width: WindowChromeToolbarMetrics.accessoryWidth,
            height: WindowChromeToolbarMetrics.controlHeight
        )
    }

    override func acceptsFirstMouse(for event: NSEvent?) -> Bool {
        true
    }
}

private final class FirstMouseNSHostingView<Content: View>: NSHostingView<Content> {
    override func acceptsFirstMouse(for event: NSEvent?) -> Bool {
        true
    }
}

private enum WindowChromeToolbarMetrics {
    static let controlHeight: CGFloat = 32
    static let agentWidth: CGFloat = 146
    static let projectWidth: CGFloat = 210
    static let toolbarSpacing: CGFloat = 8
    static let trailingSpacing: CGFloat = 6
    static let iconButtonWidth: CGFloat = 30
    static let titlebarTrailingPadding: CGFloat = 28
    static let searchWidth = CGFloat(UIOptimizationPresentation.unifiedToolbar.idealGlobalSearchWidth)

    static var trailingWidth: CGFloat {
        searchWidth + iconButtonWidth * 2 + trailingSpacing * 2
    }

    static var totalWidth: CGFloat {
        agentWidth + projectWidth + trailingWidth + toolbarSpacing * 2
    }

    static var accessoryWidth: CGFloat {
        totalWidth + titlebarTrailingPadding
    }
}

private struct WindowChromeToolbarControls: View {
    @Binding var text: String
    let results: [GlobalSearchResourceResult]
    let onSelect: (GlobalSearchResourceResult) -> Void
    let onSubmit: () -> Void

    var body: some View {
        HStack(spacing: 8) {
            TitlebarAgentSelectorControl()
                .frame(width: agentWidth, height: controlHeight, alignment: .leading)

            TitlebarProjectPickerControl(isCompact: false)
                .frame(width: projectWidth, height: controlHeight, alignment: .leading)

            WindowChromeTrailingControls(
                text: $text,
                results: results,
                onSelect: onSelect,
                onSubmit: onSubmit
            )
        }
        .frame(height: controlHeight, alignment: .leading)
        .fixedSize(horizontal: true, vertical: false)
    }

    private var controlHeight: CGFloat { WindowChromeToolbarMetrics.controlHeight }
    private var agentWidth: CGFloat { WindowChromeToolbarMetrics.agentWidth }
    private var projectWidth: CGFloat { WindowChromeToolbarMetrics.projectWidth }
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
            return UIStrings.text("agent.short.claudeCode", "Claude")
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
                    Text(UIStrings.recentProjects)
                        .font(.caption.weight(.semibold))
                        .foregroundStyle(.secondary)
                        .padding(.horizontal, 8)

                    ForEach(store.recentProjectContexts) { context in
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
                            Text(context.name)
                                .lineLimit(1)
                                .truncationMode(.middle)
                                .frame(maxWidth: .infinity, alignment: .leading)
                        }
                        .buttonStyle(.plain)
                        .padding(.horizontal, 8)
                        .padding(.vertical, 6)
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
            .frame(width: 260, alignment: .leading)
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

private enum GlobalSearchResourceKind: String, CaseIterable {
    case skill
    case session
    case configHistory

    var title: String {
        switch self {
        case .skill:
            return UIStrings.skills
        case .session:
            return UIStrings.text("sidebar.mode.sessions", "Sessions")
        case .configHistory:
            return UIStrings.agentConfigHistory
        }
    }

    var systemImage: String {
        switch self {
        case .skill:
            return "square.stack.3d.up"
        case .session:
            return "bubble.left.and.text.bubble.right"
        case .configHistory:
            return "clock.arrow.circlepath"
        }
    }
}

private enum GlobalSearchResourceTarget: Hashable {
    case skill(String)
    case session(String)
    case configSnapshot(String)
}

private struct GlobalSearchResourceResult: Identifiable, Hashable {
    let kind: GlobalSearchResourceKind
    let target: GlobalSearchResourceTarget
    let title: String
    let subtitle: String
    let agent: String?

    var id: String {
        switch target {
        case .skill(let id):
            return "\(kind.rawValue):\(id)"
        case .session(let id):
            return "\(kind.rawValue):\(id)"
        case .configSnapshot(let id):
            return "\(kind.rawValue):\(id)"
        }
    }
}

private struct GlobalSearchSuggestionRow: View {
    let result: GlobalSearchResourceResult

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
    @Binding var text: String
    let results: [GlobalSearchResourceResult]
    let onSelect: (GlobalSearchResourceResult) -> Void
    let onSubmit: () -> Void

    private let searchWidth = WindowChromeToolbarMetrics.searchWidth

    var body: some View {
        controls
    }

    private var controls: some View {
        HStack(alignment: .center, spacing: 6) {
            GlobalWindowSearchControl(
                text: $text,
                results: results,
                width: searchWidth,
                onSelect: onSelect,
                onSubmit: onSubmit
            )

            WindowChromeAboutButton()
            WindowChromeSettingsControl()
        }
        .fixedSize()
        .frame(height: 32, alignment: .center)
    }
}

private struct GlobalWindowSearchControl: View {
    @Binding var text: String
    let results: [GlobalSearchResourceResult]
    let width: CGFloat
    let onSelect: (GlobalSearchResourceResult) -> Void
    let onSubmit: () -> Void
    @State private var showsResults = false
    @State private var isSearchFocused = false

    private var trimmedText: String {
        text.trimmingCharacters(in: .whitespacesAndNewlines)
    }

    var body: some View {
        searchField
        .popover(isPresented: resultsPopoverBinding, arrowEdge: .top) {
            GlobalSearchResultsPopover(
                query: trimmedText,
                results: results
            ) { result in
                showsResults = false
                isSearchFocused = false
                onSelect(result)
            }
        }
        .accessibilityLabel(UIStrings.text("toolbar.globalSearch", "Search all"))
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
                    DispatchQueue.main.asyncAfter(deadline: .now() + 0.18) {
                        if !isSearchFocused {
                            showsResults = false
                        }
                    }
                }
            }
                .frame(maxWidth: .infinity, minHeight: 22, alignment: .leading)
                .onChange(of: text) { _ in
                    showsResults = isSearchFocused && !trimmedText.isEmpty
                }

            Image(systemName: "magnifyingglass")
                .font(.system(size: 15, weight: .semibold))
                .foregroundStyle(.secondary)
                .accessibilityHidden(true)
        }
        .padding(.leading, 14)
        .padding(.trailing, 12)
        .frame(width: width, height: 30, alignment: .center)
        .windowChromeGlassCapsule()
        .contentShape(Capsule())
    }

    private var resultsPopoverBinding: Binding<Bool> {
        Binding {
            showsResults && !trimmedText.isEmpty
        } set: { isPresented in
            if !isPresented {
                showsResults = false
            }
        }
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

private struct WindowChromeAboutButton: View {
    var body: some View {
        Button {
            NSApp.orderFrontStandardAboutPanel(nil)
        } label: {
            Image(systemName: "questionmark.circle")
                .font(.system(size: 16, weight: .semibold))
                .frame(width: 30, height: 30)
                .contentShape(Circle())
        }
        .buttonStyle(.plain)
        .windowChromeGlassCircle()
        .help(UIStrings.text("toolbar.help", "Help"))
        .accessibilityLabel(UIStrings.text("toolbar.help", "Help"))
    }
}

private struct GlobalSearchResultsPopover: View {
    let query: String
    let results: [GlobalSearchResourceResult]
    let onSelect: (GlobalSearchResourceResult) -> Void

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            Text(UIStrings.text("toolbar.globalSearch.results", "Global results"))
                .font(.caption.weight(.semibold))
                .foregroundStyle(.secondary)
                .padding(.horizontal, 4)

            if results.isEmpty {
                Text(UIStrings.text("toolbar.globalSearch.empty", "No matching resources."))
                    .font(.callout)
                    .foregroundStyle(.secondary)
                    .padding(.horizontal, 4)
                    .padding(.vertical, 8)
            } else {
                ScrollView {
                    LazyVStack(alignment: .leading, spacing: 10) {
                        ForEach(GlobalSearchResourceKind.allCases, id: \.self) { kind in
                            let kindResults = results.filter { $0.kind == kind }
                            if !kindResults.isEmpty {
                                VStack(alignment: .leading, spacing: 4) {
                                    HStack(spacing: 6) {
                                        Image(systemName: kind.systemImage)
                                            .font(.caption.weight(.semibold))
                                            .foregroundStyle(.secondary)
                                            .frame(width: 14)
                                        Text("\(kind.title) \(kindResults.count)")
                                            .font(.caption.weight(.semibold))
                                            .foregroundStyle(.secondary)
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
                .frame(maxHeight: 320)
            }
        }
        .padding(10)
        .frame(width: 360, alignment: .leading)
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
            .frame(width: 30, height: 30)
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
