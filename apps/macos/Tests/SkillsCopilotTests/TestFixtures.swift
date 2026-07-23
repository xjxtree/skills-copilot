import Foundation
@testable import SkillsCopilot

final class WeakReference<Object: AnyObject> {
    weak var value: Object?

    init(_ value: Object?) {
        self.value = value
    }
}

func skill(
    id: String,
    agent: String = "claude-code",
    scope: String,
    path: String,
    definitionId: String,
    name: String,
    state: String = "loaded",
    enabled: Bool = true
) -> SkillRecord {
    SkillRecord(
        id: id,
        agent: agent,
        scope: scope,
        path: path,
        displayPath: path,
        definitionId: definitionId,
        name: name,
        state: state,
        enabled: enabled
    )
}

func emptySkillManagerSearchRecord() -> SkillManagerSearchRecord {
    SkillManagerSearchRecord(
        preview: SkillManagerCommandPreview(
            toolId: "npx-skills",
            operation: "search",
            command: ["npx", "skills", "find", "missing"],
            cwd: "/tmp",
            env: [],
            requiresConfirmation: true,
            confirmed: false,
            networkRequired: true,
            networkAllowed: true,
            willRun: false,
            previewToken: "search:missing",
            summary: "Search",
            risks: [],
            source: nil,
            skills: []
        ),
        output: nil,
        results: [],
        readback: nil,
        returnedCount: 0,
        totalCount: nil,
        hasMore: false,
        nextCursor: nil,
        sourceCompleteness: .unknown,
        incompleteReason: .sourceLimited
    )
}
