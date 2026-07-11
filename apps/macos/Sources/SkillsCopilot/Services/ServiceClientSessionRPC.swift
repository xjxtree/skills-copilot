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
        offset: Int? = 0,
        pagingMode: String? = nil,
        cursor: String? = nil,
        sourceRevision: String? = nil,
        sort: LocalSessionSortOrder = .recent,
        direction: SkillSortDirection = .descending,
        maxFiles: Int? = 800
    ) async throws -> LocalSessionPreviewResult {
        let normalizedSearch = search?.trimmingCharacters(in: .whitespacesAndNewlines)
        let effectivePagingMode = pagingMode ?? (cursor == nil ? nil : "keyset")
        let usesKeysetPaging = effectivePagingMode == "keyset"
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
            offset: usesKeysetPaging ? nil : offset,
            pagingMode: effectivePagingMode,
            cursor: cursor,
            sourceRevision: sourceRevision,
            sort: sort.rawValue,
            direction: direction == .ascending ? "asc" : "desc",
            maxFiles: usesKeysetPaging ? nil : maxFiles,
            maxExcerptChars: 1000
        )
        do {
            return try await call(method: "session.previewLocalSessions", params: params)
        } catch ClientError.service(let error) where error.code == "unknown_method" {
            return .unavailable()
        }
    }

}
