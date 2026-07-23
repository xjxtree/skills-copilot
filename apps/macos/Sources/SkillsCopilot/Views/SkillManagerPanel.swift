import SwiftUI
import UniformTypeIdentifiers

private enum SkillManagerInventorySourceFilter: String, CaseIterable, Identifiable {
    case all
    case manager
    case local

    var id: String { rawValue }
    var title: String {
        switch self {
        case .all: return UIStrings.text("filter.allSources", "All Sources")
        case .manager: return UIStrings.text("skillManager.source.managerShort", "Manager")
        case .local: return UIStrings.text("skillManager.source.localShort", "Local")
        }
    }

    func includes(_ item: SkillManagerInventoryItem) -> Bool {
        switch self {
        case .all: return true
        case .manager: return item.origin == .manager
        case .local: return item.origin == .local
        }
    }
}

private enum SkillManagerInventorySort: String, CaseIterable, Identifiable {
    case name
    case source

    var id: String { rawValue }
    var title: String {
        switch self {
        case .name: return UIStrings.text("skillManager.inventory.sort.name", "Name")
        case .source: return UIStrings.text("skillManager.inventory.sort.source", "Source")
        }
    }

    func areInIncreasingOrder(_ lhs: SkillManagerInventoryItem, _ rhs: SkillManagerInventoryItem) -> Bool {
        switch self {
        case .name:
            let comparison = lhs.name.localizedCaseInsensitiveCompare(rhs.name)
            return comparison == .orderedSame ? lhs.id < rhs.id : comparison == .orderedAscending
        case .source:
            let left = lhs.localPath ?? lhs.source ?? lhs.name
            let right = rhs.localPath ?? rhs.source ?? rhs.name
            let comparison = left.localizedCaseInsensitiveCompare(right)
            return comparison == .orderedSame ? lhs.id < rhs.id : comparison == .orderedAscending
        }
    }
}

struct SkillManagerPanel: View {
    @EnvironmentObject private var store: SkillStore
    let showsHeader: Bool
    let entryContext: SkillManagerEntryContext

    @State private var selectedWorkflow: SkillManagerWorkflow = .searchInstall
    @State private var selectedSkill: SkillManagerSelection?
    @State private var selectedAction: SkillManagerEntryAction = .install
    @State private var actionScope: SkillManagerScope = .project
    @State private var actionAgentIDs = Set(SkillManagerAgent.defaultTargets.map(\.rawValue))
    @State private var pendingConfirmation: SkillManagerWriteConfirmation?
    @State private var isChoosingArchive = false
    @State private var isChoosingImportArchive = false
    @State private var inventoryQuery = ""
    @State private var inventorySourceFilter: SkillManagerInventorySourceFilter = .all
    @State private var inventoryAgentFilter = "all"
    @State private var inventorySort: SkillManagerInventorySort = .name
    @State private var appliedEntryContext: SkillManagerEntryContext?
    @State private var hasResolvedEntryTarget = false
    @State private var isApplyingEntryContext = false
    @FocusState private var focusedInput: SkillManagerEntryPresentation.FocusedInput?
    @AppStorage(DisplayText.screenshotPrivacyModeStorageKey) private var privacyModeEnabled = true

    init(
        showsHeader: Bool = true,
        entryContext: SkillManagerEntryContext = .default
    ) {
        let presentation = entryContext.presentation
        self.showsHeader = showsHeader
        self.entryContext = entryContext
        _selectedWorkflow = State(initialValue: presentation.workflow)
        _selectedAction = State(initialValue: presentation.preferredAction ?? .install)
        _actionScope = State(initialValue: presentation.scope)
        _actionAgentIDs = State(initialValue: presentation.agentIDs
            ?? Set(SkillManagerAgent.defaultTargets.map(\.rawValue)))
        _inventoryQuery = State(initialValue: presentation.inventoryQuery)
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 16) {
            if showsHeader {
                header
            } else {
                compactDataToolbar
            }

            WorkflowSheetSplitLayout(primaryMinWidth: 430, secondaryWidth: 380) {
                ScrollView {
                    VStack(alignment: .leading, spacing: 14) {
                        feedback
                        workflowPicker
                        if selectedWorkflow == .searchInstall {
                            searchSection
                        } else {
                            inventorySection
                        }
                    }
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .padding(.trailing, 14)
                }
            } secondary: {
                ScrollView {
                    VStack(alignment: .leading, spacing: 14) {
                        actionSection
                        previewSection
                    }
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .padding(.leading, 14)
                }
            }
        }
        .fileImporter(
            isPresented: $isChoosingArchive,
            allowedContentTypes: [.zip],
            allowsMultipleSelection: false,
            onCompletion: handleArchiveSelection
        )
        .fileImporter(
            isPresented: $isChoosingImportArchive,
            allowedContentTypes: [.zip],
            allowsMultipleSelection: false,
            onCompletion: handleImportArchiveSelection
        )
        .alert(confirmationTitle, isPresented: confirmationBinding) {
            if let confirmation = pendingConfirmation {
                Button(confirmation.confirmButtonTitle, role: confirmation.role) {
                    let confirmed = confirmation
                    pendingConfirmation = nil
                    Task { await applyConfirmed(confirmed) }
                }
                .disabled(!isCurrentConfirmation(confirmation))
            }
            Button(UIStrings.cancel, role: .cancel) { pendingConfirmation = nil }
        } message: {
            if let confirmation = pendingConfirmation { Text(confirmation.message) }
        }
        .onAppear {
            applyEntryContextIfNeeded()
        }
        .onChange(of: entryContext) { _ in
            appliedEntryContext = nil
            applyEntryContextIfNeeded()
        }
        .onChange(of: selectedWorkflow) { _ in
            selectedSkill = nil
            store.clearSkillManagerWorkflowPreviews()
            guard isApplyingEntryContext else { return }
            hasResolvedEntryTarget = false
            DispatchQueue.main.async {
                resolveEntryTargetIfAvailable(in: store.skillManagerInventoryItems)
            }
        }
        .onChange(of: selectedSkill) { selection in
            configureAction(for: selection)
            store.clearSkillManagerWorkflowPreviews()
        }
        .onChange(of: store.skillManagerScope) { _ in
            guard selectedWorkflow == .installedUpdates else { return }
            selectedSkill = nil
            guard isApplyingEntryContext else { return }
            hasResolvedEntryTarget = false
            DispatchQueue.main.async {
                resolveEntryTargetIfAvailable(in: store.skillManagerInventoryItems)
            }
        }
        .onChange(of: store.skillManagerInventoryItems) { items in
            guard selectedWorkflow == .installedUpdates else { return }
            if case .inventory(let selectedItem) = selectedSkill,
               let refreshed = items.first(where: { $0.id == selectedItem.id }) {
                selectedSkill = .inventory(refreshed)
                return
            }
            resolveEntryTargetIfAvailable(in: items)
        }
    }

    private var header: some View {
        HStack(alignment: .center, spacing: 14) {
            VStack(alignment: .leading, spacing: 4) {
                Label(
                    UIStrings.text("skillManager.title", "Skill Package Manager"),
                    systemImage: "shippingbox.and.arrow.backward"
                )
                .font(.title3.bold())
                Text(UIStrings.text(
                    "skillManager.skillFirst.help",
                    "Choose a skill first, then choose an action, scope, and affected agents."
                ))
                .font(.caption)
                .foregroundStyle(.secondary)
            }
            Spacer()
            VStack(alignment: .trailing, spacing: 3) {
                Text(startupSnapshotSummary)
                    .font(.caption)
                    .foregroundStyle(.secondary)
                loadDataButton
            }
        }
        .padding()
        .frame(maxWidth: .infinity, alignment: .leading)
        .nativePanelSurface()
    }

    private var compactDataToolbar: some View {
        HStack(spacing: 10) {
            Text(startupSnapshotSummary)
                .font(.caption)
                .foregroundStyle(.secondary)
            Spacer()
            loadDataButton
        }
        .padding(.horizontal, 16)
        .padding(.top, 12)
    }

    private var loadDataButton: some View {
        Button {
            Task { await store.refreshSkillManagerData() }
        } label: {
            Label {
                Text(UIStrings.text("skillManager.loadData", "Load Data"))
            } icon: {
                if isRefreshing {
                    ProgressView().controlSize(.small)
                } else {
                    Image(systemName: "arrow.clockwise")
                }
            }
        }
        .controlSize(.small)
        .disabled(isRefreshing)
        .help(UIStrings.text(
            "skillManager.loadData.help",
            "Reload project and global inventories. This page does not refresh automatically."
        ))
        .accessibilityIdentifier("skill-manager.load-data")
    }

    private var startupSnapshotSummary: String {
        let count = store.skillManagerInstalledByScope.values.reduce(0) { $0 + $1.installed.count }
        return String(
            format: UIStrings.text("skillManager.snapshot.count", "%d cached skills"),
            count
        )
    }

    private var isRefreshing: Bool {
        store.isLoadingSkillManagerTools || store.isListingSkillManagerInstalled
    }

    private var workflowPicker: some View {
        Picker(selection: $selectedWorkflow) {
            ForEach(SkillManagerWorkflow.allCases) { workflow in
                Label(workflow.title, systemImage: workflow.systemImage).tag(workflow)
            }
        } label: {
            Text(UIStrings.text("skillManager.workflow.accessibility", "Skill Manager workflow"))
        }
        .pickerStyle(.segmented)
        .labelsHidden()
    }

    @ViewBuilder
    private var feedback: some View {
        if let error = store.skillManagerErrorMessage {
            WorkflowSheetInlineBanner(message: error, style: .error)
        }
        if let warning = store.skillManagerWarningMessage {
            WorkflowSheetInlineBanner(message: warning, style: .warning)
        }
        if let message = store.skillManagerMessage {
            WorkflowSheetInlineBanner(message: message, style: .success)
        }
        if let message = externalManagerUnavailableMessage {
            WorkflowSheetInlineBanner(message: message, style: .warning)
        }
    }

    private var searchSection: some View {
        VStack(alignment: .leading, spacing: 12) {
            Label(UIStrings.text("skillManager.search", "Search Skills"), systemImage: "magnifyingglass")
                .font(.headline)
            HStack(spacing: 8) {
                TextField(
                    UIStrings.text("skillManager.query", "Search skills"),
                    text: $store.skillManagerSearchQuery
                )
                .textFieldStyle(.roundedBorder)
                .focused($focusedInput, equals: .search)
                .onSubmit { Task { await store.searchSkillManager() } }
                Button(UIStrings.text("skillManager.search.preview", "Preview Search")) {
                    Task { await store.searchSkillManager() }
                }
                .disabled(!canSearch)
            }

            if let result = store.skillManagerSearchResult {
                if result.results.isEmpty {
                    Text(UIStrings.text("skillManager.search.noResults", "No search results returned."))
                        .font(.caption)
                        .foregroundStyle(.secondary)
                } else {
                    LazyVStack(spacing: 8) {
                        ForEach(store.skillManagerVisibleSearchResults) { row in
                            SkillManagerSelectableRow(
                                title: row.value.name,
                                subtitle: row.value.description ?? row.value.source,
                                badge: UIStrings.text("skillManager.action.install", "Install"),
                                isSelected: selectedSkill == .search(row.value)
                            ) {
                                selectedSkill = .search(row.value)
                            }
                        }
                    }
                }
                if let status = store.skillManagerSearchStatus {
                    skillManagerSearchFooter(status, returnedCount: result.results.count)
                        .accessibilityIdentifier("skill-manager.search.completeness")
                }
            } else {
                Text(UIStrings.text(
                    "skillManager.search.prompt",
                    "Enter a keyword. Installation options appear after you select a result."
                ))
                .font(.caption)
                .foregroundStyle(.secondary)
            }

            Divider()

            localCreateSection

            Divider()

            HStack(alignment: .center, spacing: 12) {
                VStack(alignment: .leading, spacing: 3) {
                    Label(
                        UIStrings.text("skillManager.localImport.title", "Local Package"),
                        systemImage: "doc.zipper"
                    )
                    .font(.subheadline.bold())
                    Text(UIStrings.text(
                        "skillManager.localImport.help",
                        "Import one validated ZIP into the local library, then choose its install scope and agents."
                    ))
                    .font(.caption)
                    .foregroundStyle(.secondary)
                }
                Spacer()
                Button {
                    isChoosingImportArchive = true
                } label: {
                    Label(
                        UIStrings.text("skillManager.localImport.choose", "Choose Local ZIP"),
                        systemImage: "plus"
                    )
                }
                .disabled(store.isPreviewingSkillManagerLocalArchiveImport)
                .accessibilityIdentifier("skill-manager.local-import.choose")
            }
        }
        .padding()
        .frame(maxWidth: .infinity, alignment: .leading)
        .nativePanelSurface()
    }

    private var localCreateSection: some View {
        VStack(alignment: .leading, spacing: 8) {
            Label(
                UIStrings.text(
                    "skillManager.confirm.localCreate.title",
                    "Create Local Skill"
                ),
                systemImage: "doc.badge.plus"
            )
            .font(.subheadline.bold())
            HStack(spacing: 8) {
                TextField(
                    UIStrings.text(
                        "skillManager.localCreate.required",
                        "Enter a local skill name."
                    ),
                    text: $store.skillManagerLocalSkillName
                )
                .textFieldStyle(.roundedBorder)
                .focused($focusedInput, equals: .localCreate)
                .onSubmit {
                    guard canPreviewLocalCreate else { return }
                    Task { await store.previewSkillManagerLocalCreate() }
                }
                Button(UIStrings.text("skillManager.previewCreate", "Preview Create")) {
                    Task { await store.previewSkillManagerLocalCreate() }
                }
                .disabled(!canPreviewLocalCreate)
            }
        }
    }

    private func skillManagerSearchFooter(
        _ status: ListCompletenessState,
        returnedCount: Int
    ) -> some View {
        HStack {
            Text(String(
                format: UIStrings.text(
                    "skillManager.search.returned",
                    "%d returned · remote total unavailable"
                ),
                returnedCount
            ))
            .font(.caption2)
            .foregroundStyle(.secondary)
            Spacer()
            if status.canLoadMore {
                Button(UIStrings.text("action.loadMore", "Load More")) {
                    store.loadMoreSkillManagerSearchResults()
                }
                Button(UIStrings.text("action.showAll", "Show All")) {
                    store.showAllReturnedSkillManagerSearchResults()
                }
                .accessibilityIdentifier("skill-manager.search.show-all-returned")
            }
        }
        .controlSize(.small)
    }

    private var inventorySection: some View {
        VStack(alignment: .leading, spacing: 12) {
            HStack {
                Label(UIStrings.text("skillManager.inventory", "Skill Inventory"), systemImage: "list.bullet.rectangle")
                    .font(.headline)
                Spacer()
                Picker(UIStrings.scope, selection: $store.skillManagerScope) {
                    ForEach(SkillManagerScope.allCases) { scope in
                        Text(scope.title).tag(scope)
                    }
                }
                .pickerStyle(.segmented)
                .frame(width: 190)
            }

            HStack(spacing: 8) {
                TextField(
                    UIStrings.text("skillManager.inventory.search", "Filter installed skills"),
                    text: $inventoryQuery
                )
                .textFieldStyle(.roundedBorder)

                Picker(UIStrings.text("skillManager.inventory.source", "Source"), selection: $inventorySourceFilter) {
                    ForEach(SkillManagerInventorySourceFilter.allCases) { filter in
                        Text(filter.title).tag(filter)
                    }
                }
                .frame(width: 135)

                Picker(UIStrings.text("skillManager.inventory.agent", "Agent"), selection: $inventoryAgentFilter) {
                    Text(UIStrings.text("filter.allAgents", "All Agents")).tag("all")
                    ForEach(SkillManagerAgent.defaultTargets) { agent in
                        Text(agent.title).tag(agent.rawValue)
                    }
                }
                .frame(width: 145)

                Picker(UIStrings.text("skillManager.inventory.sort", "Sort"), selection: $inventorySort) {
                    ForEach(SkillManagerInventorySort.allCases) { sort in
                        Text(sort.title).tag(sort)
                    }
                }
                .frame(width: 125)
            }
            .controlSize(.small)

            if store.isListingSkillManagerInstalled && store.skillManagerInventoryItems.isEmpty {
                ProgressView(UIStrings.loading)
            } else if store.skillManagerInstalled == nil {
                Text(UIStrings.text(
                    "skillManager.inventory.notLoaded",
                    "Inventory has not been loaded for this scope. Use Load Data to fetch it."
                ))
                .font(.caption)
                .foregroundStyle(.secondary)
            } else if store.skillManagerInventoryItems.isEmpty {
                Text(UIStrings.text(
                    "skillManager.inventory.empty",
                    "The loaded inventory contains no skills for this scope."
                ))
                .font(.caption)
                .foregroundStyle(.secondary)
            } else if filteredInventoryItems.isEmpty {
                Text(UIStrings.text(
                    "skillManager.inventory.noMatches",
                    "No installed skills match the current search and filters."
                ))
                .font(.caption)
                .foregroundStyle(.secondary)
            } else {
                LazyVStack(spacing: 8) {
                    ForEach(filteredInventoryItems) { item in
                        SkillManagerSelectableRow(
                            title: item.name,
                            subtitle: inventorySubtitle(item),
                            badge: inventorySourceBadge(item),
                            isSelected: selectedSkill == .inventory(item)
                        ) {
                            selectedSkill = .inventory(item)
                        }
                    }
                }
            }
        }
        .padding()
        .frame(maxWidth: .infinity, alignment: .leading)
        .nativePanelSurface()
    }

    private func inventorySubtitle(_ item: SkillManagerInventoryItem) -> String {
        let agents = item.agents.isEmpty
            ? UIStrings.text("skillManager.agents.none", "No agent links")
            : item.agents.map(DisplayText.agent).joined(separator: ", ")
        let source = item.localPath ?? item.source
        let sourceText = source.map {
            DisplayText.privacyPath($0, privacyModeEnabled: privacyModeEnabled)
        }
        var parts = [item.scope.title, agents]
        if let sourceText {
            parts.append(sourceText)
        }
        return parts.joined(separator: " · ")
    }

    private var filteredInventoryItems: [SkillManagerInventoryItem] {
        let query = inventoryQuery.trimmingCharacters(in: .whitespacesAndNewlines).lowercased()
        return store.skillManagerInventoryItems
            .filter { item in
                inventorySourceFilter.includes(item)
                    && (inventoryAgentFilter == "all" || item.agents.contains(inventoryAgentFilter))
                    && (query.isEmpty || [item.name, item.source ?? "", item.localPath ?? ""]
                        .contains { $0.lowercased().contains(query) })
            }
            .sorted(by: inventorySort.areInIncreasingOrder)
    }

    private func inventorySourceBadge(_ item: SkillManagerInventoryItem) -> String {
        switch (item.origin, item.localOwnership) {
        case (.manager, _):
            return UIStrings.text("skillManager.source.manager", "npx managed")
        case (.local, .project):
            return UIStrings.text("skillManager.source.localProject", "Local · Project")
        case (.local, .global):
            return UIStrings.text("skillManager.source.localGlobal", "Local · Global")
        case (.local, .appOwned):
            return UIStrings.text("skillManager.source.localLibrary", "Local Library")
        case (.local, .external), (.local, nil):
            return UIStrings.text("skillManager.source.localExternal", "External local")
        }
    }

    @ViewBuilder
    private var actionSection: some View {
        VStack(alignment: .leading, spacing: 12) {
            Label(UIStrings.text("skillManager.action", "Action"), systemImage: "slider.horizontal.3")
                .font(.headline)

            if let selectedSkill {
                Text(selectedSkill.name).font(.headline)
                Text(selectedSkill.detail)
                    .font(.caption)
                    .foregroundStyle(.secondary)
                    .textSelection(.enabled)

                actionPicker(for: selectedSkill)
                actionOptions(for: selectedSkill)
                actionButton(for: selectedSkill)
            } else {
                VStack(spacing: 8) {
                    Image(systemName: "shippingbox")
                        .font(.title2)
                        .foregroundStyle(.secondary)
                    Text(UIStrings.text("skillManager.select.title", "Select a skill"))
                        .font(.headline)
                    Text(UIStrings.text(
                        "skillManager.select.help",
                        "Agent and scope options appear only after a skill is selected."
                    ))
                    .font(.caption)
                    .foregroundStyle(.secondary)
                    .multilineTextAlignment(.center)
                }
                .frame(maxWidth: .infinity)
                .padding(.vertical, 24)
            }
        }
        .padding()
        .frame(maxWidth: .infinity, alignment: .leading)
        .nativePanelSurface()
    }

    @ViewBuilder
    private func actionPicker(for selection: SkillManagerSelection) -> some View {
        let actions = availableActions(for: selection)
        if actions.count > 1 {
            Picker(UIStrings.text("skillManager.action", "Action"), selection: $selectedAction) {
                ForEach(actions) { action in
                    Text(action.title).tag(action)
                }
            }
            .pickerStyle(.segmented)
            .onChange(of: selectedAction) { _ in store.clearSkillManagerWorkflowPreviews() }
        }
    }

    @ViewBuilder
    private func actionOptions(for selection: SkillManagerSelection) -> some View {
        switch selectedAction {
        case .install:
            Picker(UIStrings.scope, selection: $actionScope) {
                ForEach(SkillManagerScope.allCases) { Text($0.title).tag($0) }
            }
            .pickerStyle(.segmented)
            agentPicker(available: SkillManagerAgent.defaultTargets.map(\.rawValue))
        case .remove:
            if case .inventory(let item) = selection {
                agentPicker(available: item.agents)
                Text(UIStrings.text(
                    "skillManager.remove.sourceRule",
                    "Removing every linked agent lets the manager remove its unreferenced source package; a partial removal deletes only those links."
                ))
                .font(.caption)
                .foregroundStyle(.secondary)
            }
        case .update:
            if case .inventory(let item) = selection {
                if item.origin == .local {
                    Text(UIStrings.text(
                        "skillManager.localUpdate.help",
                        "Choose a replacement ZIP for this local source. The package name must match; replacement is previewed with rollback and scripts are never executed."
                    ))
                    .font(.caption)
                    .foregroundStyle(.secondary)
                    if let path = item.localPath ?? item.source {
                        MetadataLine(
                            label: UIStrings.text("skillManager.localUpdate.target", "Local source"),
                            value: path
                        )
                    }
                } else {
                    Text(String(
                        format: UIStrings.text(
                            "skillManager.update.affected",
                            "The manager updates the shared source used by: %@"
                        ),
                        item.agents.map(DisplayText.agent).joined(separator: ", ")
                    ))
                    .font(.caption)
                    .foregroundStyle(.secondary)
                }
            }
        case .deleteSource:
            Text(UIStrings.text(
                "skillManager.deleteSource.help",
                "The app-owned local source has no agent links and can be deleted after confirmation."
            ))
            .font(.caption)
            .foregroundStyle(.secondary)
        }
    }

    private func agentPicker(available: [String]) -> some View {
        VStack(alignment: .leading, spacing: 8) {
            HStack {
                Text(UIStrings.text("skillManager.targets", "Agents")).font(.caption.bold())
                Spacer()
                Button(UIStrings.text("selection.all", "All")) {
                    actionAgentIDs = Set(available)
                    store.clearSkillManagerWorkflowPreviews()
                }
                Button(UIStrings.text("selection.none", "None")) {
                    actionAgentIDs = []
                    store.clearSkillManagerWorkflowPreviews()
                }
                .disabled(actionAgentIDs.isEmpty)
            }
            LazyVGrid(columns: [GridItem(.adaptive(minimum: 130), spacing: 8)], alignment: .leading, spacing: 8) {
                ForEach(SkillManagerAgent.defaultTargets.filter { available.contains($0.rawValue) }) { agent in
                    Toggle(isOn: Binding(
                        get: { actionAgentIDs.contains(agent.rawValue) },
                        set: { selected in
                            if selected { actionAgentIDs.insert(agent.rawValue) }
                            else { actionAgentIDs.remove(agent.rawValue) }
                            store.clearSkillManagerWorkflowPreviews()
                        }
                    )) {
                        Text(agent.title).lineLimit(1)
                    }
                    .toggleStyle(.checkbox)
                    .controlSize(.small)
                }
            }
        }
    }

    @ViewBuilder
    private func actionButton(for selection: SkillManagerSelection) -> some View {
        switch selectedAction {
        case .install:
            Button {
                Task {
                    await store.previewSkillManagerInstall(
                        source: selection.source,
                        skillName: selection.name,
                        agents: selectedActionAgents,
                        scope: actionScope
                    )
                }
            } label: {
                Label(UIStrings.text("skillManager.previewInstall", "Preview Install"), systemImage: "plus.circle")
            }
            .disabled(selectedActionAgents.isEmpty || externalMutationDisabled || store.isPreviewingSkillManagerMutation)
        case .remove:
            Button(role: .destructive) {
                Task {
                    let removesEveryAgent = Set(selectedActionAgents) == Set(selection.agents)
                    let cleanupLocalInstanceID = selection.localOwnership == .appOwned && removesEveryAgent
                        ? selection.localInstanceID
                        : nil
                    await store.previewSkillManagerRemove(
                        skillName: selection.name,
                        agents: selectedActionAgents,
                        scope: selection.scope,
                        cleanupLocalInstanceID: cleanupLocalInstanceID
                    )
                }
            } label: {
                Label(UIStrings.text("skillManager.previewRemove", "Preview Remove"), systemImage: "minus.circle")
            }
            .disabled(selectedActionAgents.isEmpty || externalMutationDisabled || store.isPreviewingSkillManagerMutation)
        case .update:
            if selection.isLocal {
                Button {
                    isChoosingArchive = true
                } label: {
                    Label(UIStrings.text("skillManager.chooseZip", "Choose ZIP & Preview"), systemImage: "doc.zipper")
                }
                .disabled(selection.localInstanceID == nil || store.isPreviewingSkillManagerLocalArchiveUpdate)
            } else {
                Button {
                    Task {
                        await store.previewSkillManagerUpdate(
                            skillName: selection.name,
                            affectedAgents: selection.agents,
                            scope: selection.scope
                        )
                    }
                } label: {
                    Label(UIStrings.text("skillManager.previewUpdate", "Preview Update"), systemImage: "arrow.triangle.2.circlepath")
                }
                .disabled(externalMutationDisabled || store.isPreviewingSkillManagerMutation)
            }
        case .deleteSource:
            Button(role: .destructive) {
                guard let instanceID = selection.localInstanceID,
                      let localSkill = store.localSkillLibrarySkills.first(where: { $0.id == instanceID }) else { return }
                Task { await store.previewSkillManagerLocalDelete(skill: localSkill) }
            } label: {
                Label(UIStrings.text("skillManager.previewDelete", "Preview Delete"), systemImage: "trash")
            }
            .disabled(store.isPreviewingSkillManagerLocalDelete)
        }
    }

    @ViewBuilder
    private var previewSection: some View {
        if let confirmation = store.skillManagerSearchConfirmation {
            previewCard(title: confirmation.result.preview.localizedSummary) {
                commandPreview(confirmation.result.preview)
                Button(UIStrings.text("skillManager.search.run", "Run Search")) {
                    pendingConfirmation = .search(confirmation)
                }
                .buttonStyle(.borderedProminent)
                .disabled(store.isApplyingSkillManagerMutation)
            }
        }
        if let confirmation = store.skillManagerMutationConfirmation {
            previewCard(title: confirmation.result.preview.localizedSummary) {
                commandPreview(confirmation.result.preview)
                if let output = confirmation.result.output { commandOutput(output) }
                Button(applyTitle(for: confirmation.inputs.kind)) {
                    pendingConfirmation = .mutation(confirmation)
                }
                .buttonStyle(.borderedProminent)
                .disabled(store.isApplyingSkillManagerMutation)
            }
        }
        if let confirmation = store.skillManagerLocalCreateConfirmation {
            previewCard(title: confirmation.result.preview.localizedSummary) {
                commandPreview(confirmation.result.preview)
                if let output = confirmation.result.output { commandOutput(output) }
                Button(UIStrings.text(
                    "skillManager.confirm.localCreate.title",
                    "Create Local Skill"
                )) {
                    pendingConfirmation = .localCreate(confirmation)
                }
                .buttonStyle(.borderedProminent)
                .disabled(store.isApplyingSkillManagerMutation)
            }
        }
        if let confirmation = store.skillManagerLocalArchiveImportConfirmation {
            previewCard(title: confirmation.result.summary) {
                MetadataLine(label: UIStrings.text("metadata.skill", "Skill"), value: confirmation.result.skillName)
                MetadataLine(
                    label: UIStrings.text("skillManager.archive.files", "Archive files"),
                    value: "\(confirmation.result.fileCount)"
                )
                MetadataLine(
                    label: UIStrings.text("skillManager.archive.bytes", "Uncompressed bytes"),
                    value: "\(confirmation.result.uncompressedBytes)"
                )
                Text(UIStrings.text(
                    "skillManager.localImport.nextStep",
                    "After import, select this local skill in Installed & Updates to choose project/global scope and target agents."
                ))
                .font(.caption)
                .foregroundStyle(.secondary)
                Button(UIStrings.text("skillManager.localImport.apply", "Import Local Package")) {
                    pendingConfirmation = .localArchiveImport(confirmation)
                }
                .buttonStyle(.borderedProminent)
            }
        }
        if let confirmation = store.skillManagerLocalArchiveUpdateConfirmation {
            previewCard(title: confirmation.result.summary) {
                MetadataLine(label: UIStrings.text("metadata.skill", "Skill"), value: confirmation.result.skillName)
                MetadataLine(
                    label: UIStrings.text("skillManager.archive.files", "Archive files"),
                    value: "\(confirmation.result.fileCount)"
                )
                MetadataLine(
                    label: UIStrings.text("skillManager.archive.bytes", "Uncompressed bytes"),
                    value: "\(confirmation.result.uncompressedBytes)"
                )
                Button(UIStrings.text("skillManager.applyUpdate", "Update")) {
                    pendingConfirmation = .localArchiveUpdate(confirmation)
                }
                .buttonStyle(.borderedProminent)
            }
        }
        if let confirmation = store.skillManagerLocalDeleteConfirmation {
            previewCard(title: confirmation.result.summary) {
                MetadataLine(label: UIStrings.text("metadata.skill", "Skill"), value: confirmation.result.skillName)
                MetadataLine(label: UIStrings.source, value: confirmation.result.path)
                Button(UIStrings.text("action.delete", "Delete"), role: .destructive) {
                    pendingConfirmation = .localDelete(confirmation)
                }
                .disabled(!confirmation.result.physicalDeleteAllowed)
            }
        }
    }

    private func previewCard<Content: View>(
        title: String,
        @ViewBuilder content: () -> Content
    ) -> some View {
        VStack(alignment: .leading, spacing: 10) {
            Label(UIStrings.text("skillManager.preview", "Preview"), systemImage: "doc.text.magnifyingglass")
                .font(.headline)
            Text(title).font(.callout)
            content()
        }
        .padding()
        .frame(maxWidth: .infinity, alignment: .leading)
        .nativePanelSurface()
    }

    private func commandPreview(_ preview: SkillManagerCommandPreview) -> some View {
        VStack(alignment: .leading, spacing: 8) {
            Text(preview.displayCommand)
                .font(.system(.caption, design: .monospaced))
                .textSelection(.enabled)
                .padding(8)
                .frame(maxWidth: .infinity, alignment: .leading)
                .background(Color.agentCopilotPanelBackground, in: RoundedRectangle(cornerRadius: 8))
            MetadataLine(label: "CWD", value: preview.cwd)
            if !preview.risks.isEmpty {
                VStack(alignment: .leading, spacing: 4) {
                    ForEach(preview.risks, id: \.self) { risk in
                        Label(risk, systemImage: "exclamationmark.triangle")
                            .font(.caption)
                            .foregroundStyle(.secondary)
                    }
                }
            }
        }
    }

    private func commandOutput(_ output: SkillManagerCommandOutput) -> some View {
        VStack(alignment: .leading, spacing: 4) {
            if !output.stdout.isEmpty { Text(output.stdout) }
            if !output.stderr.isEmpty { Text(output.stderr).foregroundStyle(.orange) }
        }
        .font(.system(.caption, design: .monospaced))
        .lineLimit(6)
        .textSelection(.enabled)
    }

    private func configureAction(for selection: SkillManagerSelection?) {
        guard let selection else { return }
        let presentation = entryContext.presentation
        let isEntryTarget = selection.matches(entryContext.target)
        actionScope = isEntryTarget ? presentation.scope : selection.scope
        let defaultAgents = selection.agents.isEmpty
            ? SkillManagerAgent.defaultTargets.map(\.rawValue)
            : selection.agents
        let requestedAgents = isEntryTarget ? presentation.agentIDs : nil
        let resolvedAction = (isEntryTarget
            ? presentation.resolvedAction(available: availableActions(for: selection))
            : availableActions(for: selection).first) ?? .install
        selectedAction = resolvedAction
        let eligibleAgents = resolvedAction == .remove
            ? Set(selection.agents)
            : Set(defaultAgents)
        actionAgentIDs = requestedAgents.map { $0.intersection(eligibleAgents) }
            ?? eligibleAgents
    }

    private func availableActions(for selection: SkillManagerSelection) -> [SkillManagerEntryAction] {
        switch selection {
        case .search:
            return [.install]
        case .inventory(let item):
            if item.agents.isEmpty {
                return item.origin == .local ? [.install, .deleteSource] : [.install]
            }
            if item.origin == .local, item.localInstanceID == nil {
                return [.remove]
            }
            return [.update, .remove]
        }
    }

    private var selectedActionAgents: [String] {
        SkillManagerAgent.defaultTargets.map(\.rawValue).filter(actionAgentIDs.contains)
    }

    private var canSearch: Bool {
        !store.skillManagerSearchQuery.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
            && !store.isSearchingSkillManager
            && !externalMutationDisabled
    }

    private var canPreviewLocalCreate: Bool {
        !store.skillManagerLocalSkillName
            .trimmingCharacters(in: .whitespacesAndNewlines)
            .isEmpty
            && !store.isPreviewingSkillManagerLocalCreate
            && !externalMutationDisabled
    }

    private func applyEntryContextIfNeeded() {
        guard appliedEntryContext != entryContext else { return }
        let presentation = entryContext.presentation
        appliedEntryContext = entryContext
        isApplyingEntryContext = true
        hasResolvedEntryTarget = false
        selectedWorkflow = presentation.workflow
        selectedSkill = nil
        selectedAction = presentation.preferredAction ?? .install
        actionScope = presentation.scope
        if let agentIDs = presentation.agentIDs {
            actionAgentIDs = agentIDs
        }
        inventoryQuery = presentation.inventoryQuery
        if presentation.workflow == .installedUpdates {
            store.skillManagerScope = presentation.scope
            resolveEntryTargetIfAvailable(in: store.skillManagerInventoryItems)
        }
        if let searchQuery = presentation.searchQuery {
            store.skillManagerSearchQuery = searchQuery
        }
        if let suggestedName = presentation.suggestedLocalSkillName {
            store.skillManagerLocalSkillName = suggestedName
        }
        DispatchQueue.main.async {
            guard appliedEntryContext == entryContext else { return }
            focusedInput = presentation.focusedInput
            if presentation.requestsImportArchive {
                isChoosingImportArchive = true
            }
            if presentation.workflow == .installedUpdates {
                resolveEntryTargetIfAvailable(in: store.skillManagerInventoryItems)
            }
            isApplyingEntryContext = false
        }
    }

    private func resolveEntryTargetIfAvailable(
        in items: [SkillManagerInventoryItem]
    ) {
        guard !hasResolvedEntryTarget,
              let target = entryContext.target,
              let item = target.uniqueBestMatch(in: items) else {
            return
        }
        hasResolvedEntryTarget = true
        selectedSkill = .inventory(item)
    }

    private func handleArchiveSelection(_ result: Result<[URL], Error>) {
        do {
            guard let url = try result.get().first,
                  let instanceID = selectedSkill?.localInstanceID else { return }
            Task {
                let didAccess = url.startAccessingSecurityScopedResource()
                defer { if didAccess { url.stopAccessingSecurityScopedResource() } }
                await store.previewSkillManagerLocalArchiveUpdate(
                    instanceID: instanceID,
                    archivePath: url.path
                )
            }
        } catch {
            // Cancellation needs no banner; validation errors are returned by the service preview.
        }
    }

    private func handleImportArchiveSelection(_ result: Result<[URL], Error>) {
        do {
            guard let url = try result.get().first else { return }
            Task {
                let didAccess = url.startAccessingSecurityScopedResource()
                defer { if didAccess { url.stopAccessingSecurityScopedResource() } }
                await store.previewSkillManagerLocalArchiveImport(archivePath: url.path)
            }
        } catch {
            // File picker cancellation needs no banner; service preview reports validation errors.
        }
    }

    private func applyTitle(for kind: SkillManagerMutationInputs.Kind) -> String {
        switch kind {
        case .install: return UIStrings.text("skillManager.applyInstall", "Install")
        case .remove: return UIStrings.text("skillManager.applyRemove", "Remove")
        case .update: return UIStrings.text("skillManager.applyUpdate", "Update")
        }
    }

    private var confirmationBinding: Binding<Bool> {
        Binding(
            get: { pendingConfirmation != nil },
            set: { if !$0 { pendingConfirmation = nil } }
        )
    }

    private var confirmationTitle: String {
        pendingConfirmation?.title ?? UIStrings.text("skillManager.confirm.title", "Confirm Skill Manager Operation")
    }

    private func applyConfirmed(_ confirmation: SkillManagerWriteConfirmation) async {
        switch confirmation {
        case .search(let value):
            await store.applySkillManagerSearch(confirmation: value)
        case .mutation(let value):
            switch value.inputs.kind {
            case .install: await store.applySkillManagerInstall(confirmation: value)
            case .remove: await store.applySkillManagerRemove(confirmation: value)
            case .update: await store.applySkillManagerUpdate(confirmation: value)
            }
        case .localCreate(let value):
            await store.applySkillManagerLocalCreate(confirmation: value)
        case .localDelete(let value):
            await store.applySkillManagerLocalDelete(confirmation: value)
        case .localArchiveImport(let value):
            await store.applySkillManagerLocalArchiveImport(confirmation: value)
        case .localArchiveUpdate(let value):
            await store.applySkillManagerLocalArchiveUpdate(confirmation: value)
        }
    }

    private func isCurrentConfirmation(_ confirmation: SkillManagerWriteConfirmation) -> Bool {
        switch confirmation {
        case .search(let value): return store.skillManagerSearchConfirmation == value
        case .mutation(let value): return store.skillManagerMutationConfirmation == value
        case .localCreate(let value): return store.skillManagerLocalCreateConfirmation == value
        case .localDelete(let value): return store.skillManagerLocalDeleteConfirmation == value
        case .localArchiveImport(let value): return store.skillManagerLocalArchiveImportConfirmation == value
        case .localArchiveUpdate(let value): return store.skillManagerLocalArchiveUpdateConfirmation == value
        }
    }

    private var primaryTool: SkillManagerToolRecord? {
        store.skillManagerTools.first { $0.id == "npx-skills" } ?? store.skillManagerTools.first
    }

    private var externalMutationDisabled: Bool { externalManagerUnavailableMessage != nil }

    private var externalManagerUnavailableMessage: String? {
        guard !store.isLoadingSkillManagerTools else { return nil }
        guard let tool = primaryTool else {
            return store.skillManagerTools.isEmpty && store.skillManagerInstalledByScope.isEmpty
                ? nil
                : UIStrings.text(
                    "skillManager.toolUnavailable.message",
                    "The external manager is unavailable. Install Node/npm or set SKILLS_COPILOT_NPX_PATH, then load data again."
                )
        }
        let status = tool.status.lowercased()
        guard tool.executable == nil || status.contains("unavailable") || status.contains("error") || status.contains("missing") else {
            return nil
        }
        return UIStrings.text(
            "skillManager.toolUnavailable.message",
            "The external manager is unavailable. Install Node/npm or set SKILLS_COPILOT_NPX_PATH, then load data again."
        )
    }
}

private enum SkillManagerSelection: Hashable {
    case search(SkillManagerSearchResult)
    case inventory(SkillManagerInventoryItem)

    var name: String {
        switch self {
        case .search(let value): return value.name
        case .inventory(let value): return value.name
        }
    }

    var source: String {
        switch self {
        case .search(let value): return value.source ?? value.name
        case .inventory(let value): return value.localPath ?? value.source ?? value.name
        }
    }

    var detail: String {
        switch self {
        case .search(let value): return value.description ?? value.source ?? value.name
        case .inventory(let value): return value.source ?? value.localPath ?? value.scope.title
        }
    }

    var scope: SkillManagerScope {
        switch self {
        case .search: return .project
        case .inventory(let value): return value.scope
        }
    }

    var agents: [String] {
        switch self {
        case .search: return []
        case .inventory(let value): return value.agents
        }
    }

    var isLocal: Bool {
        if case .inventory(let value) = self { return value.origin == .local }
        return false
    }

    var localInstanceID: String? {
        if case .inventory(let value) = self { return value.localInstanceID }
        return nil
    }

    var localOwnership: SkillManagerInventoryItem.LocalOwnership? {
        if case .inventory(let value) = self { return value.localOwnership }
        return nil
    }

    func matches(_ target: SkillManagerPackageTarget?) -> Bool {
        guard let target else { return false }
        if case .inventory(let value) = self {
            return target.matches(value)
        }
        return false
    }
}

private extension SkillManagerEntryAction {
    var title: String {
        switch self {
        case .install: return UIStrings.text("skillManager.action.install", "Install")
        case .update: return UIStrings.text("skillManager.action.update", "Update")
        case .remove: return UIStrings.text("skillManager.action.remove", "Remove")
        case .deleteSource: return UIStrings.text("skillManager.action.deleteSource", "Delete Source")
        }
    }
}

private enum SkillManagerWriteConfirmation {
    case search(SkillManagerSearchConfirmation)
    case mutation(SkillManagerMutationConfirmation)
    case localCreate(SkillManagerLocalCreateConfirmation)
    case localDelete(SkillManagerLocalDeleteConfirmation)
    case localArchiveImport(SkillManagerLocalArchiveImportConfirmation)
    case localArchiveUpdate(SkillManagerLocalArchiveUpdateConfirmation)

    var title: String {
        switch self {
        case .search:
            return UIStrings.text("skillManager.confirm.search.title", "Confirm Remote Skill Search")
        case .mutation(let value):
            switch value.inputs.kind {
            case .install: return UIStrings.text("skillManager.confirm.install.title", "Confirm Skill Install")
            case .remove: return UIStrings.text("skillManager.confirm.remove.title", "Confirm Skill Removal")
            case .update: return UIStrings.text("skillManager.confirm.update.title", "Confirm Skill Update")
            }
        case .localCreate:
            return UIStrings.text("skillManager.confirm.localCreate.title", "Confirm Local Skill Creation")
        case .localDelete:
            return UIStrings.text("skillManager.confirm.localDelete.title", "Confirm Local Skill Delete")
        case .localArchiveImport:
            return UIStrings.text("skillManager.confirm.localImport.title", "Confirm Local ZIP Import")
        case .localArchiveUpdate:
            return UIStrings.text("skillManager.confirm.localArchive.title", "Confirm Local ZIP Update")
        }
    }

    var confirmButtonTitle: String {
        switch self {
        case .search: return UIStrings.text("skillManager.search.run", "Run Search")
        case .mutation(let value):
            switch value.inputs.kind {
            case .install: return UIStrings.text("skillManager.applyInstall", "Install")
            case .remove: return UIStrings.text("skillManager.applyRemove", "Remove")
            case .update: return UIStrings.text("skillManager.applyUpdate", "Update")
            }
        case .localCreate:
            return UIStrings.text(
                "skillManager.confirm.localCreate.title",
                "Create Local Skill"
            )
        case .localDelete: return UIStrings.text("action.delete", "Delete")
        case .localArchiveImport: return UIStrings.text("skillManager.localImport.apply", "Import Local Package")
        case .localArchiveUpdate: return UIStrings.text("skillManager.applyUpdate", "Update")
        }
    }

    var role: ButtonRole? {
        switch self {
        case .mutation(let value) where value.inputs.kind == .remove: return .destructive
        case .localDelete: return .destructive
        default: return nil
        }
    }

    var message: String {
        switch self {
        case .search(let value):
            return [
                value.result.preview.summary,
                "\(UIStrings.text("skillManager.query", "Search skills")): \(value.query)",
                "\(UIStrings.text("skillManager.confirm.command", "Command")): \(value.result.preview.displayCommand)",
                "CWD: \(value.result.preview.cwd)",
                value.result.preview.action?.confirmationSummary.disclosureText
            ]
            .compactMap { $0 }
            .joined(separator: "\n\n")
        case .mutation(let value):
            let targets = value.inputs.kind == .update
                ? value.inputs.agents.map(DisplayText.agent).joined(separator: ", ")
                : value.inputs.agents.map(DisplayText.agent).joined(separator: ", ")
            var sections = [
                value.result.preview.summary,
                "\(UIStrings.text("metadata.skill", "Skill")): \(value.inputs.skills.joined(separator: ", "))",
                "\(UIStrings.scope): \(value.inputs.scope.title)",
                "\(UIStrings.text("skillManager.confirm.targets", "Affected agents")): \(targets)",
                "\(UIStrings.text("skillManager.confirm.command", "Command")): \(value.result.preview.displayCommand)"
            ]
            if value.inputs.cleanupLocalInstanceID != nil {
                sections.append(UIStrings.text(
                    "skillManager.confirm.fullUninstall",
                    "After every selected agent link is removed, the app-owned local source and its catalog record will also be deleted if the service confirms there are no remaining references."
                ))
            }
            if let disclosure = value.result.preview.action?
                .confirmationSummary
                .disclosureText {
                sections.append(disclosure)
            }
            return sections.joined(separator: "\n\n")
        case .localCreate(let value):
            return [
                value.result.preview.summary,
                "\(UIStrings.text("metadata.skill", "Skill")): \(value.name)",
                "\(UIStrings.text("skillManager.confirm.command", "Command")): \(value.result.preview.displayCommand)",
                "CWD: \(value.result.preview.cwd)",
                value.result.preview.action?.confirmationSummary.disclosureText
            ]
            .compactMap { $0 }
            .joined(separator: "\n\n")
        case .localDelete(let value):
            return [
                value.result.summary,
                value.result.skillName,
                value.result.path,
                value.result.action?.confirmationSummary.disclosureText
            ]
            .compactMap { $0 }
            .joined(separator: "\n\n")
        case .localArchiveImport(let value):
            return [
                value.result.summary,
                value.result.skillName,
                value.result.archivePath,
                "SHA-256: \(value.result.archiveSha256)",
                value.result.action.confirmationSummary.disclosureText
            ].joined(separator: "\n\n")
        case .localArchiveUpdate(let value):
            return [
                value.result.summary,
                value.result.skillName,
                value.result.archivePath,
                "SHA-256: \(value.result.archiveSha256)",
                value.result.action.confirmationSummary.disclosureText
            ].joined(separator: "\n\n")
        }
    }
}

private struct SkillManagerSelectableRow: View {
    let title: String
    let subtitle: String?
    let badge: String
    let isSelected: Bool
    let action: () -> Void

    var body: some View {
        Button(action: action) {
            HStack(spacing: 10) {
                VStack(alignment: .leading, spacing: 3) {
                    Text(title).font(.callout.bold()).foregroundStyle(.primary).lineLimit(1)
                    if let subtitle, !subtitle.isEmpty {
                        Text(subtitle).font(.caption).foregroundStyle(.secondary).lineLimit(2)
                    }
                }
                Spacer()
                Text(badge)
                    .font(.caption2.bold())
                    .foregroundStyle(isSelected ? Color.white : Color.accentColor)
                    .padding(.horizontal, 7)
                    .padding(.vertical, 3)
                    .background(isSelected ? Color.accentColor : Color.accentColor.opacity(0.12), in: Capsule())
                Image(systemName: "chevron.right").font(.caption).foregroundStyle(.secondary)
            }
            .padding(10)
            .background(
                isSelected ? Color.accentColor.opacity(0.12) : Color.agentCopilotPanelBackground,
                in: RoundedRectangle(cornerRadius: 8)
            )
            .overlay {
                RoundedRectangle(cornerRadius: 8)
                    .stroke(isSelected ? Color.accentColor.opacity(0.6) : Color.clear, lineWidth: 1)
            }
        }
        .buttonStyle(.plain)
        .accessibilityLabel(title)
        .accessibilityValue([subtitle, badge].compactMap { $0 }.joined(separator: ", "))
    }
}
