import Foundation

extension ServiceClient {
    func previewLocalSessions(
        authorizedRoots: [String],
        agent: String? = nil,
        scope: LocalSessionScopeFilter = .project,
        search: String? = nil,
        project: ProjectContext? = nil,
        sessionID: String? = nil,
        includeContentItems: Bool? = nil,
        limit: Int = 20,
        offset: Int? = nil,
        cursor: String? = nil,
        sourceRevision: String? = nil,
        sort: LocalSessionSortOrder = .recent,
        direction: SkillSortDirection = .descending
    ) async throws -> LocalSessionPreviewResult {
        let normalizedSearch = search?.trimmingCharacters(in: .whitespacesAndNewlines)
        let params = LocalSessionPreviewParams(
            authorizedRoots: authorizedRoots,
            autoDiscover: authorizedRoots.isEmpty,
            agent: agent,
            scope: scope.rawValue,
            search: normalizedSearch?.isEmpty == true ? nil : normalizedSearch,
            projectRoot: project?.rootPath,
            currentCWD: project?.currentCWD,
            sessionID: sessionID,
            includeContentItems: includeContentItems,
            limit: limit,
            offset: offset,
            cursor: cursor,
            sourceRevision: sourceRevision,
            sort: sort.rawValue,
            direction: direction == .ascending ? "asc" : "desc",
            maxFiles: nil,
            maxExcerptChars: 1000
        )
        do {
            return try await call(method: "session.previewLocalSessions", params: params)
        } catch ClientError.service(let error) where error.code == "unknown_method" {
            return .unavailable()
        }
    }

}
