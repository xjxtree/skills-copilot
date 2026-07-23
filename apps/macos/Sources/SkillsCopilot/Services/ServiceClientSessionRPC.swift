import Foundation

extension ServiceClient {
    func previewSessionResume(
        authorizedRoots: [String],
        agent: ProductAgentID,
        project: ProjectContext,
        sessionID: String,
        expectedSourceRevision: String,
        expectedSnapshotRevision: String
    ) async throws -> SessionContinuationRecord {
        guard ProductAgentID.projectAgents.contains(agent) else {
            throw ClientError.invalidOutput(
                "Tool-global does not own native sessions."
            )
        }
        return try await call(
            method: "session.previewResume",
            params: SessionResumePreviewParams(
                authorizedRoots: authorizedRoots,
                autoDiscover: authorizedRoots.isEmpty,
                agent: agent,
                projectRoot: project.rootPath,
                currentCWD: project.currentCWD ?? project.rootPath,
                sessionID: sessionID,
                expectedSourceRevision: expectedSourceRevision,
                expectedSnapshotRevision: expectedSnapshotRevision
            )
        )
    }

    func listLocalSessionMessages(
        authorizedRoots: [String],
        agent: String? = nil,
        project: ProjectContext? = nil,
        sessionID: String,
        limit: Int = 40,
        cursor: String? = nil,
        sourceRevision: String? = nil
    ) async throws -> LocalSessionMessagePageResult {
        let params = LocalSessionMessagePageParams(
            authorizedRoots: authorizedRoots,
            autoDiscover: authorizedRoots.isEmpty,
            agent: agent,
            projectRoot: project?.rootPath,
            currentCWD: project?.currentCWD,
            sessionID: sessionID,
            limit: limit,
            cursor: cursor,
            sourceRevision: sourceRevision
        )
        return try await call(method: "session.listLocalSessionMessages", params: params)
    }

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
        return try await call(method: "session.previewLocalSessions", params: params)
    }

}
