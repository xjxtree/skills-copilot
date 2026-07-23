import AppKit
import SwiftUI

struct SidebarView: View {
    @EnvironmentObject private var store: SkillStore

    var body: some View {
        List(selection: routeSelection) {
            Section(UIStrings.text("sidebar.primaryNavigation", "Navigate")) {
                PrimarySidebarRow(
                    title: UIStrings.text(
                        "sidebar.route.projectOverview",
                        "Project Overview"
                    ),
                    subtitle: UIStrings.text(
                        "sidebar.route.projectOverview.subtitle",
                        "Status, readiness, and recent work"
                    ),
                    systemImage: "rectangle.3.group"
                )
                .tag(AppRoute.overview)

                PrimarySidebarRow(
                    title: UIStrings.text("sidebar.route.skills", "Skills"),
                    subtitle: UIStrings.text(
                        "sidebar.route.skills.subtitle",
                        "Capabilities and attention"
                    ),
                    systemImage: "square.stack.3d.up"
                )
                .tag(AppRoute.skills)

                PrimarySidebarRow(
                    title: UIStrings.text("sidebar.route.sessions", "Sessions"),
                    subtitle: UIStrings.text(
                        "sidebar.route.sessions.subtitle",
                        "Find and continue local work"
                    ),
                    systemImage: "bubble.left.and.text.bubble.right"
                )
                .tag(AppRoute.sessions)
            }
        }
        .listStyle(.sidebar)
        .navigationTitle("")
    }

    private var routeSelection: Binding<AppRoute?> {
        Binding(
            get: {
                switch store.appRoute {
                case .overview, .skills, .sessions:
                    return store.appRoute
                case .advanced:
                    return nil
                }
            },
            set: { route in
                guard let route else { return }
                store.selectAppRoute(route)
            }
        )
    }
}

private struct PrimarySidebarRow: View {
    let title: String
    let subtitle: String?
    let systemImage: String

    var body: some View {
        HStack(alignment: .center, spacing: 10) {
            Image(systemName: systemImage)
                .font(.body.weight(.medium))
                .symbolRenderingMode(.hierarchical)
                .frame(width: 20)

            VStack(alignment: .leading, spacing: 1) {
                Text(title)
                    .font(.body)
                    .lineLimit(1)
                if let subtitle {
                    Text(subtitle)
                        .font(.caption)
                        .foregroundStyle(.secondary)
                        .lineLimit(1)
                }
            }
        }
        .padding(.vertical, 3)
        .accessibilityElement(children: .combine)
    }
}

struct SecondarySidebarView: View {
    @EnvironmentObject private var store: SkillStore
    let columnVisibility: NavigationSplitViewVisibility
    @State private var isBatchOperationPresented = false

    var body: some View {
        ScrollViewReader { proxy in
            List(selection: $store.selectedSidebarSelection) {
                switch store.sidebarContentMode {
                case .sessions:
                    SessionSidebarPanel()
                case .skills:
                    SkillSidebarPanel(isBatchOperationPresented: $isBatchOperationPresented)
                case .config:
                    ConfigSidebarPanel()
                }
            }
            .listStyle(.plain)
            .scrollContentBackground(.hidden)
            .padding(.top, 50)
            .ignoresSafeArea(.container, edges: .top)
            .secondarySidebarPaneBackground()
            .background {
                GeometryReader { proxy in
                    Color.clear
                        .preference(
                            key: SecondarySidebarHeaderWidthPreferenceKey.self,
                            value: proxy.size.width
                        )
                }
                .allowsHitTesting(false)
            }
            .navigationTitle("")
            .onChange(of: store.skillListScrollRequest) { request in
                guard let request else { return }
                DispatchQueue.main.async {
                    withAnimation(.easeInOut(duration: 0.18)) {
                        proxy.scrollTo(request.skillID, anchor: .center)
                    }
                }
            }
        }
        .sheet(isPresented: $isBatchOperationPresented) {
            BatchSkillOperationSheet()
                .environmentObject(store)
        }
    }
}

struct SecondarySidebarHeaderWidthPreferenceKey: PreferenceKey {
    static var defaultValue = CGFloat(UIOptimizationPresentation.skillList.minimumSecondaryColumnWidth)

    static func reduce(value: inout CGFloat, nextValue: () -> CGFloat) {
        value = nextValue()
    }
}

struct SecondarySidebarHeaderChrome: View {
    let columnVisibility: NavigationSplitViewVisibility
    let availableWidth: CGFloat

    var body: some View {
        let agentLeading = agentLeadingInset(for: availableWidth)
        let projectLeading = projectLeadingInset(for: availableWidth, agentLeading: agentLeading)
        let agentFrame = CGRect(
            x: agentLeading,
            y: topInset,
            width: agentWidth,
            height: controlHeight
        )
        let projectFrame = CGRect(
            x: projectLeading,
            y: topInset,
            width: projectWidth,
            height: controlHeight
        )

        ZStack(alignment: .topLeading) {
            SecondarySidebarAgentHeaderControl()
                .frame(width: agentWidth, height: controlHeight, alignment: .leading)
                .offset(x: agentLeading, y: topInset)

            SecondarySidebarProjectHeaderControl(isCompact: isPrimarySidebarCollapsed)
                .frame(width: projectWidth, height: controlHeight, alignment: .trailing)
                .offset(x: projectLeading, y: topInset)
        }
        .frame(width: availableWidth, height: topInset + controlHeight, alignment: .topLeading)
        .contentShape(
            SecondarySidebarHeaderHitShape(
                agentFrame: agentFrame,
                projectFrame: projectFrame
            )
        )
        .animation(.snappy(duration: 0.22), value: isPrimarySidebarCollapsed)
        .frame(height: topInset + controlHeight)
        .ignoresSafeArea(.container, edges: .top)
    }

    private var isPrimarySidebarCollapsed: Bool {
        columnVisibility != .all
    }

    private var topInset: CGFloat { 8 }
    private var controlHeight: CGFloat { 36 }
    private var agentWidth: CGFloat { isPrimarySidebarCollapsed ? 126 : 158 }
    private var projectWidth: CGFloat { isPrimarySidebarCollapsed ? 36 : 152 }
    private var trailingInset: CGFloat { 24 }
    private var expandedLeadingInset: CGFloat {
        CGFloat(UIOptimizationPresentation.listPage.cardHorizontalInset)
    }
    private var collapsedAgentLeadingInset: CGFloat { 152 }

    private func agentLeadingInset(for availableWidth: CGFloat) -> CGFloat {
        if isPrimarySidebarCollapsed {
            let maxLeading = max(expandedLeadingInset, availableWidth - trailingInset - projectWidth - 8 - agentWidth)
            return min(collapsedAgentLeadingInset, maxLeading)
        }
        return expandedLeadingInset
    }

    private func projectLeadingInset(for availableWidth: CGFloat, agentLeading: CGFloat) -> CGFloat {
        let preferred = availableWidth - trailingInset - projectWidth
        let minimum = agentLeading + agentWidth + 8
        return max(minimum, preferred)
    }
}

private struct SecondarySidebarHeaderHitShape: Shape {
    let agentFrame: CGRect
    let projectFrame: CGRect

    func path(in rect: CGRect) -> Path {
        var path = Path()
        path.addRoundedRect(in: agentFrame, cornerSize: CGSize(width: 18, height: 18))
        path.addRoundedRect(in: projectFrame, cornerSize: CGSize(width: 18, height: 18))
        return path
    }
}

struct SecondarySidebarAgentHeaderControl: View {
    var body: some View {
        SecondarySidebarAgentSelectorMenu()
            .frame(minWidth: 126, idealWidth: 148, maxWidth: 158, alignment: .leading)
    }
}

struct SecondarySidebarProjectHeaderControl: View {
    let isCompact: Bool

    var body: some View {
        SecondarySidebarProjectPickerMenu(isCompact: isCompact)
            .frame(
                minWidth: isCompact ? 36 : 42,
                idealWidth: isCompact ? 36 : 140,
                maxWidth: isCompact ? 36 : 152,
                alignment: .trailing
            )
    }
}

private struct SecondarySidebarAgentSelectorMenu: View {
    @EnvironmentObject private var store: SkillStore

    var body: some View {
        Menu {
            ForEach(SkillAgentFilter.managementCases) { filter in
                Button {
                    store.agentFilter = filter
                } label: {
                    agentMenuItemLabel(for: filter)
                }
            }
        } label: {
            SecondarySidebarAgentSelectorLabel(
                filter: store.agentFilter,
                title: shortTitle(for: store.agentFilter)
            )
            .accessibilityHidden(true)
        }
        .menuStyle(.button)
        .buttonStyle(.plain)
        .help("\(UIStrings.text("help.agentSelector", "Select the agent workspace.")) \(store.agentFilter.title)")
        .accessibilityLabel(UIStrings.agent)
        .accessibilityValue(store.agentFilter.title)
    }

    @ViewBuilder
    private func agentMenuItemLabel(for filter: SkillAgentFilter) -> some View {
        if filter == store.agentFilter {
            Label(shortTitle(for: filter), systemImage: "checkmark")
        } else {
            Label(shortTitle(for: filter), systemImage: systemImage(for: filter))
        }
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

private struct SecondarySidebarAgentSelectorLabel: View {
    let filter: SkillAgentFilter
    let title: String

    var body: some View {
        HStack(spacing: 8) {
            AgentIconBadge(filter: filter, size: 24)

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
        .frame(minWidth: 126, maxWidth: 158, minHeight: 36, maxHeight: 36, alignment: .leading)
        .secondarySidebarHeaderControlCapsule()
        .contentShape(Capsule())
        .accessibilityElement(children: .combine)
        .accessibilityLabel(title)
    }
}

private struct SecondarySidebarProjectPickerMenu: View {
    @EnvironmentObject private var store: SkillStore
    let isCompact: Bool

    var body: some View {
        Menu {
            Button {
                chooseProject()
            } label: {
                Label(UIStrings.chooseProject, systemImage: "folder.badge.plus")
            }

            if !store.recentProjectContexts.isEmpty {
                Divider()

                Section(UIStrings.recentProjects) {
                    ForEach(store.recentProjectContexts) { context in
                        Button {
                            Task {
                                await store.setProject(
                                    rootPath: context.rootPath,
                                    currentCWD: context.currentCWD,
                                    name: context.name
                                )
                            }
                        } label: {
                            Text(recentProjectTitle(context))
                        }
                    }
                }

                Menu {
                    ForEach(store.recentProjectContexts) { context in
                        Button(role: .destructive) {
                            Task { await store.removeRecentProject(id: context.id) }
                        } label: {
                            Label(recentProjectTitle(context), systemImage: "trash")
                        }
                    }

                    Divider()

                    Button(role: .destructive) {
                        Task { await store.previewClearRecentProjects() }
                    } label: {
                        Label(UIStrings.clearRecentProjects, systemImage: "trash.slash")
                    }
                } label: {
                    Label(UIStrings.manageRecentProjects, systemImage: "clock.arrow.circlepath")
                }
            }

            if store.activeProjectContext != nil {
                Divider()

                Button {
                    revealActiveProject()
                } label: {
                    Label(UIStrings.revealInFinder, systemImage: "arrow.up.forward.app")
                }

                Button(role: .destructive) {
                    Task { await store.previewClearProject() }
                } label: {
                    Label(UIStrings.clearProject, systemImage: "xmark.circle")
                }
            }
        } label: {
            SecondarySidebarProjectPickerLabel(
                title: projectTitle,
                systemImage: statusImage ?? "folder.badge.plus",
                isWarning: statusImage == "exclamationmark.triangle.fill",
                isCompact: isCompact
            )
            .accessibilityHidden(true)
        }
        .menuStyle(.button)
        .buttonStyle(.plain)
        .disabled(store.isRefreshBusy)
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

    private func recentProjectTitle(_ context: ProjectContext) -> String {
        UIStrings.recentProjectItem(
            context.name,
            path: DisplayText.privacyPath(context.rootPath, privacyModeEnabled: true)
        )
    }
}

private struct SecondarySidebarProjectPickerLabel: View {
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
        .frame(minWidth: 120, maxWidth: 152, minHeight: 36, maxHeight: 36, alignment: .leading)
        .secondarySidebarHeaderControlCapsule()
        .contentShape(Capsule())
        .accessibilityElement(children: .combine)
        .accessibilityLabel(title)
    }

    private var collapsedLabel: some View {
        icon
            .frame(width: 36, height: 36)
            .secondarySidebarHeaderControlCircle()
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

struct SkillPackageManagerSheet: View {
    @EnvironmentObject private var store: SkillStore
    let entryContext: SkillManagerEntryContext

    init(entryContext: SkillManagerEntryContext = .default) {
        self.entryContext = entryContext
    }

    var body: some View {
        WorkflowSheetShell(
            title: UIStrings.text("skillManager.title", "Skill Package Manager"),
            systemImage: "shippingbox.and.arrow.backward",
            subtitle: UIStrings.text("skillManager.workflow.label", "Workflow"),
            content: {
                SkillManagerPanel(
                    showsHeader: false,
                    entryContext: entryContext
                )
            }
        )
        .frame(
            minWidth: CGFloat(UIOptimizationPresentation.skillManager.sheetMinimumWidth),
            idealWidth: CGFloat(UIOptimizationPresentation.skillManager.sheetIdealWidth),
            minHeight: CGFloat(UIOptimizationPresentation.skillManager.sheetMinimumHeight),
            idealHeight: CGFloat(UIOptimizationPresentation.skillManager.sheetIdealHeight)
        )
        .onDisappear {
            store.clearSkillManagerWorkflowPreviews()
        }
    }
}

private struct SecondarySidebarPaneBackground: ViewModifier {
    func body(content: Content) -> some View {
        content
            .background {
                Rectangle()
                    .fill(Color.agentCopilotPanelBackground)
                    .ignoresSafeArea()
            }
    }
}

private extension View {
    func secondarySidebarPaneBackground() -> some View {
        modifier(SecondarySidebarPaneBackground())
    }

    @ViewBuilder
    func secondarySidebarHeaderControlCapsule() -> some View {
#if compiler(>=6.2)
        if #available(macOS 26.0, *) {
            glassEffect(.regular.interactive(), in: Capsule())
        } else {
            secondarySidebarHeaderControlFallback(shape: Capsule())
        }
#else
        secondarySidebarHeaderControlFallback(shape: Capsule())
#endif
    }

    @ViewBuilder
    func secondarySidebarHeaderControlCircle() -> some View {
#if compiler(>=6.2)
        if #available(macOS 26.0, *) {
            glassEffect(.regular.interactive(), in: Circle())
        } else {
            secondarySidebarHeaderControlFallback(shape: Circle())
        }
#else
        secondarySidebarHeaderControlFallback(shape: Circle())
#endif
    }

    func secondarySidebarHeaderControlFallback<S: Shape>(shape: S) -> some View {
        background(Color.agentCopilotPanelBackground, in: shape)
            .overlay(
                shape
                    .stroke(Color.secondary.opacity(0.12), lineWidth: 1)
            )
    }

    func listPageChromeRow() -> some View {
        self
            .listRowInsets(EdgeInsets(top: 0, leading: 0, bottom: 4, trailing: 0))
            .listRowSeparator(.hidden)
            .listRowBackground(Color.clear)
    }

    func listPageCardRow() -> some View {
        self
            .listRowInsets(
                EdgeInsets(
                    top: CGFloat(UIOptimizationPresentation.listPage.cardRowSpacing) / 2,
                    leading: CGFloat(UIOptimizationPresentation.listPage.cardHorizontalInset),
                    bottom: CGFloat(UIOptimizationPresentation.listPage.cardRowSpacing) / 2,
                    trailing: CGFloat(UIOptimizationPresentation.listPage.cardHorizontalInset)
                )
            )
            .listRowSeparator(.hidden)
            .listRowBackground(Color.clear)
    }
}

private struct SecondarySidebarTitleBar: View {
    @EnvironmentObject private var store: SkillStore

    var body: some View {
        HStack(spacing: 9) {
            AgentIconBadge(filter: store.agentFilter)
                .fixedSize()

            Text(title)
                .font(.title3.weight(.semibold))
                .foregroundStyle(.primary)
                .lineLimit(1)
                .minimumScaleFactor(0.82)
                .allowsTightening(true)
                .layoutPriority(1)

            Spacer(minLength: 0)
        }
        .padding(.horizontal, 22)
        .padding(.top, 8)
        .padding(.bottom, 12)
        .frame(maxWidth: .infinity, alignment: .leading)
        .accessibilityElement(children: .combine)
        .accessibilityLabel(title)
    }

    private var title: String {
        "\(store.agentFilter.title) \(store.sidebarContentMode.title)"
    }
}

private struct ListPageTitleBlock: View {
    let title: String
    let subtitle: String
    let countText: String?

    var body: some View {
        HStack(alignment: .firstTextBaseline, spacing: 10) {
            VStack(alignment: .leading, spacing: 3) {
                Text(title)
                    .font(.title3.weight(.semibold))
                    .foregroundStyle(.primary)
                    .lineLimit(1)
                    .minimumScaleFactor(0.82)

                Text(subtitle)
                    .font(.caption)
                    .foregroundStyle(.secondary)
                    .lineLimit(1)
                    .minimumScaleFactor(0.75)
            }
            .layoutPriority(1)

            Spacer(minLength: 8)

            if let countText {
                Text(countText)
                    .font(.caption.bold().monospacedDigit())
                    .foregroundStyle(.secondary)
                    .lineLimit(1)
                    .padding(.horizontal, 8)
                    .padding(.vertical, 4)
                    .background(Color.agentCopilotPanelBackground, in: Capsule())
            }
        }
        .accessibilityElement(children: .combine)
        .accessibilityLabel("\(title), \(subtitle)")
    }
}

private struct SidebarNavigationMetric: Identifiable {
    let title: String
    let value: String
    var tone: SidebarNavigationMetricTone = .neutral

    var id: String { "\(title)-\(value)-\(tone)" }
}

private enum SidebarNavigationMetricTone: Hashable {
    case neutral
    case muted
    case info
    case positive
    case warning
    case danger

    var valueColor: Color {
        switch self {
        case .neutral:
            return .primary
        case .muted:
            return .secondary
        case .info:
            return .blue
        case .positive:
            return .green
        case .warning:
            return .orange
        case .danger:
            return .red
        }
    }

    var selectedValueColor: Color {
        switch self {
        case .neutral, .muted:
            return .white.opacity(0.9)
        case .info:
            return .cyan
        case .positive:
            return .green
        case .warning:
            return .orange
        case .danger:
            return .red
        }
    }
}

private struct SidebarFooterToolRow: View {
    let isSkillManagerPresented: Bool
    let onOpenSkillManager: () -> Void
    let onOpenPreflight: () -> Void

    var body: some View {
        VStack(spacing: 8) {
            SidebarFooterToolButton(
                title: UIStrings.text("skillManager.title", "Skill Package Manager"),
                subtitle: UIStrings.text("skillManager.sidebar.subtitle", "Search, install, local library"),
                systemImage: "shippingbox.and.arrow.backward",
                accent: .accentColor,
                badge: UIStrings.text("sidebar.skillManager.metric.global", "Global"),
                isSelected: isSkillManagerPresented,
                action: onOpenSkillManager
            )

            SidebarFooterToolButton(
                title: UIStrings.taskCockpitTitle,
                subtitle: UIStrings.text("sidebar.preflight.subtitle", "Read-only task check"),
                systemImage: "checklist",
                accent: .accentColor,
                badge: UIStrings.text("sidebar.preflight.metric.readOnly", "Read-only"),
                action: onOpenPreflight
            )
        }
    }
}

private struct SidebarFooterToolButton: View {
    let title: String
    let subtitle: String
    let systemImage: String
    let accent: Color
    let badge: String
    var isSelected = false
    let action: () -> Void

    var body: some View {
        Button(action: action) {
            HStack(alignment: .center, spacing: 7) {
                Image(systemName: systemImage)
                    .font(.caption.weight(.semibold))
                    .foregroundStyle(iconColor)
                    .frame(width: 22, height: 22)
                    .background(iconBackground, in: RoundedRectangle(cornerRadius: 6))

                VStack(alignment: .leading, spacing: 1) {
                    Text(title)
                        .font(.caption.weight(.semibold))
                        .foregroundStyle(primaryTextColor)
                        .lineLimit(1)
                    Text(subtitle)
                        .font(.caption2)
                        .foregroundStyle(secondaryTextColor)
                        .lineLimit(1)
                }
                .frame(maxWidth: .infinity, alignment: .leading)
                .layoutPriority(1)

                Text(badge)
                    .font(.caption2.weight(.medium))
                    .foregroundStyle(badgeTextColor)
                    .lineLimit(1)
                    .padding(.horizontal, 5)
                    .padding(.vertical, 2)
                    .background(badgeBackground, in: Capsule())
                    .fixedSize(horizontal: true, vertical: false)
            }
            .padding(.horizontal, 8)
            .padding(.vertical, 6)
            .frame(maxWidth: .infinity, minHeight: 46, alignment: .leading)
            .background(buttonBackground, in: RoundedRectangle(cornerRadius: 8))
            .overlay(
                RoundedRectangle(cornerRadius: 8)
                    .stroke(borderColor, lineWidth: 1)
            )
            .contentShape(RoundedRectangle(cornerRadius: 8))
        }
        .buttonStyle(.plain)
        .accessibilityLabel(title)
    }

    private var iconColor: Color {
        .primary
    }

    private var iconBackground: Color {
        isSelected ? Color.primary.opacity(0.12) : Color.secondary.opacity(0.10)
    }

    private var primaryTextColor: Color {
        .primary
    }

    private var secondaryTextColor: Color {
        .secondary
    }

    private var badgeTextColor: Color {
        .primary
    }

    private var badgeBackground: Color {
        Color.agentCopilotPanelBackground
    }

    private var buttonBackground: Color {
        isSelected ? Color(nsColor: .selectedContentBackgroundColor).opacity(0.14) : Color.agentCopilotPanelBackground
    }

    private var borderColor: Color {
        isSelected ? accent.opacity(0.38) : Color.secondary.opacity(0.14)
    }
}

private struct SidebarNavigationCardButton: View {
    let title: String
    let subtitle: String
    let systemImage: String
    let count: String?
    let metrics: [SidebarNavigationMetric]
    let isSelected: Bool
    let action: () -> Void

    var body: some View {
        Button(action: action) {
            VStack(alignment: .leading, spacing: 8) {
                HStack(spacing: 10) {
                    Image(systemName: systemImage)
                        .font(.title3.weight(.semibold))
                        .foregroundStyle(iconColor)
                        .frame(width: 30, height: 30)
                        .background(iconBackground, in: RoundedRectangle(cornerRadius: 8))

                    VStack(alignment: .leading, spacing: 2) {
                        Text(title)
                            .font(.headline)
                            .foregroundStyle(primaryTextColor)
                            .lineLimit(1)
                        Text(subtitle)
                            .font(.caption)
                            .foregroundStyle(secondaryTextColor)
                            .lineLimit(1)
                    }
                    .layoutPriority(1)

                    Spacer(minLength: 8)

                    if let count {
                        Text(count)
                            .font(.caption.bold().monospacedDigit())
                            .foregroundStyle(secondaryTextColor)
                            .lineLimit(1)
                    }
                }

                if !metrics.isEmpty {
                    HStack(spacing: 5) {
                        ForEach(metrics) { metric in
                            SidebarNavigationMetricPill(
                                metric: metric,
                                isSelected: isSelected
                            )
                        }
                    }
                    .frame(maxWidth: .infinity, alignment: .leading)
                }
            }
            .padding(.horizontal, 10)
            .padding(.vertical, 10)
            .frame(maxWidth: .infinity, alignment: .leading)
            .background(background, in: RoundedRectangle(cornerRadius: 9))
            .overlay(
                ZStack(alignment: .leading) {
                    RoundedRectangle(cornerRadius: 9)
                        .stroke(borderColor, lineWidth: 1)
                    if isSelected {
                        RoundedRectangle(cornerRadius: CGFloat(UIOptimizationPresentation.sidebarSelection.accentLineWidth) / 2)
                            .fill(Color.accentColor)
                            .frame(width: CGFloat(UIOptimizationPresentation.sidebarSelection.accentLineWidth))
                            .padding(.vertical, 7)
                            .padding(.leading, 1)
                    }
                }
            )
            .contentShape(RoundedRectangle(cornerRadius: 9))
        }
        .buttonStyle(.plain)
        .accessibilityLabel(title)
        .accessibilityAddTraits(isSelected ? .isSelected : [])
    }

    private var background: Color {
        isSelected ? Color(nsColor: .selectedContentBackgroundColor).opacity(0.14) : Color.agentCopilotPanelBackground
    }

    private var borderColor: Color {
        isSelected ? Color.accentColor.opacity(0.38) : Color.secondary.opacity(0.12)
    }

    private var iconBackground: Color {
        isSelected ? Color.primary.opacity(0.12) : Color.secondary.opacity(0.1)
    }

    private var iconColor: Color {
        .primary
    }

    private var primaryTextColor: Color {
        .primary
    }

    private var secondaryTextColor: Color {
        .secondary
    }
}

private struct SidebarNavigationMetricPill: View {
    let metric: SidebarNavigationMetric
    let isSelected: Bool

    var body: some View {
        HStack(spacing: 3) {
            Text(metric.title)
                .foregroundStyle(labelColor)
                .lineLimit(1)
            Text(metric.value)
                .fontWeight(.semibold)
                .monospacedDigit()
                .foregroundStyle(valueColor)
                .lineLimit(1)
        }
        .font(.caption2)
        .lineLimit(1)
        .minimumScaleFactor(0.5)
        .padding(.horizontal, 5)
        .padding(.vertical, 3)
        .background(
            isSelected ? Color.accentColor.opacity(0.10) : Color.agentCopilotPanelBackground,
            in: Capsule()
        )
    }

    private var labelColor: Color {
        .secondary
    }

    private var valueColor: Color {
        metric.tone.valueColor
    }
}

private struct SessionSidebarPanel: View {
    @EnvironmentObject private var store: SkillStore

    var body: some View {
        let preview = store.localSessionPreviewResult
        let filteredRows = store.filteredLocalSessionRows

        Group {
            Section {
                sessionToolbar

                if let message = sessionStatusMessage {
                    Text(message)
                        .font(.caption)
                        .foregroundStyle(.secondary)
                        .lineLimit(3)
                        .padding(.horizontal, CGFloat(UIOptimizationPresentation.listPage.cardHorizontalInset))
                }
            }
            .listPageChromeRow()

            Section(UIStrings.text("sidebar.sessions.list", "Sessions")) {
                if preview.sessionRows.isEmpty && store.isPreviewingLocalSessions {
                    SidebarEmptyMessage(message: UIStrings.text("sidebar.sessions.loading", "Loading sessions..."))
                } else if preview.sessionRows.isEmpty {
                    SidebarEmptyMessage(message: UIStrings.text("sidebar.sessions.empty", "No local sessions found."))
                } else if filteredRows.isEmpty {
                    SidebarEmptyMessage(message: UIStrings.localSessionNoMatchesMessage(totalCount: preview.totalMatchedCount))
                } else {
                    ForEach(filteredRows) { session in
                        SessionSidebarRow(
                            session: session,
                            showsProjectRoot: store.localSessionScopeFilter == .all,
                            isSelected: store.selectedSidebarSelection == .session(session.id)
                        ) {
                            store.selectLocalSession(session)
                        }
                        .listPageCardRow()
                    }
                }
            }

            if shouldShowPagingFooter {
                Section {
                    VStack(alignment: .leading, spacing: 8) {
                        ListCompletenessFooter(
                            state: store.localSessionCompleteness,
                            onLoadMore: { Task { await store.loadMoreLocalSessions() } },
                            onLoadAll: { Task { await store.loadAllLocalSessions() } },
                            onCancel: { store.cancelLocalSessionLoadAll() },
                            accessibilityIdentifierPrefix: "sessions"
                        )
                        .accessibilityIdentifier("sessions.completeness")
                        Text(UIStrings.text(
                            "sidebar.sessions.loadedRowsOnly",
                            "Search, scope, and sort cover loaded rows while more remain."
                        ))
                            .font(.caption2)
                            .foregroundStyle(.secondary)
                    }
                    .padding(.horizontal, CGFloat(UIOptimizationPresentation.listPage.cardHorizontalInset))
                }
                .listPageChromeRow()
            }

            if !preview.skillUsageRows.isEmpty {
                Section(UIStrings.localSessionTopSkillsSummary(totalCount: preview.skillUsageRows.count)) {
                    ExpandableSummaryList(
                        preview.skillUsageRows,
                        visibleLimit: 3,
                        accessibilityIdentifier: "session-top-skills.show-all"
                    ) { row in
                        SidebarMetricRow(
                            title: row.skillName,
                            value: "\(row.callCount)",
                            systemImage: "square.stack.3d.up"
                        )
                    }
                }
            }
        }
    }

    private var sessionStatusMessage: String? {
        if let displayError = store.localSessionSummaryDisplayError, !displayError.isEmpty {
            return displayError
        }
        if let fallback = store.localSessionPreviewResult.fallbackReason, !fallback.isEmpty {
            return fallback
        }
        if store.localSessionPreviewResult.authorizationRequired {
            return UIStrings.text("sidebar.sessions.authorizationHint", "No supported local session store was found for the selected agent.")
        }
        return nil
    }

    private var sessionToolbar: some View {
        let layout = UIOptimizationPresentation.skillList

        return VStack(alignment: .leading, spacing: 8) {
            ListPageTitleBlock(
                title: SidebarContentMode.sessions.title,
                subtitle: "\(store.agentFilter.title) · \(UIStrings.text("sidebar.sessions.loaded", "Local sessions"))",
                countText: sessionCountText
            )
            .padding(.horizontal, CGFloat(UIOptimizationPresentation.listPage.cardHorizontalInset))
            .padding(.top, 12)

            HStack(alignment: .center, spacing: CGFloat(layout.filterControlSpacing)) {
                sessionScopePicker
                sessionSortPicker
                sessionSortDirectionButton(
                    width: CGFloat(layout.sortDirectionButtonWidth),
                    height: CGFloat(layout.filterControlHeight)
                )
                sessionRefreshButton
            }
            .padding(.horizontal, CGFloat(UIOptimizationPresentation.listPage.cardHorizontalInset))

            sessionSearchField
                .padding(.horizontal, CGFloat(UIOptimizationPresentation.listPage.cardHorizontalInset))
        }
    }

    private var sessionCountText: String {
        let completeness = store.localSessionCompleteness
        guard let total = completeness.totalCount, total > completeness.loadedCount else {
            return "\(completeness.loadedCount)"
        }
        return "\(completeness.loadedCount)/\(total)"
    }

    private var shouldShowPagingFooter: Bool {
        store.localSessionCompleteness.hasMore
            || store.localSessionCompleteness.incompleteReason != nil
            || store.localSessionCompleteness.loadingPhase != .idle
    }

    private var sessionScopePicker: some View {
        SkillFilterMenuPicker(
            title: UIStrings.scope,
            selection: $store.localSessionScopeFilter,
            options: LocalSessionScopeFilter.allCases,
            optionTitle: \.title,
            width: 116,
            height: CGFloat(UIOptimizationPresentation.skillList.filterControlHeight),
            expands: false
        )
    }

    private var sessionSortPicker: some View {
        SkillFilterMenuPicker(
            title: UIStrings.sort,
            selection: $store.localSessionSortOrder,
            options: LocalSessionSortOrder.allCases,
            optionTitle: \.title,
            width: 98,
            height: CGFloat(UIOptimizationPresentation.skillList.filterControlHeight),
            expands: false
        )
    }

    private var sessionSearchField: some View {
        SidebarSearchField(
            placeholder: UIStrings.text("sidebar.sessions.search", "Search sessions"),
            text: $store.localSessionSearchText,
            minimumWidth: CGFloat(UIOptimizationPresentation.sessionList.minimumSearchWidth)
        )
    }

    private func sessionSortDirectionButton(width: CGFloat, height: CGFloat) -> some View {
        Button {
            store.localSessionSortDirection = store.localSessionSortDirection == .ascending ? .descending : .ascending
        } label: {
            Image(systemName: store.localSessionSortDirection == .ascending ? "arrow.up" : "arrow.down")
                .font(.caption.weight(.semibold))
                .foregroundStyle(.primary)
                .frame(width: width, height: height)
                .background(Color.agentCopilotPanelBackground, in: Capsule())
                .overlay(
                    Capsule()
                        .stroke(Color.secondary.opacity(0.12), lineWidth: 1)
                )
        }
        .buttonStyle(.plain)
        .help(store.localSessionSortDirection.title)
        .accessibilityLabel(UIStrings.text("sort.direction", "Direction"))
        .accessibilityValue(store.localSessionSortDirection.title)
    }

    private var sessionRefreshButton: some View {
        Button {
            Task { await store.previewLocalSessions() }
        } label: {
            ZStack {
                Image(systemName: "arrow.clockwise")
                    .opacity(store.isPreviewingLocalSessions ? 0 : 1)
                if store.isPreviewingLocalSessions {
                    ProgressView()
                        .controlSize(.small)
                        .scaleEffect(0.68)
                }
            }
            .frame(width: CGFloat(UIOptimizationPresentation.skillList.sortDirectionButtonWidth), height: CGFloat(UIOptimizationPresentation.skillList.filterControlHeight))
            .background(Color.agentCopilotPanelBackground, in: Capsule())
            .overlay(
                Capsule()
                    .stroke(Color.secondary.opacity(0.12), lineWidth: 1)
            )
        }
        .buttonStyle(.plain)
        .disabled(store.isRefreshBusy || store.isPreviewingLocalSessions)
        .help(UIStrings.text("sidebar.sessions.preview", "Refresh Sessions"))
        .accessibilityLabel(UIStrings.text("sidebar.sessions.preview", "Refresh Sessions"))
    }

}

private struct SidebarSearchField: View {
    let placeholder: String
    @Binding var text: String
    let minimumWidth: CGFloat

    var body: some View {
        TextField(placeholder, text: $text)
            .textFieldStyle(.roundedBorder)
            .controlSize(.small)
            .frame(minWidth: minimumWidth, maxWidth: .infinity)
    }
}

private extension View {
    func optimizedSidebarSelection(isSelected: Bool) -> some View {
        modifier(OptimizedSidebarSelectionModifier(isSelected: isSelected))
    }

    func listPageCardBackground(isSelected: Bool) -> some View {
        modifier(ListPageCardBackgroundModifier(isSelected: isSelected))
    }
}

private struct OptimizedSidebarSelectionModifier: ViewModifier {
    let isSelected: Bool

    func body(content: Content) -> some View {
        content
            .background(
                RoundedRectangle(cornerRadius: CGFloat(UIOptimizationPresentation.sidebarSelection.rowCornerRadius))
                    .fill(isSelected ? Color(nsColor: .selectedContentBackgroundColor).opacity(0.14) : Color.clear)
            )
            .overlay(alignment: .leading) {
                if isSelected {
                    Rectangle()
                        .fill(Color.accentColor)
                        .frame(width: CGFloat(UIOptimizationPresentation.sidebarSelection.accentLineWidth))
                        .clipShape(Capsule())
                        .padding(.vertical, 5)
                }
            }
    }
}

private struct ListPageCardBackgroundModifier: ViewModifier {
    let isSelected: Bool

    func body(content: Content) -> some View {
        content
            .background(cardFill, in: RoundedRectangle(cornerRadius: CGFloat(UIOptimizationPresentation.listPage.cardCornerRadius)))
            .overlay(
                ZStack(alignment: .leading) {
                    RoundedRectangle(cornerRadius: CGFloat(UIOptimizationPresentation.listPage.cardCornerRadius))
                        .stroke(borderColor, lineWidth: 1)
                    if isSelected {
                        RoundedRectangle(cornerRadius: CGFloat(UIOptimizationPresentation.sidebarSelection.accentLineWidth) / 2)
                            .fill(Color.accentColor)
                            .frame(width: CGFloat(UIOptimizationPresentation.sidebarSelection.accentLineWidth))
                            .padding(.vertical, 8)
                            .padding(.leading, 1)
                    }
                }
            )
    }

    private var cardFill: AnyShapeStyle {
        isSelected
            ? AnyShapeStyle(Color(nsColor: .selectedContentBackgroundColor).opacity(0.16))
            : AnyShapeStyle(Color.agentCopilotPanelBackground)
    }

    private var borderColor: Color {
        isSelected ? Color.accentColor.opacity(0.36) : Color.secondary.opacity(0.13)
    }
}

private struct SessionSidebarRow: View {
    let session: LocalSessionPreviewRow
    let showsProjectRoot: Bool
    let isSelected: Bool
    let onSelect: () -> Void

    var body: some View {
        Button(action: onSelect) {
            HStack(alignment: .center, spacing: 10) {
                Image(systemName: "bubble.left.and.text.bubble.right")
                    .font(.body.weight(.semibold))
                    .foregroundStyle(Color.blue)
                    .frame(width: 32, height: 32)
                    .background(Color.blue.opacity(0.12), in: RoundedRectangle(cornerRadius: 8))

                VStack(alignment: .leading, spacing: 4) {
                    Text(session.title)
                        .font(.callout.weight(.semibold))
                        .foregroundStyle(.primary)
                        .lineLimit(1)
                    Text(sessionCompactSummary)
                        .font(.caption)
                        .foregroundStyle(.secondary)
                        .lineLimit(1)
                }
                .frame(maxWidth: .infinity, alignment: .leading)

                Spacer(minLength: 4)

                Image(systemName: "chevron.right")
                    .font(.caption2.weight(.semibold))
                    .foregroundStyle(isSelected ? Color.accentColor : Color.secondary.opacity(0.65))
                    .frame(width: 10)
                    .accessibilityHidden(true)
            }
            .padding(.horizontal, 10)
            .padding(.vertical, 9)
            .frame(
                maxWidth: .infinity,
                minHeight: CGFloat(UIOptimizationPresentation.listPage.minimumCardRowHeight),
                alignment: .leading
            )
            .listPageCardBackground(isSelected: isSelected)
            .contentShape(RoundedRectangle(cornerRadius: CGFloat(UIOptimizationPresentation.listPage.cardCornerRadius)))
        }
        .buttonStyle(.plain)
        .accessibilityLabel(session.title)
        .accessibilityAddTraits(isSelected ? .isSelected : [])
        .help(sessionHelp)
    }

    private var sessionCompactSummary: String {
        var parts = [
            "\(session.userMessageCount) \(UIStrings.text("sidebar.sessions.userShort", "user"))",
            "\(session.toolCallCount) \(UIStrings.text("sidebar.sessions.toolShort", "tool"))",
            "\(session.skillCallCount) \(UIStrings.text("sidebar.sessions.skillShort", "skill"))"
        ]
        if showsProjectRoot, let project = session.projectRoot, !project.isEmpty {
            parts.append(DisplayText.collapsePath(project, limit: 32))
        } else if let endedAt = session.endedAt ?? session.startedAt {
            parts.append(DisplayText.timestamp(endedAt))
        }
        return parts.joined(separator: " · ")
    }

    private var sessionHelp: String {
        var lines = [session.title, sessionCompactSummary]
        if let startedAt = session.startedAt {
            lines.append("\(UIStrings.text("sidebar.sessions.startShort", "Start")) \(DisplayText.timestamp(startedAt))")
        }
        if let endedAt = session.endedAt, session.startedAt.map({ $0 != endedAt }) ?? true {
            lines.append("\(UIStrings.text("sidebar.sessions.lastShort", "Last")) \(DisplayText.timestamp(endedAt))")
        }
        if let project = session.projectRoot, !project.isEmpty {
            lines.append(project)
        }
        return lines.joined(separator: "\n")
    }

}

private struct SidebarMetricRow: View {
    let title: String
    let value: String
    let systemImage: String

    var body: some View {
        HStack(spacing: 8) {
            Image(systemName: systemImage)
                .foregroundStyle(.secondary)
                .frame(width: 17)
            Text(title)
                .font(.caption)
                .foregroundStyle(.secondary)
                .lineLimit(1)
            Spacer(minLength: 8)
            Text(value)
                .font(.caption.bold())
                .foregroundStyle(.primary)
                .lineLimit(1)
        }
    }
}

private struct SkillSidebarPanel: View {
    @EnvironmentObject private var store: SkillStore
    @Binding var isBatchOperationPresented: Bool
    @State private var searchDraftText = ""
    @State private var searchCommitTask: Task<Void, Never>?

    private static let searchDebounceDelayNanoseconds: UInt64 = 250_000_000

    var body: some View {
        let visibleSkills = store.filteredSkills
        let catalogCompleteness = store.filteredCatalogListCompleteness

        Group {
            Section {
                skillToolbar(visibleSkills: visibleSkills)
            }
            .listPageChromeRow()

            if store.skills.isEmpty {
                Section(UIStrings.skills) {
                    SidebarEmptyMessage(message: store.isLoading ? UIStrings.loading : emptyCatalogMessage)
                }
            } else if visibleSkills.isEmpty {
                Section(UIStrings.skills) {
                    SidebarEmptyMessage(message: emptyFilteredMessage)
                }
            } else {
                Section(UIStrings.text("sidebar.skills.list", "Skill List")) {
                    ForEach(visibleSkills) { skill in
                        SkillRow(
                            skill: skill,
                            issueCount: store.issueIndicatorCount(for: skill),
                            conflictCount: store.conflictIndicatorCount(for: skill),
                            isSelected: store.selectedSidebarSelection == .skill(skill.id)
                        ) {
                            store.selectedSidebarSelection = .skill(skill.id)
                        }
                        .equatable()
                        .id(skill.id)
                        .listPageCardRow()
                    }
                }
                .id(skillListRefreshID(visibleCount: visibleSkills.count))
            }

            if catalogCompleteness.completeness != .complete {
                Section {
                    ListCompletenessFooter(
                        state: catalogCompleteness,
                        onLoadMore: {},
                        onLoadAll: {},
                        onCancel: {}
                    )
                }
            }
        }
        .onAppear {
            synchronizeSearchDraft(with: store.searchText)
        }
        .onChange(of: store.searchText) { committedText in
            synchronizeSearchDraft(with: committedText)
        }
        .onDisappear {
            searchCommitTask?.cancel()
            searchCommitTask = nil
        }
    }

    private func skillToolbar(visibleSkills: [SkillRecord]) -> some View {
        VStack(alignment: .leading, spacing: 8) {
            ListPageTitleBlock(
                title: UIStrings.skills,
                subtitle: "\(store.agentFilter.title) · \(store.skillScopeFilter.title)",
                countText: "\(visibleSkills.count)"
            )
            .padding(.horizontal, CGFloat(UIOptimizationPresentation.listPage.cardHorizontalInset))
            .padding(.top, 12)

            filterControls
                .padding(.horizontal, CGFloat(UIOptimizationPresentation.listPage.cardHorizontalInset))

            HStack(spacing: 8) {
                searchField
                    .frame(minWidth: CGFloat(UIOptimizationPresentation.skillList.minimumSearchWidth))
                batchToolbarButton(visibleSkills: visibleSkills)
            }
            .padding(.horizontal, CGFloat(UIOptimizationPresentation.listPage.cardHorizontalInset))
        }
    }

    private var filterControls: some View {
        let layout = UIOptimizationPresentation.skillList

        return HStack(alignment: .center, spacing: CGFloat(layout.filterControlSpacing)) {
            SkillFilterMenuPicker(
                title: UIStrings.text("sidebar.skillFilter", "Filter"),
                selection: $store.stateFilter,
                options: SkillStateFilter.sidebarCases,
                optionTitle: \.title,
                width: CGFloat(layout.filterControlWidth),
                height: CGFloat(layout.filterControlHeight)
            )

            SkillFilterMenuPicker(
                title: UIStrings.text("sidebar.scopeFilter", "Scope"),
                selection: $store.skillScopeFilter,
                options: SkillScopeFilter.allCases,
                optionTitle: \.title,
                width: CGFloat(layout.filterControlWidth),
                height: CGFloat(layout.filterControlHeight)
            )

            SkillFilterMenuPicker(
                title: UIStrings.sort,
                selection: $store.sortOrder,
                options: SkillSortOrder.allCases,
                optionTitle: \.title,
                width: CGFloat(layout.filterControlWidth),
                height: CGFloat(layout.filterControlHeight)
            )

            sortDirectionButton(width: CGFloat(layout.sortDirectionButtonWidth), height: CGFloat(layout.filterControlHeight))
        }
        .padding(.vertical, CGFloat(layout.filterToolbarVerticalPadding))
    }

    private func sortDirectionButton(width: CGFloat, height: CGFloat) -> some View {
        Button {
            store.sortDirection = store.sortDirection == .ascending ? .descending : .ascending
        } label: {
            Image(systemName: store.sortDirection == .ascending ? "arrow.up" : "arrow.down")
                .font(.caption.weight(.semibold))
                .foregroundStyle(.primary)
                .frame(width: width, height: height)
                .background(Color.agentCopilotPanelBackground, in: Capsule())
                .overlay(
                    Capsule()
                        .stroke(Color.secondary.opacity(0.12), lineWidth: 1)
                )
        }
        .buttonStyle(.plain)
        .help(store.sortDirection.title)
        .accessibilityLabel(UIStrings.text("sort.direction", "Direction"))
        .accessibilityValue(store.sortDirection.title)
    }

    private var searchField: some View {
        TextField(UIStrings.searchPrompt, text: $searchDraftText)
            .textFieldStyle(.roundedBorder)
            .controlSize(.small)
            .frame(maxWidth: .infinity)
            .onChange(of: searchDraftText) { newValue in
                scheduleSearchCommit(newValue)
            }
    }

    private func batchToolbarButton(visibleSkills: [SkillRecord]) -> some View {
        Button {
            store.resetBatchToggleSelectionToVisibleSkills()
            isBatchOperationPresented = true
        } label: {
            Image(systemName: "checklist.checked")
                .foregroundStyle(.primary)
                .frame(width: CGFloat(UIOptimizationPresentation.skillList.sortDirectionButtonWidth), height: CGFloat(UIOptimizationPresentation.skillList.filterControlHeight))
                .background(Color.agentCopilotPanelBackground, in: Capsule())
                .overlay(
                    Capsule()
                        .stroke(Color.secondary.opacity(0.12), lineWidth: 1)
                )
        }
        .buttonStyle(.plain)
        .disabled(visibleSkills.isEmpty || store.isRefreshBusy)
        .help(UIStrings.batchToggleOpenHelp)
        .accessibilityLabel(UIStrings.batchToggleOpen)
    }

    private func skillListRefreshID(visibleCount: Int) -> String {
        [
            store.agentFilter.rawValue,
            store.stateFilter.rawValue,
            store.skillScopeFilter.rawValue,
            store.sortOrder.rawValue,
            store.sortDirection.rawValue,
            store.searchText,
            String(visibleCount)
        ].joined(separator: "|")
    }

    private var emptyCatalogMessage: String {
        if store.activeProjectContext == nil {
            return UIStrings.noProjectSkillsMessage
        }
        return UIStrings.noSkillsInCatalog
    }

    private var emptyFilteredMessage: String {
        if let capability = store.selectedAdapterCapability, !capability.scan.supported {
            return capability.scan.reason ?? UIStrings.adapterNotImplementedMessage(DisplayText.agent(capability.agent))
        }
        return UIOptimizationPresentation.skillList.emptyFilteredMessage(
            agentFilter: store.agentFilter,
            hasActiveProjectContext: store.activeProjectContext != nil,
            hasActiveSearchOrFilter: hasActiveSearchOrFilter
        )
    }

    private var hasActiveSearchOrFilter: Bool {
        store.stateFilter != .all
            || store.skillScopeFilter != .all
            || !store.searchText.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
    }

    private func synchronizeSearchDraft(with committedText: String) {
        guard searchDraftText != committedText else { return }
        searchCommitTask?.cancel()
        searchCommitTask = nil
        searchDraftText = committedText
    }

    private func scheduleSearchCommit(_ text: String) {
        searchCommitTask?.cancel()
        let delay = Self.searchDebounceDelayNanoseconds
        searchCommitTask = Task { @MainActor in
            try? await Task.sleep(nanoseconds: delay)
            guard !Task.isCancelled else { return }
            if store.searchText != text {
                store.searchText = text
            }
            searchCommitTask = nil
        }
    }

}

private struct SkillFilterMenuPicker<Option: Identifiable>: View where Option.ID: Hashable {
    let title: String
    @Binding var selection: Option
    let options: [Option]
    let optionTitle: (Option) -> String
    let width: CGFloat
    let height: CGFloat
    var expands = true

    var body: some View {
        Menu {
            ForEach(options) { option in
                Button {
                    selection = option
                } label: {
                    menuItemLabel(for: option)
                }
            }
        } label: {
            pickerLabel
                .accessibilityHidden(true)
        }
        .menuStyle(.button)
        .buttonStyle(.plain)
        .help(title)
        .accessibilityLabel(title)
        .accessibilityValue(optionTitle(selection))
    }

    private var pickerLabel: some View {
        SidebarMenuButtonLabel(
            title: title,
            value: optionTitle(selection),
            width: width,
            height: height,
            expands: expands
        )
    }

    @ViewBuilder
    private func menuItemLabel(for option: Option) -> some View {
        if option.id == selection.id {
            Label(optionTitle(option), systemImage: "checkmark")
        } else {
            Text(optionTitle(option))
        }
    }
}

private struct SidebarMenuButtonLabel: View {
    let title: String?
    let value: String?
    var agentFilter: SkillAgentFilter?
    var systemImage: String?
    let width: CGFloat
    let height: CGFloat
    var expands = true
    var showsChevron = true
    var horizontalPadding: CGFloat = 7

    init(
        title: String? = nil,
        value: String? = nil,
        agentFilter: SkillAgentFilter? = nil,
        systemImage: String? = nil,
        width: CGFloat,
        height: CGFloat,
        expands: Bool = true,
        showsChevron: Bool = true,
        horizontalPadding: CGFloat = 7
    ) {
        self.title = title
        self.value = value
        self.agentFilter = agentFilter
        self.systemImage = systemImage
        self.width = width
        self.height = height
        self.expands = expands
        self.showsChevron = showsChevron
        self.horizontalPadding = horizontalPadding
    }

    var body: some View {
        HStack(spacing: 5) {
            if let agentFilter {
                AgentIconBadge(filter: agentFilter, size: 26)
                    .fixedSize()
                    .help(DisplayText.agent(agentFilter.rawValue))
            } else if let systemImage {
                Image(systemName: systemImage)
                    .font(.caption.weight(.semibold))
                    .foregroundStyle(.primary)
                    .frame(width: 15)
            }

            if let title, !title.isEmpty {
                Text(title)
                    .foregroundStyle(.secondary)
                    .lineLimit(1)
                    .minimumScaleFactor(0.72)
            }

            if let value, !value.isEmpty {
                Text(value)
                    .lineLimit(1)
                    .minimumScaleFactor(0.78)
                    .frame(maxWidth: expands ? .infinity : nil, alignment: .leading)
            }

            if showsChevron {
                Image(systemName: "chevron.up.chevron.down")
                    .font(.caption2.weight(.semibold))
                    .foregroundStyle(.secondary)
            }
        }
        .font(.caption.weight(.medium))
        .foregroundStyle(.primary)
        .padding(.horizontal, horizontalPadding)
        .frame(minWidth: width, maxWidth: expands ? .infinity : nil, minHeight: height, maxHeight: height)
        .fixedSize(horizontal: !expands, vertical: false)
        .background(Color.agentCopilotPanelBackground, in: Capsule())
        .overlay(
            Capsule()
                .stroke(Color.secondary.opacity(0.12), lineWidth: 1)
        )
        .contentShape(Capsule())
    }
}

private struct ConfigSidebarPanel: View {
    @EnvironmentObject private var store: SkillStore

    private var capability: AdapterCapabilityRecord? {
        store.adapterCapabilities.first { $0.agent == store.agentFilter.rawValue }
    }

    private var selectedSnapshots: [ConfigSnapshotRecord] {
        AgentConfigSidebarModel.filteredSnapshots(
            store.agentConfigSnapshots,
            agentFilter: store.agentFilter,
            scopeFilter: store.configScopeFilter,
            searchText: store.configSidebarSearchText
        )
    }

    private var disabledSkills: [SkillRecord] {
        AgentConfigDisplay.disabledSkills(for: store.agentFilter, store: store)
    }

    private var selectedConfigDocuments: [ConfigDocumentRecord] {
        store.visibleConfigDocuments
    }

    var body: some View {
        Group {
            Section {
                configToolbar
            }
            .listPageChromeRow()

            Section(UIStrings.currentConfigFile) {
                if selectedConfigDocuments.isEmpty, store.isLoadingAgentConfigDocuments {
                    Label(UIStrings.loading, systemImage: "hourglass")
                        .font(.caption)
                        .foregroundStyle(.secondary)
                } else if selectedConfigDocuments.isEmpty {
                    SidebarEmptyMessage(message: UIStrings.agentConfigNoReadableDocuments)
                } else {
                    ForEach(selectedConfigDocuments, id: \.target) { document in
                        ConfigCurrentDocumentSidebarRow(
                            document: document,
                            isSelected: store.selectedSidebarSelection == .configDocument(document.target)
                        ) {
                            store.selectConfigDocument(document)
                        }
                    }
                }
            }

            Section(UIStrings.text("sidebar.config.operations", "Supported operations")) {
                ConfigOperationRow(title: UIStrings.scan, capability: capability?.scan, systemImage: "magnifyingglass")
                ConfigOperationRow(title: UIStrings.projectScan, capability: capability?.projectScan, systemImage: "folder")
                ConfigOperationRow(title: UIStrings.configToggle, capability: capability?.configToggle, systemImage: "switch.2")
                ConfigOperationRow(title: UIStrings.configSnapshot, capability: capability?.configSnapshot, systemImage: "clock.arrow.circlepath")
                ConfigOperationRow(title: UIStrings.writableConfig, capability: capability?.writable, systemImage: "lock.open")
            }

            Section(UIStrings.agentConfigSkillEnablement) {
                ConfigDisabledSkillSummaryRow(skills: disabledSkills)
            }

            Section(UIStrings.agentConfigSettingsHistory) {
                if selectedSnapshots.isEmpty, store.isLoadingAgentConfigSnapshots {
                    Label(UIStrings.loading, systemImage: "hourglass")
                        .font(.caption)
                        .foregroundStyle(.secondary)
                } else if selectedSnapshots.isEmpty {
                    SidebarEmptyMessage(message: UIStrings.agentConfigHistoryEmpty(DisplayText.agent(store.agentFilter.rawValue)))
                } else {
                    ForEach(selectedSnapshots) { snapshot in
                        ConfigSnapshotSidebarRow(
                            item: AgentConfigTimelineItem(snapshot: snapshot),
                            isSelected: store.selectedSidebarSelection == .configSnapshot(snapshot.id)
                        ) {
                            store.selectConfigSnapshot(snapshot)
                        }
                    }
                }

                if store.agentConfigSnapshotCompleteness.loadingPhase != .idle
                    || store.agentConfigSnapshotCompleteness.completeness != .complete {
                    ListCompletenessFooter(
                        state: store.agentConfigSnapshotCompleteness,
                        onLoadMore: {
                            Task { await store.loadMoreAgentConfigSnapshots(loadAll: false) }
                        },
                        onLoadAll: {
                            Task { await store.loadMoreAgentConfigSnapshots(loadAll: true) }
                        },
                        onCancel: {
                            store.cancelAgentConfigSnapshotLoadAll()
                        },
                        accessibilityIdentifierPrefix: "config-history"
                    )
                    .accessibilityIdentifier("config-history.completeness")
                }
            }
        }
        .task(id: store.selectedAgentConfigRefreshKey) {
            await store.loadSelectedAgentConfigDataIfNeeded()
        }
    }

    private var configToolbar: some View {
        let layout = UIOptimizationPresentation.skillList

        return VStack(alignment: .leading, spacing: 8) {
            ListPageTitleBlock(
                title: SidebarContentMode.config.title,
                subtitle: "\(store.agentFilter.title) · \(store.configScopeFilter.title)",
                countText: "\(selectedConfigDocuments.count)"
            )
            .padding(.horizontal, CGFloat(UIOptimizationPresentation.listPage.cardHorizontalInset))
            .padding(.top, 12)

            HStack(alignment: .center, spacing: CGFloat(layout.filterControlSpacing)) {
                configScopePicker
                configRefreshButton(
                    width: CGFloat(layout.sortDirectionButtonWidth),
                    height: CGFloat(layout.filterControlHeight)
                )
            }
            .padding(.horizontal, CGFloat(UIOptimizationPresentation.listPage.cardHorizontalInset))

            configSearchField
                .padding(.horizontal, CGFloat(UIOptimizationPresentation.listPage.cardHorizontalInset))
        }
    }

    private var configScopePicker: some View {
        SkillFilterMenuPicker(
            title: UIStrings.scope,
            selection: $store.configScopeFilter,
            options: AgentConfigScopeFilter.allCases,
            optionTitle: \.title,
            width: 116,
            height: CGFloat(UIOptimizationPresentation.skillList.filterControlHeight),
            expands: false
        )
    }

    private var configSearchField: some View {
        SidebarSearchField(
            placeholder: UIStrings.text("sidebar.config.search", "Search config"),
            text: $store.configSidebarSearchText,
            minimumWidth: CGFloat(UIOptimizationPresentation.configList.minimumSearchWidth)
        )
    }

    private func configRefreshButton(width: CGFloat, height: CGFloat) -> some View {
        Button {
            Task { await store.refreshSelectedAgentConfigData() }
        } label: {
            ZStack {
                Image(systemName: "arrow.clockwise")
                    .opacity(store.isLoadingAgentConfigDocuments || store.isLoadingAgentConfigSnapshots ? 0 : 1)
                if store.isLoadingAgentConfigDocuments || store.isLoadingAgentConfigSnapshots {
                    ProgressView()
                        .controlSize(.small)
                        .scaleEffect(0.68)
                }
            }
            .foregroundStyle(.primary)
            .frame(width: width, height: height)
            .background(Color.agentCopilotPanelBackground, in: Capsule())
            .overlay(
                Capsule()
                    .stroke(Color.secondary.opacity(0.12), lineWidth: 1)
            )
        }
        .buttonStyle(.plain)
        .disabled(store.isLoadingAgentConfigDocuments || store.isLoadingAgentConfigSnapshots)
        .help(UIStrings.reload)
        .accessibilityLabel(UIStrings.reload)
    }
}

private struct ConfigCurrentDocumentSidebarRow: View {
    let document: ConfigDocumentRecord
    let isSelected: Bool
    let onSelect: () -> Void

    var body: some View {
        Button(action: onSelect) {
            HStack(alignment: .center, spacing: 8) {
                Image(systemName: document.exists ? "doc.text" : "doc.badge.plus")
                    .foregroundStyle(isSelected ? Color.accentColor : .secondary)
                    .frame(width: 16)

                VStack(alignment: .leading, spacing: 3) {
                    Text(DisplayText.scope(document.scope))
                        .font(.caption.bold())
                        .foregroundStyle(.primary)
                        .lineLimit(1)
                    Text("\(AgentConfigDisplay.pathSummary(document.target)) · \(document.exists ? UIStrings.existingFile : UIStrings.willCreateFile)")
                        .font(.caption2)
                        .foregroundStyle(.secondary)
                        .lineLimit(1)
                        .help(document.target)
                }

                Spacer(minLength: 4)
            }
            .padding(.vertical, 5)
            .padding(.horizontal, 7)
            .frame(
                maxWidth: .infinity,
                minHeight: CGFloat(UIOptimizationPresentation.configList.compactRowMinHeight),
                maxHeight: CGFloat(UIOptimizationPresentation.configList.compactRowMaxHeight),
                alignment: .leading
            )
            .optimizedSidebarSelection(isSelected: isSelected)
            .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
        .accessibilityLabel("\(DisplayText.scope(document.scope)), \(AgentConfigDisplay.pathSummary(document.target))")
    }
}

private struct ConfigOperationRow: View {
    let title: String
    let capability: AdapterFeatureCapability?
    let systemImage: String

    var body: some View {
        HStack(spacing: 8) {
            Image(systemName: systemImage)
                .foregroundStyle(AgentConfigDisplay.supportColor(capability))
                .frame(width: 17)
            VStack(alignment: .leading, spacing: 1) {
                Text(title)
                    .font(.caption.bold())
                    .lineLimit(1)
            }
            Spacer(minLength: 6)
            Image(systemName: AgentConfigDisplay.supportSymbol(capability))
                .foregroundStyle(AgentConfigDisplay.supportColor(capability))
        }
        .help(capability?.reason ?? AgentConfigDisplay.supportText(capability))
    }
}

private struct ConfigDisabledSkillSummaryRow: View {
    let skills: [SkillRecord]

    var body: some View {
        HStack(alignment: .top, spacing: 8) {
            Image(systemName: skills.isEmpty ? "checkmark.circle.fill" : "pause.circle.fill")
                .foregroundStyle(skills.isEmpty ? Color.green : Color.orange)
                .frame(width: 17)

            VStack(alignment: .leading, spacing: 2) {
                Text(UIStrings.agentConfigDisabledSkillsCount(skills.count))
                    .font(.caption.bold())
                    .lineLimit(1)
                Text(summary)
                    .font(.caption2)
                    .foregroundStyle(.secondary)
                    .lineLimit(2)
            }

            Spacer(minLength: 4)
        }
        .help(summary)
    }

    private var summary: String {
        guard !skills.isEmpty else {
            return UIStrings.agentConfigDisabledSkillsEmpty
        }
        return AgentConfigDisplay.disabledSkillNamesSummary(skills, limit: 2)
    }
}

private struct ConfigSnapshotSidebarRow: View {
    let item: AgentConfigTimelineItem
    let isSelected: Bool
    let onSelect: () -> Void

    var body: some View {
        Button(action: onSelect) {
            HStack(alignment: .center, spacing: 8) {
                Image(systemName: "doc.text")
                    .foregroundStyle(isSelected ? Color.accentColor : .secondary)
                    .frame(width: 16)

                VStack(alignment: .leading, spacing: 3) {
                    Text(item.actionText)
                        .font(.caption.bold())
                        .foregroundStyle(.primary)
                        .lineLimit(1)
                    Text("\(item.timeText) · \(item.scopeText) · \(item.capturedText)")
                        .font(.caption2)
                        .foregroundStyle(.secondary)
                        .lineLimit(1)
                }

                Spacer(minLength: 4)
            }
            .padding(.vertical, 5)
            .padding(.horizontal, 7)
            .frame(
                maxWidth: .infinity,
                minHeight: CGFloat(UIOptimizationPresentation.configList.compactRowMinHeight),
                maxHeight: CGFloat(UIOptimizationPresentation.configList.compactRowMaxHeight),
                alignment: .leading
            )
            .optimizedSidebarSelection(isSelected: isSelected)
            .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
        .accessibilityLabel("\(item.actionText), \(item.timeText), \(item.scopeText), \(item.capturedText)")
        .help(item.targetSummary)
    }
}

private struct AgentConfigTimelinePanel: View {
    let model: AgentConfigTimelineModel
    let isLoading: Bool
    let isWriting: Bool
    let onPreview: (String) async throws -> SnapshotRollbackPreviewRecord
    let onRollback: (String) async -> Void

    @State private var isExpanded = false
    @State private var preview: SnapshotRollbackPreviewRecord?
    @State private var previewError: String?
    @State private var snapshotToRollback: ConfigSnapshotRecord?

    var body: some View {
        DisclosureGroup(isExpanded: $isExpanded) {
            VStack(alignment: .leading, spacing: 10) {
                if isLoading {
                    Label(UIStrings.loading, systemImage: "hourglass")
                        .font(.caption)
                        .foregroundStyle(.secondary)
                }

                if let previewError {
                    Label(previewError, systemImage: "exclamationmark.triangle")
                        .font(.caption)
                        .foregroundStyle(.red)
                        .padding(8)
                        .frame(maxWidth: .infinity, alignment: .leading)
                        .background(.red.opacity(0.08), in: RoundedRectangle(cornerRadius: 8))
                }

                Text(UIStrings.agentConfigTimelineBoundary)
                    .font(.caption2)
                    .foregroundStyle(.secondary)
                    .lineLimit(3)
                    .padding(8)
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .background(.blue.opacity(0.08), in: RoundedRectangle(cornerRadius: 8))

                if !model.isSpecificAgent {
                    SidebarEmptyMessage(message: UIStrings.agentConfigTimelineSelectAgent)
                } else if model.items.isEmpty, !isLoading {
                    SidebarEmptyMessage(message: UIStrings.noSnapshotsMessage)
                } else {
                    LazyVStack(alignment: .leading, spacing: 8) {
                        ForEach(model.items) { item in
                            AgentConfigTimelineRow(
                                item: item,
                                isWriting: isWriting,
                                onPreview: {
                                    loadPreview(item.id)
                                },
                                onRollback: {
                                    snapshotToRollback = item.snapshot
                                }
                            )
                        }

                    }
                }
            }
            .padding(.top, 8)
        } label: {
            HStack(alignment: .center, spacing: 8) {
                Image(systemName: "clock.arrow.circlepath")
                    .foregroundStyle(.secondary)
                VStack(alignment: .leading, spacing: 2) {
                    Text(UIStrings.agentConfigTimeline)
                        .font(.subheadline.bold())
                    Text(model.summaryText)
                        .font(.caption)
                        .foregroundStyle(.secondary)
                        .lineLimit(2)
                }
            }
        }
        .sheet(item: $preview) { preview in
            SnapshotPreviewSheet(preview: preview)
        }
        .confirmationDialog(
            UIStrings.rollbackSnapshotQuestion,
            isPresented: Binding(
                get: { snapshotToRollback != nil },
                set: { isPresented in
                    if !isPresented {
                        snapshotToRollback = nil
                    }
                }
            ),
            titleVisibility: .visible
        ) {
            Button(UIStrings.rollback, role: .destructive) {
                if let snapshotID = snapshotToRollback?.id {
                    Task { await onRollback(snapshotID) }
                }
                snapshotToRollback = nil
            }
            Button(UIStrings.cancel, role: .cancel) {
                snapshotToRollback = nil
            }
        } message: {
            Text(UIStrings.agentConfigTimelineRollbackConfirm(
                AgentConfigDisplay.pathSummary(snapshotToRollback?.target ?? "")
            ))
        }
    }

    private func loadPreview(_ snapshotID: String) {
        previewError = nil
        Task {
            do {
                preview = try await onPreview(snapshotID)
            } catch {
                previewError = error.localizedDescription
            }
        }
    }
}

private struct AgentConfigTimelineRow: View {
    let item: AgentConfigTimelineItem
    let isWriting: Bool
    let onPreview: () -> Void
    let onRollback: () -> Void

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            HStack(alignment: .top, spacing: 8) {
                VStack(spacing: 4) {
                    Circle()
                        .fill(Color.accentColor)
                        .frame(width: 8, height: 8)
                    Rectangle()
                        .fill(.separator)
                        .frame(width: 1, height: 32)
                }
                .padding(.top, 4)

                VStack(alignment: .leading, spacing: 5) {
                    HStack(alignment: .firstTextBaseline, spacing: 6) {
                        Text(item.actionText)
                            .font(.caption.bold())
                            .lineLimit(1)
                        Spacer(minLength: 4)
                        Text(item.timeText)
                            .font(.caption2)
                            .foregroundStyle(.secondary)
                            .lineLimit(1)
                    }

                    Text(item.targetSummary)
                        .font(.caption2)
                        .foregroundStyle(.secondary)
                        .lineLimit(1)
                        .help(item.targetSummary)

                    HStack(spacing: 6) {
                        TimelinePill(title: item.scopeText, systemImage: "folder")
                        TimelinePill(title: item.statusText, systemImage: "checkmark.seal")
                    }

                    Text(item.capturedText)
                        .font(.caption2)
                        .foregroundStyle(.tertiary)
                }
            }

            HStack(spacing: 8) {
                Button {
                    onPreview()
                } label: {
                    Label(UIStrings.previewDiff, systemImage: "doc.text.magnifyingglass")
                }
                .controlSize(.small)
                .disabled(isWriting)

                Button(role: .destructive) {
                    onRollback()
                } label: {
                    Label(UIStrings.rollback, systemImage: "arrow.uturn.backward")
                }
                .controlSize(.small)
                .disabled(isWriting)
            }
            .buttonStyle(.borderless)
        }
        .padding(9)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(Color.agentCopilotPanelBackground, in: RoundedRectangle(cornerRadius: 10))
    }
}

private struct TimelinePill: View {
    let title: String
    let systemImage: String

    var body: some View {
        Label(title, systemImage: systemImage)
            .font(.caption2.bold())
            .foregroundStyle(.secondary)
            .lineLimit(1)
            .padding(.horizontal, 6)
            .padding(.vertical, 3)
            .background(Color.agentCopilotPanelBackground, in: Capsule())
    }
}

private struct AgentIconBadge: View {
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

private struct SidebarEmptyMessage: View {
    let message: String

    var body: some View {
        Text(message)
            .font(.callout)
            .foregroundStyle(.secondary)
            .lineLimit(nil)
            .fixedSize(horizontal: false, vertical: true)
            .frame(maxWidth: .infinity, alignment: .leading)
            .padding(.vertical, 4)
    }
}

private struct AgentStatTile: View {
    let value: String
    let label: String
    let systemImage: String

    var body: some View {
        HStack(spacing: 6) {
            Image(systemName: systemImage)
                .font(.caption)
                .foregroundStyle(.secondary)
            VStack(alignment: .leading, spacing: 1) {
                Text(value)
                    .font(.headline)
                Text(label)
                    .font(.caption2)
                    .foregroundStyle(.secondary)
                    .lineLimit(1)
            }
        }
        .padding(.horizontal, 8)
        .padding(.vertical, 7)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(Color.agentCopilotPanelBackground, in: RoundedRectangle(cornerRadius: 8))
    }
}

private struct SkillRow: View, Equatable {
    let skill: SkillRecord
    let issueCount: Int
    let conflictCount: Int
    let isSelected: Bool
    let onSelect: () -> Void

    static func == (lhs: SkillRow, rhs: SkillRow) -> Bool {
        lhs.skill == rhs.skill
            && lhs.issueCount == rhs.issueCount
            && lhs.conflictCount == rhs.conflictCount
            && lhs.isSelected == rhs.isSelected
    }

    var body: some View {
        Button(action: onSelect) {
            HStack(alignment: .center, spacing: 10) {
                Image(systemName: DisplayText.isReadOnlyPreview(skill) ? "lock.fill" : DisplayText.stateSystemImage(skill.state, enabled: skill.enabled))
                    .font(.body.weight(.semibold))
                    .foregroundStyle(DisplayText.isReadOnlyPreview(skill) ? .secondary : DisplayText.stateColor(skill.state, enabled: skill.enabled))
                    .frame(width: 32, height: 32)
                    .background(iconBackground, in: RoundedRectangle(cornerRadius: 8))

                VStack(alignment: .leading, spacing: 4) {
                    Text(skill.name)
                        .font(.callout.weight(.semibold))
                        .foregroundStyle(.primary)
                        .lineLimit(1)
                        .truncationMode(.middle)
                    Text(secondaryText)
                        .font(.caption)
                        .foregroundStyle(.secondary)
                        .lineLimit(1)
                        .truncationMode(.middle)
                }
                .frame(maxWidth: .infinity, alignment: .leading)

                if issueCount > 0 {
                    HStack(spacing: 3) {
                        Image(systemName: "exclamationmark.triangle.fill")
                            .font(.caption2.weight(.semibold))
                        Text("\(issueCount)")
                            .font(.caption2.bold().monospacedDigit())
                    }
                    .foregroundStyle(.orange)
                    .padding(.horizontal, 6)
                    .padding(.vertical, 3)
                    .background(Color.orange.opacity(0.13), in: Capsule())
                    .help(UIStrings.text("sidebar.skillRow.issueCount.help", "Issues associated with this skill"))
                    .accessibilityLabel(UIStrings.text("sidebar.skillRow.issueCount", "Issues"))
                    .accessibilityValue("\(issueCount)")
                }

                if conflictCount > 0 {
                    HStack(spacing: 3) {
                        Image(systemName: "rectangle.stack.badge.exclamationmark")
                            .font(.caption2.weight(.semibold))
                        Text("\(conflictCount)")
                            .font(.caption2.bold().monospacedDigit())
                    }
                    .foregroundStyle(.red)
                    .padding(.horizontal, 6)
                    .padding(.vertical, 3)
                    .background(Color.red.opacity(0.12), in: Capsule())
                    .help(UIStrings.text("sidebar.skillRow.conflictCount.help", "Same-agent conflicts associated with this skill"))
                    .accessibilityLabel(UIStrings.text("sidebar.skillRow.conflictCount", "Conflicts"))
                    .accessibilityValue("\(conflictCount)")
                }

                Image(systemName: "chevron.right")
                    .font(.caption2.weight(.semibold))
                    .foregroundStyle(isSelected ? Color.accentColor : Color.secondary.opacity(0.65))
                    .frame(width: 10)
                    .accessibilityHidden(true)
            }
            .padding(.horizontal, 10)
            .padding(.vertical, 9)
            .frame(
                maxWidth: .infinity,
                minHeight: CGFloat(UIOptimizationPresentation.listPage.minimumCardRowHeight),
                alignment: .leading
            )
            .listPageCardBackground(isSelected: isSelected)
            .contentShape(RoundedRectangle(cornerRadius: CGFloat(UIOptimizationPresentation.listPage.cardCornerRadius)))
        }
        .buttonStyle(.plain)
        .help("\(skill.name)\n\(skill.displayPath)")
        .accessibilityElement(children: .combine)
        .accessibilityLabel(skill.name)
        .accessibilityValue(accessibilityValue)
        .accessibilityAddTraits(isSelected ? .isSelected : [])
    }

    private var iconBackground: Color {
        if DisplayText.isReadOnlyPreview(skill) {
            return Color.secondary.opacity(0.12)
        }
        return DisplayText.stateColor(skill.state, enabled: skill.enabled).opacity(0.13)
    }

    private var accessibilityValue: String {
        var metrics: [String] = []
        if issueCount > 0 {
            metrics.append("\(issueCount) \(UIStrings.findings)")
        }
        if conflictCount > 0 {
            metrics.append("\(conflictCount) \(UIStrings.text("filter.conflicts", "Conflicts"))")
        }
        return ([secondaryText] + metrics).joined(separator: ", ")
    }

    private var secondaryText: String {
        let packageContext = skill.pluginPackageSummary
            ?? skill.packageVersion.map { "v\($0)" }
            ?? skill.sourceKind
        if DisplayText.isToolGlobal(skill) {
            return [DisplayText.scope(for: skill), UIStrings.readOnlyPreview, packageContext ?? skill.provenance.label]
                .joined(separator: " · ")
        }
        if skill.agent == "hermes", DisplayText.isReadOnlyPreview(skill) {
            return "\(DisplayText.scope(for: skill)) · \(skill.provenance.label)"
        }
        if DisplayText.isReadOnlyPreview(skill) {
            return [DisplayText.scope(for: skill), UIStrings.readOnly, packageContext ?? skill.provenance.label]
                .joined(separator: " · ")
        }
        var parts = [DisplayText.scope(for: skill), DisplayText.state(skill.state, enabled: skill.enabled)]
        if let packageContext {
            parts.append(packageContext)
        } else {
            parts.append(skill.provenance.label)
        }
        return parts.joined(separator: " · ")
    }
}
