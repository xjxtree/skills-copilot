import AppKit
import SwiftUI

struct DetailView: View {
    @EnvironmentObject private var store: SkillStore
    @EnvironmentObject private var sessionStore: SessionStore
    @Environment(\.accessibilityReduceMotion) private var reduceMotion
    let skill: SkillRecord?
    @State private var isSingleTogglePreviewPresented = false

    private static let topAnchorID = "skills-copilot.detail.top"

    var body: some View {
        ScrollViewReader { proxy in
            ScrollView {
                VStack(spacing: 0) {
                    Color.clear
                        .frame(height: 0)
                        .accessibilityHidden(true)
                        .id(Self.topAnchorID)

                    VStack(alignment: .leading, spacing: 24) {
                        DetailFeedbackInlineView(
                            errorMessage: store.errorMessage,
                            lastMutationMessage: store.lastMutationMessage,
                            partialRefreshMessage: store.partialScanWarningMessage
                        )
                        .equatable()

                        if store.selectedSidebarSelection?.isSession == true {
                            AgentSessionDetailPanel()
                        } else if store.selectedSidebarSelection?.isConfig == true {
                            AgentConfigDetailPanel()
                        } else if store.selectedSidebarSelection?.isSkill == true, let skill {
                                SkillDetailContentView(
                                    skill: skill,
                                    sessionUsage: sessionUsage(for: skill),
                                    onToggle: { targetSkill, on in
                                        Task {
                                            await store.prepareSingleSkillTogglePreview(skill: targetSkill, on: on)
                                            isSingleTogglePreviewPresented = true
                                        }
                                    }
                                )
                        } else {
                            EmptyDetailView(
                                title: emptyDetailTitle,
                                message: emptyDetailMessage,
                                systemImage: emptyDetailSystemImage
                            )
                        }
                    }
                    .padding(.top, 8)
                    .padding(.horizontal, 28)
                    .padding(.bottom, 28)
                    .frame(maxWidth: .infinity, alignment: .leading)
                }
            }
            .onChange(of: store.selectedDetailSection) { _ in
                scrollToTop(proxy)
            }
            .onChange(of: store.selectedSidebarSelection) { _ in
                scrollToTop(proxy)
            }
        }
        .navigationTitle("")
        .sheet(isPresented: $isSingleTogglePreviewPresented) {
            BatchSkillOperationSheet()
                .environmentObject(store)
        }
        .transaction { transaction in
            if reduceMotion {
                transaction.animation = nil
            }
        }
    }

    private var emptyDetailTitle: String {
        switch store.sidebarContentMode {
        case .sessions:
            UIStrings.noSessionSelected
        case .config:
            UIStrings.noConfigSelected
        case .skills:
            UIStrings.noSkillSelected
        }
    }

    private var emptyDetailMessage: String {
        switch store.sidebarContentMode {
        case .sessions:
            UIStrings.noSessionSelectedMessage
        case .config:
            UIStrings.noConfigSelectedMessage
        case .skills:
            UIStrings.noSkillSelectedMessage
        }
    }

    private var emptyDetailSystemImage: String {
        switch store.sidebarContentMode {
        case .sessions:
            "bubble.left.and.bubble.right"
        case .config:
            "slider.horizontal.3"
        case .skills:
            "sparkle.magnifyingglass"
        }
    }

    private func scrollToTop(_ proxy: ScrollViewProxy) {
        if reduceMotion {
            proxy.scrollTo(Self.topAnchorID, anchor: .top)
        } else {
            withAnimation(.easeInOut(duration: 0.18)) {
                proxy.scrollTo(Self.topAnchorID, anchor: .top)
            }
        }
    }

    private func sessionUsage(for skill: SkillRecord) -> LocalSessionSkillUsageRow? {
        sessionStore.localSessionPreviewResult.skillUsageRows.first { row in
            row.skillId == skill.id
                || row.skillName == skill.name
                || row.skillName.caseInsensitiveCompare(skill.name) == .orderedSame
        }
    }
}


private struct DetailFeedbackInlineView: View, Equatable {
    let errorMessage: String?
    let lastMutationMessage: String?
    let partialRefreshMessage: String?

    static func == (lhs: DetailFeedbackInlineView, rhs: DetailFeedbackInlineView) -> Bool {
        lhs.errorMessage == rhs.errorMessage
            && lhs.lastMutationMessage == rhs.lastMutationMessage
            && lhs.partialRefreshMessage == rhs.partialRefreshMessage
    }

    var body: some View {
        if let error = errorMessage {
            DetailFeedbackToast(
                message: error,
                systemImage: "exclamationmark.triangle.fill",
                color: .red
            )
        } else if let message = partialRefreshMessage {
            DetailFeedbackToast(
                message: message,
                systemImage: "exclamationmark.triangle.fill",
                color: .orange
            )
        } else if let message = lastMutationMessage {
            DetailFeedbackToast(
                message: message,
                systemImage: "checkmark.circle.fill",
                color: .green
            )
        }
    }
}

private struct SkillDetailContentView: View {
    @EnvironmentObject private var store: SkillStore
    let skill: SkillRecord
    let sessionUsage: LocalSessionSkillUsageRow?
    let onToggle: (SkillRecord, Bool) -> Void

    var body: some View {
        let selectedDetailSection = store.selectedDetailSection.visibleSkillDetailSection
        let selectedFindingGroups = FindingDisplayModel.issueGroups(
            findings: store.selectedDisplayFindings,
            severityFilter: FindingDisplayModel.allFilterValue,
            ruleFilter: FindingDisplayModel.allFilterValue
        )
        let catalogStatusIssueCount = SkillListModel.catalogStatusIssueKind(for: skill) == nil ? 0 : 1

        VStack(alignment: .leading, spacing: 24) {
            HeaderView(
                skill: skill,
                adoptingAgentSummary: store.adoptingAgentSummary(for: skill),
                sessionUsage: sessionUsage,
                issueCount: selectedFindingGroups.count + catalogStatusIssueCount,
                conflictCount: store.selectedConflicts.count,
                isWriting: store.isWriting,
                adapterCapability: store.adapterCapabilities.first { $0.agent == skill.agent },
                onSelectSection: { section in
                    store.selectedDetailSection = section
                },
                onToggle: { on in
                    onToggle(skill, on)
                }
            )

            DetailSectionSwitcher(selection: Binding(
                get: { store.selectedDetailSection.visibleSkillDetailSection },
                set: { store.selectedDetailSection = $0 }
            ))

            switch selectedDetailSection {
            case .overview:
                VStack(alignment: .leading, spacing: 16) {
                    SkillSummaryCard(
                        skill: skill,
                        detail: store.selectedSkillDetail,
                        scriptPreview: store.scriptExecutionPreview(for: skill),
                        isLoading: store.isLoadingDetail
                    )

                    if DisplayText.isToolGlobal(skill) {
                        ToolGlobalPreviewCard(skill: skill)
                    }
                }
            case .findings:
                FindingsSection(
                    skill: skill,
                    findings: store.selectedDisplayFindings,
                    catalogCompleteness: store.catalogCompleteness(forAgent: skill.agent)
                )
            case .conflicts:
                ConflictsSection(
                    conflicts: store.selectedConflicts,
                    selectedSkillID: skill.id,
                    currentAgentSkills: store.skills.filter { $0.agent == skill.agent },
                    catalogCompleteness: store.catalogCompleteness(forAgent: skill.agent),
                    onSelectSkill: { conflictSkill in
                        store.selectedSidebarSelection = .skill(conflictSkill.id)
                    },
                    onDisableDuplicate: { conflictSkill in
                        onToggle(conflictSkill, false)
                    }
                )
            case .history:
                HistorySection(
                    events: store.selectedSkillEvents,
                    isLoading: store.isLoadingSelectedSkillEvents,
                    completeness: store.selectedSkillEventCompleteness,
                    onLoadMore: {
                        Task { await store.loadMoreSkillEvents(instanceID: skill.id, loadAll: false) }
                    },
                    onLoadAll: {
                        Task { await store.loadMoreSkillEvents(instanceID: skill.id, loadAll: true) }
                    },
                    onCancel: {
                        store.cancelSkillEventLoadAll(instanceID: skill.id)
                    }
                )
            case .metadata:
                SkillDetailCard(
                    skill: skill,
                    detail: store.selectedSkillDetail,
                    adapterCapability: store.adapterCapabilities.first { $0.agent == skill.agent },
                    isLoading: store.isLoadingDetail
                )
            }
        }
        .frame(
            maxWidth: CGFloat(MainWindowModel.maximumReadableDetailWidth),
            alignment: .leading
        )
        .frame(maxWidth: .infinity, alignment: .center)
    }
}

enum SkillProvenanceDisplay {
    static func rootClass(for skill: SkillRecord) -> String {
        switch skill.provenance.rootKind {
        case .toolGlobal:
            return UIStrings.provenanceToolGlobalRoot
        case .native:
            if isNativeOpencodeRoot(skill) {
                return UIStrings.provenanceNativeOpencodeRoot
            }
            return "\(DisplayText.agent(skill.agent)) \(UIStrings.provenanceNativeRoot)"
        case .compatibility:
            if isClaudeCompatibilityRoot(skill) {
                return UIStrings.provenanceClaudeCompatibilityRoot
            }
            if isAgentsCompatibilityRoot(skill) {
                return UIStrings.provenanceAgentsCompatibilityRoot
            }
            return skill.provenance.label
        case .configured:
            return "\(DisplayText.agent(skill.agent)) \(UIStrings.provenanceConfiguredRoot)"
        case .external:
            if skill.agent == "hermes" {
                return UIStrings.provenanceHermesExternalRoot
            }
            return UIStrings.provenanceExternalRoot
        case .readOnly:
            if skill.agent == "hermes" {
                return UIStrings.provenanceHermesHomeProfileRoot
            }
            if skill.agent == "openclaw" {
                if skill.provenance.scopeKind == .project {
                    return UIStrings.provenanceOpenClawWorkspaceRoot
                }
                return UIStrings.provenanceOpenClawReadOnlyRoot
            }
            return "\(DisplayText.agent(skill.agent)) \(UIStrings.provenanceReadOnlyRoot)"
        case .unknown:
            return UIStrings.provenanceUnclassifiedRoot
        }
    }

    static func kind(for skill: SkillRecord) -> String {
        switch skill.provenance.rootKind {
        case .toolGlobal:
            return UIStrings.provenanceToolGlobalKind
        case .native:
            return UIStrings.provenanceNativeKind
        case .compatibility:
            return UIStrings.provenanceCompatibilityKind
        case .configured:
            return UIStrings.provenanceConfiguredKind
        case .external:
            return UIStrings.provenanceExternalKind
        case .readOnly:
            return UIStrings.provenanceReadOnlyKind
        case .unknown:
            return UIStrings.provenanceInferredKind
        }
    }

    private static func isClaudeCompatibilityRoot(_ skill: SkillRecord) -> Bool {
        pathText(for: skill).contains(".claude/skills")
    }

    private static func isAgentsCompatibilityRoot(_ skill: SkillRecord) -> Bool {
        pathText(for: skill).contains(".agents/skills")
    }

    private static func isNativeOpencodeRoot(_ skill: SkillRecord) -> Bool {
        let path = pathText(for: skill)
        return path.contains(".config/opencode/skills") || path.contains(".opencode/skills")
    }

    private static func pathText(for skill: SkillRecord) -> String {
        "\(skill.path)\n\(skill.displayPath)".lowercased()
    }
}
