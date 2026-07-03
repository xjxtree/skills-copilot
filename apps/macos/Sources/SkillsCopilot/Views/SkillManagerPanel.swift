import SwiftUI

struct SkillManagerPanel: View {
    @EnvironmentObject private var store: SkillStore
    var showsHeader = true
    @State private var selectedWorkflow: SkillManagerWorkflow = .searchInstall
    @State private var pendingConfirmation: SkillManagerWriteConfirmation?
    @State private var isShowingSkillManagerTargets = false

    var body: some View {
        VStack(alignment: .leading, spacing: 16) {
            if showsHeader {
                header
            }

            WorkflowSheetSplitLayout(primaryMinWidth: 430, secondaryWidth: 360) {
                ScrollView {
                    VStack(alignment: .leading, spacing: 14) {
                        skillManagerFeedback
                        targetControls
                        workflowPicker
                        if selectedWorkflow.allowsExternalManagerMutation, let message = externalManagerUnavailableMessage {
                            toolUnavailableCard(message)
                        }
                        workflowInputContent
                    }
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .padding(.trailing, 14)
                }
            } secondary: {
                ScrollView {
                    VStack(alignment: .leading, spacing: 14) {
                        workflowResultsContent
                        workflowPreview
                    }
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .padding(.leading, 14)
                }
            }
        }
        .task {
            if store.skillManagerTools.isEmpty {
                await store.loadSkillManagerTools()
            }
            if store.skillManagerInstalled == nil {
                await store.listSkillManagerInstalled()
            }
        }
        .alert(confirmationTitle, isPresented: confirmationBinding) {
            if let confirmation = pendingConfirmation {
                Button(confirmation.confirmButtonTitle, role: confirmation.role) {
                    let confirmed = confirmation
                    pendingConfirmation = nil
                    Task { await applyConfirmed(confirmed) }
                }
            }
            Button(UIStrings.cancel, role: .cancel) {
                pendingConfirmation = nil
            }
        } message: {
            if let confirmation = pendingConfirmation {
                Text(confirmation.message)
            }
        }
        .onChange(of: selectedWorkflow) { _ in
            store.clearSkillManagerWorkflowPreviews()
        }
    }

    private var header: some View {
        VStack(alignment: .leading, spacing: 12) {
            HStack(alignment: .firstTextBaseline) {
                Label(UIStrings.text("skillManager.title", "Skill Package Manager"), systemImage: "shippingbox.and.arrow.backward")
                    .font(.title3.bold())
                Spacer()
                Button {
                    Task {
                        await store.loadSkillManagerTools()
                        await store.listSkillManagerInstalled()
                    }
                } label: {
                    Label {
                        Text(UIStrings.text("action.refresh", "Refresh"))
                    } icon: {
                        if isRefreshing {
                            ProgressView()
                                .controlSize(.small)
                        } else {
                            Image(systemName: "arrow.clockwise")
                        }
                    }
                }
                .controlSize(.small)
                .disabled(isRefreshing)
            }

            DetailMetricGrid(maxColumns: 4, minColumnWidth: 150) {
                SummaryChip(
                    title: UIStrings.text("skillManager.defaultTool", "Tool"),
                    value: primaryTool?.displayName ?? UIStrings.notLoaded,
                    systemImage: "terminal"
                )
                SummaryChip(
                    title: UIStrings.text("skillManager.toolStatus", "Status"),
                    value: primaryTool?.status ?? UIStrings.notLoaded,
                    systemImage: "checkmark.seal"
                )
                SummaryChip(
                    title: UIStrings.text("skillManager.targets", "Targets"),
                    value: "\(store.skillManagerSelectedAgents.count)",
                    systemImage: "person.3"
                )
                SummaryChip(
                    title: UIStrings.text("skillManager.localLibrary", "Local Library"),
                    value: "\(store.localSkillLibrarySkills.count)",
                    systemImage: "folder"
                )
            }
        }
        .padding()
        .frame(maxWidth: .infinity, alignment: .leading)
        .nativePanelSurface()
    }

    private var isRefreshing: Bool {
        store.isLoadingSkillManagerTools || store.isListingSkillManagerInstalled
    }

    private var selectedTargetAgents: [SkillManagerAgent] {
        SkillManagerAgent.defaultTargets.filter { agent in
            store.skillManagerSelectedAgentIDs.contains(agent.rawValue)
        }
    }

    private var targetControls: some View {
        VStack(alignment: .leading, spacing: 12) {
            HStack(spacing: 10) {
                Label(UIStrings.text("skillManager.targets", "Targets"), systemImage: "person.3")
                    .font(.headline)
                SkillManagerTargetSummary(agents: selectedTargetAgents)
                Spacer()
                Button(UIStrings.text("selection.all", "All")) {
                    store.selectAllSkillManagerAgents()
                }
                .controlSize(.small)
                Button(UIStrings.text("selection.none", "None")) {
                    store.clearSkillManagerAgents()
                }
                .controlSize(.small)
                Button {
                    withAnimation(.easeInOut(duration: 0.12)) {
                        isShowingSkillManagerTargets.toggle()
                    }
                } label: {
                    Image(systemName: isShowingSkillManagerTargets ? "chevron.up" : "chevron.down")
                        .frame(width: 22, height: 22)
                }
                .buttonStyle(.plain)
                .accessibilityLabel(UIStrings.text("skillManager.targets.toggle", "Show target agents"))
                .accessibilityValue(isShowingSkillManagerTargets ? UIStrings.text("state.expanded", "Expanded") : UIStrings.text("state.collapsed", "Collapsed"))
            }

            if isShowingSkillManagerTargets {
                LazyVGrid(columns: [GridItem(.adaptive(minimum: 126), spacing: 8)], alignment: .leading, spacing: 8) {
                    ForEach(SkillManagerAgent.defaultTargets) { agent in
                        Toggle(isOn: Binding(
                            get: { store.skillManagerSelectedAgentIDs.contains(agent.rawValue) },
                            set: { store.setSkillManagerAgent(agent.rawValue, selected: $0) }
                        )) {
                            Text(agent.title)
                                .lineLimit(1)
                        }
                        .toggleStyle(.checkbox)
                        .controlSize(.small)
                    }
                }
                .transition(.opacity.combined(with: .move(edge: .top)))
            }

            HStack(spacing: 12) {
                Picker(UIStrings.scope, selection: $store.skillManagerScope) {
                    ForEach(SkillManagerScope.allCases) { scope in
                        Text(scope.title).tag(scope)
                    }
                }
                .pickerStyle(.segmented)
                .frame(maxWidth: 220)

                Picker(UIStrings.text("skillManager.distribution", "Distribution"), selection: $store.skillManagerDistribution) {
                    ForEach(SkillManagerDistribution.allCases) { distribution in
                        Text(distribution.title).tag(distribution)
                    }
                }
                .pickerStyle(.segmented)
                .frame(maxWidth: 240)

                Toggle(UIStrings.text("skillManager.network", "Network"), isOn: $store.skillManagerNetworkAllowed)
                    .toggleStyle(.switch)
                    .controlSize(.small)
            }
        }
        .padding()
        .frame(maxWidth: .infinity, alignment: .leading)
        .nativePanelSurface()
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
        .frame(maxWidth: .infinity, alignment: .leading)
        .accessibilityLabel(UIStrings.text("skillManager.workflow.accessibility", "Skill Manager workflow"))
    }

    @ViewBuilder
    private var skillManagerFeedback: some View {
        if let error = store.skillManagerErrorMessage {
            WorkflowSheetInlineBanner(message: error, style: .error)
        }

        if let message = store.skillManagerMessage {
            WorkflowSheetInlineBanner(message: message, style: .success)
        }
    }

    @ViewBuilder
    private var workflowInputContent: some View {
        switch selectedWorkflow {
        case .searchInstall:
            searchAndInstallControls
        case .installedUpdates:
            installedActionControls
        case .localLibrary:
            localLibraryControls
        }
    }

    @ViewBuilder
    private var workflowResultsContent: some View {
        switch selectedWorkflow {
        case .searchInstall:
            searchResultsSection
        case .installedUpdates:
            installedResultsSection
        case .localLibrary:
            localLibraryResultsSection
        }
    }

    @ViewBuilder
    private var workflowPreview: some View {
        if let preview = store.skillManagerMutationPreview {
            previewSection {
                mutationPreview(preview)
            }
        }

        switch selectedWorkflow {
        case .searchInstall, .installedUpdates:
            EmptyView()
        case .localLibrary:
            if store.skillManagerLocalCreatePreview != nil || store.skillManagerLocalDeletePreview != nil {
                previewSection {
                    if let preview = store.skillManagerLocalCreatePreview {
                        localCreatePreview(preview)
                    }
                    if let preview = store.skillManagerLocalDeletePreview {
                        localDeletePreview(preview)
                    }
                }
            }
        }
    }

    private var searchAndInstallControls: some View {
        VStack(alignment: .leading, spacing: 12) {
            Label(UIStrings.text("skillManager.searchInstall", "Search & Install"), systemImage: "magnifyingglass")
                .font(.headline)

            VStack(alignment: .leading, spacing: 10) {
                TextField(UIStrings.text("skillManager.query", "Search skills"), text: $store.skillManagerSearchQuery)
                    .textFieldStyle(.roundedBorder)
                HStack(spacing: 10) {
                    TextField(UIStrings.text("skillManager.owner", "Owner"), text: $store.skillManagerOwner)
                        .textFieldStyle(.roundedBorder)
                    Button {
                        Task { await store.searchSkillManager() }
                    } label: {
                        Label(UIStrings.text("action.search", "Search"), systemImage: "magnifyingglass")
                    }
                    .disabled(!canSearchSkillManager)
                }
            }

            skillManagerSuggestionBar

            VStack(alignment: .leading, spacing: 10) {
                TextField(UIStrings.text("skillManager.source", "Source"), text: $store.skillManagerSource)
                    .textFieldStyle(.roundedBorder)
                HStack(spacing: 10) {
                    TextField(UIStrings.text("skillManager.installSkillName", "Skill name"), text: $store.skillManagerInstallSkillName)
                        .textFieldStyle(.roundedBorder)
                    Button {
                        Task { await store.previewSkillManagerInstall() }
                    } label: {
                        Label(UIStrings.text("skillManager.previewInstall", "Preview Install"), systemImage: "plus.circle")
                    }
                    .disabled(store.isPreviewingSkillManagerMutation || externalMutationDisabled)
                }
            }
        }
        .padding()
        .frame(maxWidth: .infinity, alignment: .leading)
        .nativePanelSurface()
    }

    private var searchResultsSection: some View {
        VStack(alignment: .leading, spacing: 12) {
            Label(UIStrings.text("skillManager.results", "Results"), systemImage: "list.bullet.rectangle")
                .font(.headline)
            if let search = store.skillManagerSearchResult {
                commandPreviewLine(search.preview)
                if search.isBlockedByNetwork {
                    WorkflowSheetInlineBanner(
                        message: UIStrings.text(
                            "skillManager.search.networkBlocked",
                            "Network is off, so this remote search was previewed but not run."
                        ),
                        style: .warning
                    )
                } else if search.results.isEmpty {
                    Text(UIStrings.text("skillManager.search.noResults", "No search results returned."))
                        .font(.caption)
                        .foregroundStyle(.secondary)
                } else {
                    VStack(spacing: 8) {
                        ForEach(search.results.prefix(8)) { result in
                            SearchResultRow(result: result) {
                                Task {
                                    await store.previewSkillManagerInstall(
                                        source: result.source ?? result.name,
                                        skillName: result.name
                                    )
                                }
                            }
                            .disabled(externalMutationDisabled)
                        }
                    }
                }
            } else {
                Text(UIStrings.text("skillManager.results.empty", "Run a search or preview an install to populate this side of the workflow."))
                    .font(.caption)
                    .foregroundStyle(.secondary)
            }
        }
        .padding()
        .frame(maxWidth: .infinity, alignment: .leading)
        .nativePanelSurface()
    }

    private var skillManagerSearchSuggestions: [String] {
        let localNames = store.localSkillLibrarySkills.map(\.name)
        let installedNames = store.skillManagerInstalled?.installed.map(\.name) ?? []
        let fallback = [
            UIStrings.text("skillManager.suggestion.security", "security"),
            UIStrings.text("skillManager.suggestion.review", "code review"),
            UIStrings.text("skillManager.suggestion.diagnostics", "diagnostics")
        ]
        var seen = Set<String>()
        return (localNames + installedNames + fallback).compactMap { value in
            let trimmed = value.trimmingCharacters(in: .whitespacesAndNewlines)
            guard !trimmed.isEmpty, !seen.contains(trimmed.lowercased()) else { return nil }
            seen.insert(trimmed.lowercased())
            return trimmed
        }
        .prefix(6)
        .map { $0 }
    }

    private var canSearchSkillManager: Bool {
        !store.skillManagerSearchQuery.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
            && !store.isSearchingSkillManager
            && !externalMutationDisabled
    }

    private var skillManagerSuggestionBar: some View {
        HStack(spacing: 8) {
            Text(UIStrings.text("skillManager.suggestions", "Suggestions"))
                .font(.caption.bold())
                .foregroundStyle(.secondary)
            ScrollView(.horizontal, showsIndicators: false) {
                HStack(spacing: 6) {
                    ForEach(skillManagerSearchSuggestions, id: \.self) { suggestion in
                        Button {
                            store.skillManagerSearchQuery = suggestion
                        } label: {
                            SkillManagerSuggestionPill(title: suggestion)
                        }
                        .buttonStyle(.plain)
                        .help(suggestion)
                    }
                }
            }
        }
    }

    private func previewSection<Content: View>(@ViewBuilder content: () -> Content) -> some View {
        VStack(alignment: .leading, spacing: 12) {
            Label(UIStrings.text("skillManager.preview", "Preview"), systemImage: "doc.text.magnifyingglass")
                .font(.headline)
            content()
        }
        .padding()
        .frame(maxWidth: .infinity, alignment: .leading)
        .nativePanelSurface()
    }

    private var installedActionControls: some View {
        VStack(alignment: .leading, spacing: 12) {
            HStack {
                Label(UIStrings.text("skillManager.installed", "Installed"), systemImage: "list.bullet.rectangle")
                    .font(.headline)
                Spacer()
                Button {
                    Task { await store.listSkillManagerInstalled() }
                } label: {
                    Label(UIStrings.text("action.refresh", "Refresh"), systemImage: "arrow.clockwise")
                }
                .controlSize(.small)
                .disabled(store.isListingSkillManagerInstalled || externalMutationDisabled)
            }

            DisclosureGroup {
                VStack(alignment: .leading, spacing: 10) {
                    TextField(UIStrings.text("skillManager.removeSkillName", "Skill to remove or update"), text: $store.skillManagerRemoveSkillName)
                        .textFieldStyle(.roundedBorder)
                    HStack(spacing: 10) {
                        Button {
                            Task { await store.previewSkillManagerUpdate() }
                        } label: {
                            Label(UIStrings.text("skillManager.previewUpdate", "Preview Update"), systemImage: "arrow.triangle.2.circlepath")
                        }
                        .disabled(store.isPreviewingSkillManagerMutation || externalMutationDisabled)
                        Button(role: .destructive) {
                            Task { await store.previewSkillManagerRemove() }
                        } label: {
                            Label(UIStrings.text("skillManager.previewRemove", "Preview Remove"), systemImage: "minus.circle")
                        }
                        .disabled(store.isPreviewingSkillManagerMutation || externalMutationDisabled || store.skillManagerSelectedAgents.isEmpty)
                    }

                    Text(UIStrings.text(
                        "skillManager.removeSelected.summary",
                        "Removes manager-installed skill links for the selected agents above. Per-agent enablement remains controlled by agent configuration."
                    ))
                    .font(.caption)
                    .foregroundStyle(.secondary)
                }
                .padding(.top, 8)
            } label: {
                Label(UIStrings.text("skillManager.advancedInstalledActions", "Advanced package actions"), systemImage: "slider.horizontal.3")
            }
        }
        .padding()
        .frame(maxWidth: .infinity, alignment: .leading)
        .nativePanelSurface()
    }

    private var installedResultsSection: some View {
        VStack(alignment: .leading, spacing: 12) {
            Label(UIStrings.text("skillManager.installedResults", "Installed Skills"), systemImage: "checklist")
                .font(.headline)

            if let installed = store.skillManagerInstalled {
                commandPreviewLine(installed.preview)
                if installed.installed.isEmpty {
                    Text(UIStrings.text("skillManager.installed.empty", "No manager-installed skills returned."))
                        .font(.caption)
                        .foregroundStyle(.secondary)
                } else {
                    VStack(spacing: 8) {
                        ForEach(installed.installed.prefix(12)) { record in
                            InstalledSkillRow(record: record, externalMutationDisabled: externalMutationDisabled)
                        }
                    }
                }
            } else {
                Text(UIStrings.notLoaded)
                    .font(.caption)
                    .foregroundStyle(.secondary)
            }
        }
        .padding()
        .frame(maxWidth: .infinity, alignment: .leading)
        .nativePanelSurface()
    }

    private var localLibraryControls: some View {
        VStack(alignment: .leading, spacing: 12) {
            Label(UIStrings.text("skillManager.localLibrary", "Local Library"), systemImage: "folder")
                .font(.headline)

            TextField(UIStrings.text("skillManager.localName", "Local skill name"), text: $store.skillManagerLocalSkillName)
                .textFieldStyle(.roundedBorder)
            HStack(spacing: 10) {
                Button {
                    Task { await store.previewSkillManagerLocalCreate() }
                } label: {
                    Label(UIStrings.text("skillManager.previewCreate", "Preview Create"), systemImage: "doc.badge.plus")
                }
                .disabled(store.isPreviewingSkillManagerMutation)
            }
        }
        .padding()
        .frame(maxWidth: .infinity, alignment: .leading)
        .nativePanelSurface()
    }

    private var localLibraryResultsSection: some View {
        VStack(alignment: .leading, spacing: 12) {
            Label(UIStrings.text("skillManager.localLibraryResults", "Local Skills"), systemImage: "folder")
                .font(.headline)
            if store.localSkillLibrarySkills.isEmpty {
                Text(UIStrings.text("skillManager.local.empty", "No app-owned local skills in the library."))
                    .font(.caption)
                    .foregroundStyle(.secondary)
            } else {
                VStack(spacing: 8) {
                    ForEach(store.localSkillLibrarySkills.prefix(12)) { skill in
                        LocalSkillLibraryRow(skill: skill, externalMutationDisabled: externalMutationDisabled)
                    }
                }
            }
        }
        .padding()
        .frame(maxWidth: .infinity, alignment: .leading)
        .nativePanelSurface()
    }

    private func mutationPreview(_ preview: SkillManagerMutationRecord) -> some View {
        VStack(alignment: .leading, spacing: 10) {
            commandPreviewBlock(preview.preview)
            if let output = preview.output {
                commandOutput(output)
            }
            HStack {
                Spacer()
                Button {
                    pendingConfirmation = .mutation(preview.preview)
                } label: {
                    Label(applyTitle(for: preview.preview.operation), systemImage: "checkmark.circle")
                }
                .disabled(!canApply(preview.preview))
            }
        }
    }

    private func localCreatePreview(_ preview: SkillManagerLocalCreateRecord) -> some View {
        VStack(alignment: .leading, spacing: 10) {
            commandPreviewBlock(preview.preview)
            MetadataLine(label: UIStrings.source, value: preview.sourcePath)
            if let output = preview.output {
                commandOutput(output)
            }
            HStack {
                Spacer()
                Button {
                    pendingConfirmation = .localCreate(preview.preview, sourcePath: preview.sourcePath)
                } label: {
                    Label(UIStrings.text("skillManager.applyCreate", "Create"), systemImage: "checkmark.circle")
                }
                .disabled(!canApply(preview.preview))
            }
        }
    }

    private func localDeletePreview(_ preview: SkillManagerLocalDeleteRecord) -> some View {
        VStack(alignment: .leading, spacing: 10) {
            MetadataLine(label: UIStrings.text("metadata.skill", "Skill"), value: preview.skillName)
            MetadataLine(label: UIStrings.source, value: preview.path)
            Text(preview.summary)
                .font(.caption)
                .foregroundStyle(preview.physicalDeleteAllowed ? Color.secondary : Color.orange)
            if !preview.blockedByReferences.isEmpty {
                VStack(alignment: .leading, spacing: 5) {
                    Text(UIStrings.text("skillManager.delete.blockedRefs", "Agent references"))
                        .font(.caption.bold())
                        .foregroundStyle(.secondary)
                    ForEach(preview.blockedByReferences) { reference in
                        MetadataLine(label: DisplayText.agent(reference.agent), value: reference.path)
                    }
                }
            }
            HStack {
                Spacer()
                Button(role: .destructive) {
                    pendingConfirmation = .localDelete(preview)
                } label: {
                    Label(UIStrings.text("action.delete", "Delete"), systemImage: "trash")
                }
                .disabled(!preview.physicalDeleteAllowed || store.isApplyingSkillManagerMutation)
            }
        }
    }

    private func commandPreviewLine(_ preview: SkillManagerCommandPreview) -> some View {
        HStack(spacing: 8) {
            Image(systemName: preview.willRun ? "play.circle" : "doc.text.magnifyingglass")
                .foregroundStyle(.secondary)
            Text(preview.localizedSummary)
                .font(.caption)
                .foregroundStyle(.secondary)
                .lineLimit(2)
            Spacer()
            Text(preview.networkRequired ? UIStrings.text("skillManager.network.required", "Network") : UIStrings.text("skillManager.network.local", "Local"))
                .font(.caption2.bold())
                .foregroundStyle(preview.networkRequired && !preview.networkAllowed ? .orange : .secondary)
        }
    }

    private func commandPreviewBlock(_ preview: SkillManagerCommandPreview) -> some View {
        VStack(alignment: .leading, spacing: 8) {
            commandPreviewLine(preview)
            Text(preview.displayCommand)
                .font(.system(.caption, design: .monospaced))
                .textSelection(.enabled)
                .padding(8)
                .frame(maxWidth: .infinity, alignment: .leading)
                .background(Color.white, in: RoundedRectangle(cornerRadius: 8))
            CompactMetadataGrid(rows: preview.compactMetadataRows)
            if !preview.risks.isEmpty {
                DenseDisclosureList(preview.risks, visibleLimit: 3) { risk in
                    Label(risk, systemImage: "exclamationmark.triangle")
                        .font(.caption)
                        .foregroundStyle(.secondary)
                }
            }
        }
    }

    private func commandOutput(_ output: SkillManagerCommandOutput) -> some View {
        SkillManagerCommandOutputView(output: output)
    }

    private var confirmationBinding: Binding<Bool> {
        Binding(
            get: { pendingConfirmation != nil },
            set: { isPresented in
                if !isPresented {
                    pendingConfirmation = nil
                }
            }
        )
    }

    private var confirmationTitle: String {
        pendingConfirmation?.title ?? UIStrings.text("skillManager.confirm.title", "Confirm Skill Manager Operation")
    }

    private func canApply(_ preview: SkillManagerCommandPreview) -> Bool {
        preview.requiresConfirmation
            && (!preview.networkRequired || preview.networkAllowed)
            && !store.isApplyingSkillManagerMutation
    }

    private func applyTitle(for operation: String) -> String {
        switch operation {
        case "install":
            return UIStrings.text("skillManager.applyInstall", "Install")
        case "remove":
            return UIStrings.text("skillManager.applyRemove", "Remove")
        case "update":
            return UIStrings.text("skillManager.applyUpdate", "Update")
        default:
            return UIStrings.text("action.apply", "Apply")
        }
    }

    private func applyCurrentMutation(_ operation: String) async {
        switch operation {
        case "install":
            await store.applySkillManagerInstall()
        case "remove":
            await store.applySkillManagerRemove()
        case "update":
            await store.applySkillManagerUpdate()
        default:
            break
        }
    }

    private func applyConfirmed(_ confirmation: SkillManagerWriteConfirmation) async {
        switch confirmation {
        case .mutation(let preview):
            await applyCurrentMutation(preview.operation)
        case .localCreate:
            await store.applySkillManagerLocalCreate()
        case .localDelete:
            await store.applySkillManagerLocalDelete()
        }
    }

    private var primaryTool: SkillManagerToolRecord? {
        store.skillManagerTools.first { $0.id == "npx-skills" } ?? store.skillManagerTools.first
    }

    private var externalMutationDisabled: Bool {
        externalManagerUnavailableMessage != nil
    }

    private var externalManagerUnavailableMessage: String? {
        guard !store.isLoadingSkillManagerTools else { return nil }
        if let tool = primaryTool {
            let status = tool.status.lowercased()
            if tool.executable == nil || status.contains("unavailable") || status.contains("error") || status.contains("missing") {
                return UIStrings.text(
                    "skillManager.toolUnavailable.message",
                    "The external manager tool is unavailable. Install Node/npm or set SKILLS_COPILOT_NPX_PATH, then refresh."
                )
            }
            return nil
        }
        if store.skillManagerErrorMessage != nil {
            return UIStrings.text(
                "skillManager.toolUnavailable.message",
                "The external manager tool is unavailable. Install Node/npm or set SKILLS_COPILOT_NPX_PATH, then refresh."
            )
        }
        return nil
    }

    private func toolUnavailableCard(_ message: String) -> some View {
        VStack(alignment: .leading, spacing: 8) {
            Label(UIStrings.text("skillManager.toolUnavailable.title", "External manager unavailable"), systemImage: "exclamationmark.triangle.fill")
                .font(.headline)
                .foregroundStyle(.orange)
            Text(message)
                .font(.callout)
                .foregroundStyle(.secondary)
                .fixedSize(horizontal: false, vertical: true)
            if let error = store.skillManagerErrorMessage {
                Text(error)
                    .font(.caption)
                    .foregroundStyle(.secondary)
                    .textSelection(.enabled)
                    .lineLimit(3)
            }
        }
        .padding(.vertical, 12)
        .padding(.horizontal, 14)
        .frame(maxWidth: .infinity, alignment: .leading)
        .nativePanelSurface()
        .overlay(alignment: .leading) {
            Rectangle()
                .fill(Color.orange)
                .frame(width: 3)
                .clipShape(Capsule())
        }
    }
}

private enum SkillManagerWriteConfirmation {
    case mutation(SkillManagerCommandPreview)
    case localCreate(SkillManagerCommandPreview, sourcePath: String)
    case localDelete(SkillManagerLocalDeleteRecord)

    var title: String {
        switch self {
        case .mutation(let preview):
            switch preview.operation {
            case "install":
                return UIStrings.text("skillManager.confirm.install.title", "Confirm Skill Install")
            case "remove":
                return UIStrings.text("skillManager.confirm.remove.title", "Confirm Skill Removal")
            case "update":
                return UIStrings.text("skillManager.confirm.update.title", "Confirm Skill Update")
            default:
                return UIStrings.text("skillManager.confirm.title", "Confirm Skill Manager Operation")
            }
        case .localCreate:
            return UIStrings.text("skillManager.confirm.localCreate.title", "Confirm Local Skill Creation")
        case .localDelete:
            return UIStrings.text("skillManager.confirm.localDelete.title", "Confirm Local Skill Delete")
        }
    }

    var confirmButtonTitle: String {
        switch self {
        case .mutation(let preview):
            switch preview.operation {
            case "install":
                return UIStrings.text("skillManager.applyInstall", "Install")
            case "remove":
                return UIStrings.text("skillManager.applyRemove", "Remove")
            case "update":
                return UIStrings.text("skillManager.applyUpdate", "Update")
            default:
                return UIStrings.text("action.apply", "Apply")
            }
        case .localCreate:
            return UIStrings.text("skillManager.applyCreate", "Create")
        case .localDelete:
            return UIStrings.text("action.delete", "Delete")
        }
    }

    var role: ButtonRole? {
        switch self {
        case .mutation(let preview) where preview.operation == "remove":
            return .destructive
        case .localDelete:
            return .destructive
        default:
            return nil
        }
    }

    var message: String {
        switch self {
        case .mutation(let preview):
            return [
                preview.summary,
                "\(UIStrings.text("skillManager.confirm.targets", "Targets")): \(targetSummary(from: preview.command))",
                "\(UIStrings.text("skillManager.cwd", "CWD")): \(preview.cwd)",
                "\(UIStrings.text("skillManager.confirm.command", "Command")): \(preview.displayCommand)"
            ].joined(separator: "\n\n")
        case .localCreate(let preview, let sourcePath):
            return [
                preview.summary,
                "\(UIStrings.source): \(sourcePath)",
                "\(UIStrings.text("skillManager.cwd", "CWD")): \(preview.cwd)",
                "\(UIStrings.text("skillManager.confirm.command", "Command")): \(preview.displayCommand)"
            ].joined(separator: "\n\n")
        case .localDelete(let preview):
            return [
                preview.summary,
                "\(UIStrings.text("metadata.skill", "Skill")): \(preview.skillName)",
                "\(UIStrings.source): \(preview.path)"
            ].joined(separator: "\n\n")
        }
    }

    private func targetSummary(from command: [String]) -> String {
        var agents: [String] = []
        var index = command.startIndex
        while index < command.endIndex {
            if command[index] == "--agent" {
                let valueIndex = command.index(after: index)
                if valueIndex < command.endIndex {
                    agents.append(DisplayText.agent(command[valueIndex]))
                    index = command.index(after: valueIndex)
                } else {
                    index = valueIndex
                }
            } else {
                index = command.index(after: index)
            }
        }
        return agents.isEmpty ? UIStrings.text("skillManager.agents.unknown", "unknown agents") : agents.joined(separator: ", ")
    }
}

private struct SkillManagerCommandOutputView: View {
    let output: SkillManagerCommandOutput
    @State private var isExpanded = false

    var body: some View {
        VStack(alignment: .leading, spacing: 6) {
            MetadataLine(label: UIStrings.text("metadata.status", "Status"), value: output.status)
            if !output.stdout.isEmpty {
                outputText(output.stdout, foregroundStyle: AnyShapeStyle(.primary))
            }
            if !output.stderr.isEmpty {
                outputText(output.stderr, foregroundStyle: AnyShapeStyle(Color.orange))
            }
            if hasOverflow {
                Button {
                    isExpanded.toggle()
                } label: {
                    Label(
                        isExpanded ? UIStrings.text("action.showLess", "Show less") : UIStrings.text("action.showMore", "Show more"),
                        systemImage: isExpanded ? "chevron.up" : "chevron.down"
                    )
                }
                .buttonStyle(.borderless)
                .controlSize(.small)
            }
        }
    }

    private func outputText(_ value: String, foregroundStyle: AnyShapeStyle) -> some View {
        Text(value)
            .font(.system(.caption, design: .monospaced))
            .foregroundStyle(foregroundStyle)
            .textSelection(.enabled)
            .lineLimit(isExpanded ? nil : 6)
    }

    private var hasOverflow: Bool {
        output.stdout.split(separator: "\n", omittingEmptySubsequences: false).count > 6
            || output.stderr.split(separator: "\n", omittingEmptySubsequences: false).count > 6
    }
}

private struct SkillManagerTargetSummary: View {
    let agents: [SkillManagerAgent]

    var body: some View {
        HStack(spacing: 6) {
            ForEach(agents.prefix(4)) { agent in
                SkillManagerTargetIcon(agent: agent)
            }
            Text(summary)
                .font(.caption)
                .foregroundStyle(.secondary)
                .lineLimit(1)
        }
        .padding(.horizontal, 8)
        .padding(.vertical, 4)
        .background(Color.white, in: Capsule())
        .help(agents.map(\.title).joined(separator: ", "))
    }

    private var summary: String {
        if agents.isEmpty {
            return UIStrings.text("skillManager.agents.none", "No target agents selected")
        }
        if agents.count == SkillManagerAgent.defaultTargets.count {
            return UIStrings.text("skillManager.agents.allSelected", "All agents")
        }
        let names = agents.prefix(2).map(\.title).joined(separator: ", ")
        let remaining = agents.count - min(agents.count, 2)
        if remaining > 0 {
            return "\(names) +\(remaining)"
        }
        return names
    }
}

private struct SkillManagerTargetIcon: View {
    let agent: SkillManagerAgent

    var body: some View {
        Group {
            if let image = AgentIconProvider.image(for: agent.skillAgentFilter) {
                Image(nsImage: image)
                    .resizable()
                    .scaledToFit()
            } else {
                Image(systemName: "person.crop.circle")
                    .font(.caption)
                    .foregroundStyle(.secondary)
            }
        }
        .frame(width: 16, height: 16)
        .clipShape(RoundedRectangle(cornerRadius: 3))
        .accessibilityLabel(agent.title)
    }
}

private struct SkillManagerSuggestionPill: View {
    let title: String

    var body: some View {
        Text(title)
            .font(.caption.bold())
            .lineLimit(1)
            .padding(.horizontal, 9)
            .padding(.vertical, 5)
            .background(Color.accentColor.opacity(0.12), in: Capsule())
            .overlay(
                Capsule()
                    .stroke(Color.accentColor.opacity(0.25), lineWidth: 1)
            )
    }
}

private struct SearchResultRow: View {
    let result: SkillManagerSearchResult
    let onPreviewInstall: () -> Void

    var body: some View {
        HStack(alignment: .center, spacing: 10) {
            VStack(alignment: .leading, spacing: 3) {
                Text(result.name)
                    .font(.callout.bold())
                    .lineLimit(1)
                Text(result.description ?? result.source ?? "")
                    .font(.caption)
                    .foregroundStyle(.secondary)
                    .lineLimit(2)
            }
            Spacer()
            Button(action: onPreviewInstall) {
                Label(UIStrings.text("skillManager.previewInstall", "Preview Install"), systemImage: "plus.circle")
            }
            .controlSize(.small)
        }
        .padding(10)
        .background(Color.white, in: RoundedRectangle(cornerRadius: 8))
        .accessibilityElement(children: .combine)
        .accessibilityLabel(result.name)
        .accessibilityValue(result.description ?? result.source ?? "")
    }
}

private struct InstalledSkillRow: View {
    @EnvironmentObject private var store: SkillStore
    let record: SkillManagerInstalledRecord
    let externalMutationDisabled: Bool

    var body: some View {
        HStack(alignment: .center, spacing: 10) {
            VStack(alignment: .leading, spacing: 3) {
                Text(record.name)
                    .font(.callout.bold())
                    .lineLimit(1)
                Text(installedSummary)
                    .font(.caption)
                    .foregroundStyle(.secondary)
                    .lineLimit(2)
            }
            Spacer()
            Button {
                Task { await store.previewSkillManagerUpdate(skillName: record.name) }
            } label: {
                Label(UIStrings.text("skillManager.previewUpdate", "Preview Update"), systemImage: "arrow.triangle.2.circlepath")
            }
            .controlSize(.small)
            .disabled(externalMutationDisabled)
            Button(role: .destructive) {
                Task { await store.previewSkillManagerRemove(skillName: record.name) }
            } label: {
                Label(UIStrings.text("skillManager.previewRemove", "Preview Remove"), systemImage: "minus.circle")
            }
            .controlSize(.small)
            .disabled(externalMutationDisabled)
        }
        .padding(10)
        .background(Color.white, in: RoundedRectangle(cornerRadius: 8))
        .accessibilityElement(children: .combine)
        .accessibilityLabel(record.name)
        .accessibilityValue(installedSummary)
    }

    private var installedSummary: String {
        let agents = record.agents.isEmpty ? UIStrings.text("skillManager.agents.unknown", "unknown agents") : record.agents.map(DisplayText.agent).joined(separator: ", ")
        return [record.source, record.scope, agents].compactMap { $0 }.joined(separator: " · ")
    }
}

private struct LocalSkillLibraryRow: View {
    @EnvironmentObject private var store: SkillStore
    let skill: SkillRecord
    let externalMutationDisabled: Bool

    var body: some View {
        HStack(alignment: .center, spacing: 10) {
            VStack(alignment: .leading, spacing: 3) {
                Text(skill.name)
                    .font(.callout.bold())
                    .lineLimit(1)
                Text(skill.displayPath)
                    .font(.caption)
                    .foregroundStyle(.secondary)
                    .lineLimit(1)
                    .truncationMode(.middle)
            }
            Spacer()
            Button {
                Task {
                    await store.previewSkillManagerInstall(
                        source: store.skillManagerSourcePath(for: skill),
                        skillName: skill.name
                    )
                }
            } label: {
                Label(UIStrings.text("skillManager.previewInstall", "Preview Install"), systemImage: "plus.circle")
            }
            .controlSize(.small)
            .disabled(externalMutationDisabled)
            Button(role: .destructive) {
                Task { await store.previewSkillManagerLocalDelete(skill: skill) }
            } label: {
                Label(UIStrings.text("skillManager.previewDelete", "Preview Delete"), systemImage: "trash")
            }
            .controlSize(.small)
        }
        .padding(10)
        .background(Color.white, in: RoundedRectangle(cornerRadius: 8))
        .accessibilityElement(children: .combine)
        .accessibilityLabel(skill.name)
        .accessibilityValue(skill.displayPath)
    }
}

private extension SkillManagerAgent {
    var skillAgentFilter: SkillAgentFilter {
        switch self {
        case .claudeCode:
            return .claudeCode
        case .pi:
            return .pi
        case .opencode:
            return .opencode
        case .codex:
            return .codex
        case .hermesAgent:
            return .hermes
        case .openclaw:
            return .openclaw
        }
    }
}
