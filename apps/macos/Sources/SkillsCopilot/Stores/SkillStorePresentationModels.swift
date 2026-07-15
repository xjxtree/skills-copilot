import Foundation

struct FilteredSkillListCacheKey: Equatable {
    let dataRevision: Int
    let searchText: String
    let agentFilter: String
    let stateFilter: String
    let scopeFilter: String
    let sortOrder: String
    let sortDirection: String
}

struct FilteredSkillListCache {
    let key: FilteredSkillListCacheKey
    let result: FilteredSkillListResult
}

struct FilteredSkillListResult {
    let skills: [SkillRecord]
    let issueCountsBySkillID: [SkillRecord.ID: Int]
    let conflictCountsBySkillID: [SkillRecord.ID: Int]

    func issueCount(for skillID: SkillRecord.ID) -> Int {
        issueCountsBySkillID[skillID] ?? 0
    }

    func conflictCount(for skillID: SkillRecord.ID) -> Int {
        conflictCountsBySkillID[skillID] ?? 0
    }
}

struct ScopedLocalSessionSummary {
    let rows: [LocalSessionPreviewRow]
    let userMessageCount: Int
    let totalMessageCount: Int
    let toolCallCount: Int
    let skillCallCount: Int
}

struct ScopedLocalSessionSummaryCache {
    let revision: Int
    let summary: ScopedLocalSessionSummary
}

struct AppStartupLoadingState: Equatable {
    let message: String
    let progress: Double

    init(message: String, progress: Double) {
        self.message = message
        self.progress = min(max(progress, 0), 1)
    }
}

struct SkillListScrollRequest: Equatable {
    let skillID: SkillRecord.ID
    let token = UUID()
}

struct TaskCockpitPromptConfirmation: Identifiable, Hashable {
    let preview: LLMPromptPreview
    let taskText: String
    let agentIDs: [String]
    let instanceIDs: [String]

    var id: String {
        preview.previewID.isEmpty ? taskText : preview.previewID
    }
}

struct ProviderActivityFilterKey: Hashable {
    let provider: String?
    let model: String?
    let action: String?
    let windowDays: Int?
    let startAt: Int?
    let endAt: Int?
}
