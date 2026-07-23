import AppKit
import SwiftUI

struct SessionsWorkspaceView: View {
    @EnvironmentObject private var store: SkillStore

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
                onCopyResumeCommand: copyResumeCommand
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
}
