import Combine
import Foundation

struct ProjectReadinessCacheKey: Hashable {
    let projectID: String
    let projectContextRevision: String
}

struct ProjectReadinessCacheEntry: Hashable {
    let key: ProjectReadinessCacheKey
    let record: ProjectReadinessRecord

    var sourceRevision: String { record.sourceRevision }
}

enum ProjectReadinessCacheState: Hashable {
    case empty
    case refreshing(previous: ProjectReadinessCacheEntry?)
    case accepted(ProjectReadinessCacheEntry)
    case stale(ProjectReadinessCacheEntry, message: String?)
    case failed(message: String)

    var visibleEntry: ProjectReadinessCacheEntry? {
        switch self {
        case .empty, .failed:
            return nil
        case .refreshing(let previous):
            return previous
        case .accepted(let entry), .stale(let entry, _):
            return entry
        }
    }

    var isRefreshing: Bool {
        if case .refreshing = self {
            return true
        }
        return false
    }

    var isStale: Bool {
        if case .stale = self {
            return true
        }
        return false
    }

    var errorMessage: String? {
        switch self {
        case .stale(_, let message):
            return message
        case .failed(let message):
            return message
        case .empty, .refreshing, .accepted:
            return nil
        }
    }
}

@MainActor
final class AppContextStore: ObservableObject {
    static let maximumReadinessCacheEntries = 12

    @Published private(set) var projectContextState: ProjectContextState?
    @Published private(set) var route: AppRoute
    @Published private(set) var agentFilter: ProductAgentID?
    @Published private(set) var readinessState: ProjectReadinessCacheState = .empty
    @Published private(set) var isLoadingProjectContext = false
    @Published private(set) var projectContextErrorMessage: String?

    var activeProject: ProjectContext? { projectContextState?.active }
    var recentProjects: [ProjectContext] { projectContextState?.recent ?? [] }
    var visibleProjectReadiness: ProjectReadinessRecord? {
        readinessState.visibleEntry?.record
    }

    var hasCurrentProjectReadiness: Bool {
        guard let key = currentReadinessKey,
              let visibleKey = readinessState.visibleEntry?.key else {
            return false
        }
        return key == visibleKey && !readinessState.isStale
    }

    private let service: ServiceClient
    private var contextRefreshTask: Task<ProjectContextState, Error>?
    private var readinessRefreshTask: Task<ProjectReadinessRecord, Error>?
    private var contextRefreshGeneration: UInt64 = 0
    private var readinessRefreshGeneration: UInt64 = 0
    private var readinessCache: [ProjectReadinessCacheKey: ProjectReadinessCacheEntry] = [:]
    private var readinessCacheOrder: [ProjectReadinessCacheKey] = []

    init(
        service: ServiceClient,
        restoredRoute: AppRoute = .defaultRoute,
        initialProjectContextState: ProjectContextState? = nil,
        initialAgentFilter: ProductAgentID? = nil
    ) {
        self.service = service
        route = restoredRoute
        projectContextState = initialProjectContextState
        agentFilter = initialAgentFilter.flatMap {
            ProductAgentID.projectAgents.contains($0) ? $0 : nil
        }
    }

    func selectRoute(_ route: AppRoute) {
        self.route = route
    }

    func restoreRoute(from data: Data) throws {
        selectRoute(try AppRoute.restored(from: data))
    }

    func adoptSidebarSelection(
        _ selection: SidebarSelection?,
        fallback: AppRoute = .overview
    ) {
        selectRoute(selection?.appRoute ?? fallback)
    }

    @discardableResult
    func selectAgent(_ agent: ProductAgentID?) -> Bool {
        guard let agent else {
            agentFilter = nil
            return true
        }
        guard ProductAgentID.projectAgents.contains(agent) else {
            return false
        }
        agentFilter = agent
        return true
    }

    func prewarm() async {
        await refreshProjectContext()
        guard !Task.isCancelled,
              projectContextErrorMessage == nil,
              activeProject != nil else {
            return
        }
        await loadProjectReadinessIfNeeded()
    }

    func refreshProjectContext() async {
        guard !Task.isCancelled else { return }
        contextRefreshTask?.cancel()
        contextRefreshGeneration &+= 1
        let generation = contextRefreshGeneration
        isLoadingProjectContext = true
        projectContextErrorMessage = nil

        let requestTask = Task { [service] in
            try await service.getProjectContext()
        }
        contextRefreshTask = requestTask
        let result = await withTaskCancellationHandler {
            await requestTask.result
        } onCancel: {
            requestTask.cancel()
        }

        guard generation == contextRefreshGeneration else {
            return
        }
        contextRefreshTask = nil
        isLoadingProjectContext = false
        switch result {
        case .success(let state):
            publishProjectContextState(state)
        case .failure(let error):
            guard !(error is CancellationError), !Task.isCancelled else {
                return
            }
            projectContextErrorMessage = error.localizedDescription
        }
    }

    func cancelProjectContextRefresh() {
        contextRefreshGeneration &+= 1
        contextRefreshTask?.cancel()
        contextRefreshTask = nil
        isLoadingProjectContext = false
    }

    func acceptProjectContextState(_ state: ProjectContextState) {
        contextRefreshGeneration &+= 1
        contextRefreshTask?.cancel()
        contextRefreshTask = nil
        isLoadingProjectContext = false
        projectContextErrorMessage = nil
        publishProjectContextState(state)
    }

    func loadProjectReadinessIfNeeded() async {
        guard !Task.isCancelled else { return }
        guard let key = currentReadinessKey else {
            readinessState = .empty
            return
        }
        if let entry = readinessCache[key] {
            touchReadinessCacheKey(key)
            readinessState = .accepted(entry)
            return
        }
        await refreshProjectReadiness()
    }

    func refreshProjectReadiness(sourceRevision: String? = nil) async {
        guard !Task.isCancelled else { return }
        guard let state = projectContextState,
              let project = state.active,
              let key = currentReadinessKey else {
            readinessState = .empty
            return
        }

        readinessRefreshTask?.cancel()
        readinessRefreshGeneration &+= 1
        let generation = readinessRefreshGeneration
        let previous = preferredReadinessEntry(for: key)
        readinessState = .refreshing(previous: previous)

        let requestTask = Task { [service] in
            try await service.getProjectReadiness(
                projectID: project.id,
                expectedProjectContextRevision: state.revision,
                sourceRevision: sourceRevision
            )
        }
        readinessRefreshTask = requestTask
        let result = await withTaskCancellationHandler {
            await requestTask.result
        } onCancel: {
            requestTask.cancel()
        }

        guard generation == readinessRefreshGeneration,
              currentReadinessKey == key else {
            return
        }
        readinessRefreshTask = nil
        switch result {
        case .success(let record):
            guard !Task.isCancelled else {
                restoreReadinessState(for: key)
                return
            }
            guard record.projectID == key.projectID else {
                failReadinessRefresh(
                    previous: previous,
                    message: "Project readiness does not match the active project."
                )
                return
            }
            let entry = ProjectReadinessCacheEntry(key: key, record: record)
            cacheReadinessEntry(entry)
            readinessState = .accepted(entry)
        case .failure(let error):
            guard !(error is CancellationError), !Task.isCancelled else {
                restoreReadinessState(for: key)
                return
            }
            failReadinessRefresh(previous: previous, message: error.localizedDescription)
        }
    }

    func cancelProjectReadinessRefresh() {
        readinessRefreshGeneration &+= 1
        readinessRefreshTask?.cancel()
        readinessRefreshTask = nil
        restoreReadinessStateForCurrentContext()
    }

    private var currentReadinessKey: ProjectReadinessCacheKey? {
        guard let state = projectContextState,
              let project = state.active,
              !project.id.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty,
              !state.revision.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty else {
            return nil
        }
        return ProjectReadinessCacheKey(
            projectID: project.id,
            projectContextRevision: state.revision
        )
    }

    private func publishProjectContextState(_ state: ProjectContextState) {
        guard !state.revision.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty else {
            projectContextErrorMessage = "Project context revision is unavailable."
            return
        }
        let previousKey = currentReadinessKey
        projectContextState = state
        projectContextErrorMessage = nil
        if previousKey != currentReadinessKey {
            readinessRefreshGeneration &+= 1
            readinessRefreshTask?.cancel()
            readinessRefreshTask = nil
        }
        restoreReadinessStateForCurrentContext()
    }

    private func preferredReadinessEntry(
        for key: ProjectReadinessCacheKey
    ) -> ProjectReadinessCacheEntry? {
        readinessCache[key] ?? lastReadinessEntry(for: key.projectID)
    }

    private func lastReadinessEntry(for projectID: String) -> ProjectReadinessCacheEntry? {
        readinessCacheOrder.reversed().lazy.compactMap { key in
            guard key.projectID == projectID else { return nil }
            return self.readinessCache[key]
        }.first
    }

    private func cacheReadinessEntry(_ entry: ProjectReadinessCacheEntry) {
        readinessCache[entry.key] = entry
        touchReadinessCacheKey(entry.key)
        while readinessCacheOrder.count > Self.maximumReadinessCacheEntries {
            let removed = readinessCacheOrder.removeFirst()
            readinessCache.removeValue(forKey: removed)
        }
    }

    private func touchReadinessCacheKey(_ key: ProjectReadinessCacheKey) {
        readinessCacheOrder.removeAll { $0 == key }
        readinessCacheOrder.append(key)
    }

    private func failReadinessRefresh(
        previous: ProjectReadinessCacheEntry?,
        message: String
    ) {
        if let previous {
            readinessState = .stale(previous, message: message)
        } else {
            readinessState = .failed(message: message)
        }
    }

    private func restoreReadinessState(for key: ProjectReadinessCacheKey) {
        if let exact = readinessCache[key] {
            readinessState = .accepted(exact)
        } else if let previous = lastReadinessEntry(for: key.projectID) {
            readinessState = .stale(previous, message: nil)
        } else {
            readinessState = .empty
        }
    }

    private func restoreReadinessStateForCurrentContext() {
        guard let key = currentReadinessKey else {
            readinessState = .empty
            return
        }
        restoreReadinessState(for: key)
    }
}
