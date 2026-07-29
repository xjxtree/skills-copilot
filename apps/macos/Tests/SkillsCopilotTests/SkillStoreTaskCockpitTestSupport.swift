import Foundation
@testable import SkillsCopilot

final class TaskCockpitFallbackServiceRunner: ServiceProcessRunning {
    private let recorder = TaskCockpitFallbackCallRecorder()

    func run(executableURL: URL, input: Data, timeoutNanoseconds: UInt64?) async throws -> Data {
        let rawInput = String(data: input, encoding: .utf8) ?? ""
        await recorder.record(rawInput)

        let object = try JSONSerialization.jsonObject(with: input) as? [String: Any]
        let method = object?["method"] as? String ?? ""
        switch method {
        case "app.stateSnapshot":
            return Data(Self.stateSnapshotResponse.utf8)
        case "llm.previewPrompt":
            return Data(Self.unknownPreviewPromptResponse.utf8)
        default:
            return Data(Self.unexpectedMethodResponse(method).utf8)
        }
    }

    func calls() async -> String {
        await recorder.calls()
    }

    private static let stateSnapshotResponse = """
    {"id":"test","ok":true,"result":{"status":{"protocol_version":1,"version":"test","app_data_dir":"/tmp/skills-copilot","catalog_path":"/tmp/skills-copilot/catalog.sqlite","user_home":"/tmp/home","supported_methods":["app.stateSnapshot","llm.previewPrompt"]},"skills":[{"id":"alpha","agent":"claude-code","scope":"agent-global","path":"/tmp/global/alpha/SKILL.md","display_path":"/tmp/global/alpha/SKILL.md","definition_id":"def.alpha","name":"Alpha","state":"loaded","enabled":true},{"id":"beta","agent":"claude-code","scope":"agent-project","path":"/tmp/project/beta/SKILL.md","display_path":"/tmp/project/beta/SKILL.md","definition_id":"def.beta","name":"Beta","state":"loaded","enabled":true}],"findings":[],"conflicts":[]}}
    """

    private static let unknownPreviewPromptResponse = """
    {"id":"test","ok":false,"result":null,"error":{"code":"unknown_method","message":"unknown method: llm.previewPrompt"}}
    """

    private static func unexpectedMethodResponse(_ method: String) -> String {
        """
        {"id":"test","ok":false,"result":null,"error":{"code":"unexpected_method","message":"unexpected method: \(method)"}}
        """
    }
}

private actor TaskCockpitFallbackCallRecorder {
    private var recordedCalls: [String] = []

    func record(_ rawInput: String) {
        recordedCalls.append(rawInput)
    }

    func calls() -> String {
        recordedCalls.joined(separator: "\n")
    }
}
