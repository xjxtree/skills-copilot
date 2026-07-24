import AppKit
import SwiftUI

struct SessionsWorkspaceView: View {
    @EnvironmentObject private var store: SkillStore
    @State private var evidenceSelection: ContextualEvidenceSelection?

    private var workspace: SessionWorkspaceStore {
        store.sessionWorkspaceStore
    }

    var body: some View {
        HSplitView {
            SessionsWorkspaceListView(
                workspace: workspace,
                onSetCriteria: workspace.setCriteria,
                onSelectSession: workspace.selectSession,
                onLoadNext: workspace.loadNextInventoryPage,
                onRefresh: workspace.refreshInventory,
                onCancelLoading: workspace.cancelInventoryRequest,
                onAgentFilterChange: updateAgentFilter
            )
            .frame(
                minWidth: 380,
                idealWidth: 460,
                maxWidth: 580,
                maxHeight: .infinity
            )

            SessionWorkspaceDetailView(
                session: workspace.selectedSession,
                detailState: workspace.selectedSessionDetailState,
                messageCompleteness: workspace.selectedMessageCompleteness,
                inventorySourceRevision: workspace.sourceRevision,
                productSnapshotRevision: workspace.snapshotRevision,
                resumePreview: workspace.resumePreview,
                resumeError: workspace.resumeError,
                isLoadingResume: workspace.isPreviewingResume,
                gapNotes: detailGapNotes,
                contextualFlow: selectedDigestFlow,
                contextualSourceRevision: productSourceRevision,
                providerGateMessage: providerGateMessage,
                onLoadTimelineMore: {
                    Task { await workspace.loadNextSelectedSessionTimelinePage() }
                },
                onLoadTimelineAll: {
                    Task { await workspace.loadAllSelectedSessionTimeline() }
                },
                onCancelTimelineLoad: workspace.cancelMessageRequest,
                onPreviewResume: {
                    Task { await workspace.previewSelectedSessionResume() }
                },
                onCopyResumeCommand: copyResumeCommand,
                onPreviewDigest: {
                    Task { await previewSelectedSessionDigest() }
                },
                onConfirmDigest: {
                    Task { await sendSelectedSessionDigest() }
                },
                onDismissDigestPreview: clearSelectedSessionDigest,
                onOpenDigestEvidence: {
                    evidenceSelection = ContextualEvidenceSelection(reference: $0)
                }
            )
            .id(workspace.selectedSessionID)
            .frame(maxWidth: .infinity, maxHeight: .infinity)
        }
        .task(id: workspace.inventoryBindingID) {
            await workspace.loadInventoryIfNeeded()
            await workspace.loadPendingSelectionIfNeeded()
        }
        .task(id: workspace.selectedSessionID) {
            await workspace.loadSelectedSessionTimelineIfNeeded()
        }
        .onDisappear {
            workspace.cancelInventoryRequest()
            workspace.cancelMessageRequest()
            workspace.cancelResumeRequest()
        }
        .sheet(item: $evidenceSelection) { selection in
            ContextualEvidenceSheet(selection: selection)
        }
        .accessibilityIdentifier("sessions.workspace")
    }

    private var detailGapNotes: [String] {
        guard let result = workspace.acceptedSnapshot?.result else { return [] }
        return Array(Set(result.gapNotes + result.blockerNotes)).sorted()
    }

    private func updateAgentFilter(_ agent: ProductAgentID?) {
        store.agentFilter = agent.flatMap { productAgent in
            SkillAgentFilter.managementCases.first {
                $0.rawValue == productAgent.rawValue
            }
        } ?? .all
    }

    private func copyResumeCommand(_ command: String) {
        let pasteboard = NSPasteboard.general
        pasteboard.clearContents()
        pasteboard.setString(command, forType: .string)
    }

    private var productSourceRevision: String? {
        store.appContextStore.visibleProjectReadiness?.sourceRevision
    }

    private var selectedDigestFlow: ContextualIntelligenceFlow? {
        guard let id = workspace.selectedSessionID else { return nil }
        return store.contextualIntelligenceStore.flow(
            for: ContextualIntelligenceStore.sessionKey(id)
        )
    }

    private var providerGateMessage: String? {
        guard store.appContextStore.activeProject != nil,
              productSourceRevision != nil else {
            return UIStrings.text(
                "intelligence.snapshotRequired",
                "Refresh the project before requesting interpretation."
            )
        }
        let status = store.aiProviderStatus
        if !status.serviceAvailable {
            return UIStrings.localizedServiceMessage(
                status.disabledReason ?? UIStrings.aiProviderUnavailable
            )
        }
        if !status.configured || status.activeProfile == nil {
            return UIStrings.text(
                "intelligence.providerRequired",
                "Configure an AI provider to request optional interpretation."
            )
        }
        return status.enabled
            ? nil
            : status.disabledReason.map(UIStrings.localizedServiceMessage)
                ?? UIStrings.text(
                    "intelligence.providerDisabled",
                    "The configured AI provider is disabled."
                )
    }

    private func previewSelectedSessionDigest() async {
        if workspace.resumePreview?.id != workspace.selectedSessionID {
            await workspace.previewSelectedSessionResume()
        }
        guard let continuation = workspace.resumePreview,
              let project = store.appContextStore.activeProject,
              let revision = productSourceRevision else { return }
        await store.contextualIntelligenceStore.previewSessionDigest(
            authorizedRoots: workspace.authorizedRoots,
            project: project,
            session: continuation,
            productSourceRevision: revision
        )
    }

    private func sendSelectedSessionDigest() async {
        guard let continuation = workspace.resumePreview,
              let project = store.appContextStore.activeProject,
              let revision = productSourceRevision else { return }
        await store.contextualIntelligenceStore.sendSessionDigest(
            authorizedRoots: workspace.authorizedRoots,
            project: project,
            session: continuation,
            productSourceRevision: revision
        )
    }

    private func clearSelectedSessionDigest() {
        guard let id = workspace.selectedSessionID else { return }
        store.contextualIntelligenceStore.clear(
            ContextualIntelligenceStore.sessionKey(id)
        )
    }
}
