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

    var body: some View {
        List(selection: $store.selectedSidebarSelection) {
            ConfigSidebarPanel()
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
        .navigationTitle(UIStrings.appWindowTitle)
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
