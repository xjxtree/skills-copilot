import Foundation
import Darwin

struct NativeModelTestFailure: Error, CustomStringConvertible {
    let description: String
}

func expectEqual<T: Equatable>(_ actual: T, _ expected: T, _ label: String) throws {
    if actual != expected {
        throw NativeModelTestFailure(description: "\(label): \(actual) != \(expected)")
    }
}

func expectFalse(_ value: Bool, _ label: String) throws {
    if value {
        throw NativeModelTestFailure(description: "\(label): expected false")
    }
}

func expectNil<T>(_ value: T?, _ label: String) throws {
    if let value {
        throw NativeModelTestFailure(description: "\(label): expected nil, got \(value)")
    }
}

func expectContains(_ value: String?, _ expected: String, _ label: String) throws {
    guard let value, value.contains(expected) else {
        throw NativeModelTestFailure(description: "\(label): expected \(String(describing: value)) to contain \(expected)")
    }
}

func runAsyncTest(_ body: @escaping () async throws -> Void) throws {
    let resultQueue = DispatchQueue(label: "com.agent-copilot.native-model-test-result")
    var result: Result<Void, Error>?

    Task {
        let completed: Result<Void, Error>
        do {
            try await body()
            completed = .success(())
        } catch {
            completed = .failure(error)
        }

        resultQueue.sync {
            result = completed
        }
    }

    var completed: Result<Void, Error>?
    while completed == nil {
        completed = resultQueue.sync { result }
        if completed == nil {
            RunLoop.current.run(mode: .default, before: Date().addingTimeInterval(0.01))
        }
    }

    try completed?.get()
}

func runNamed(_ name: String, _ body: () throws -> Void) throws {
    fputs("SkillsCopilotTests: \(name) start\n", stderr)
    fflush(stderr)
    try body()
    fputs("SkillsCopilotTests: \(name) ok\n", stderr)
    fflush(stderr)
}

func runAsyncNamed(_ name: String, _ body: () async throws -> Void) async throws {
    fputs("SkillsCopilotTests: \(name) start\n", stderr)
    fflush(stderr)
    try await body()
    fputs("SkillsCopilotTests: \(name) ok\n", stderr)
    fflush(stderr)
}

func runNativeModelTestsAsync() async throws {
    fputs("SkillsCopilotTests: native model runner start\n", stderr)
    fflush(stderr)

    let suite = ProcessInfo.processInfo.environment["SKILLS_COPILOT_NATIVE_MODEL_TEST_SUITE"] ?? "main"
    if suite == "service-process" {
        try await runAsyncNamed("ServiceClientProcessTests") {
            try await ServiceClientProcessTests().run()
        }
        fputs("SkillsCopilotTests: native service process model checks passed\n", stderr)
        fflush(stderr)
        return
    }
    if suite == "service-rpc" {
        try await runAsyncNamed("ServiceClientRPCTests") {
            try await ServiceClientRPCTests().run()
        }
        fputs("SkillsCopilotTests: native service RPC model checks passed\n", stderr)
        fflush(stderr)
        return
    }
    if suite.hasPrefix("skill-store-") {
        let rawGroup = String(suite.dropFirst("skill-store-".count))
        guard let group = Int(rawGroup) else {
            throw NativeModelTestFailure(description: "Invalid SkillStore native model test group: \(rawGroup)")
        }
        let groupCount = Int(ProcessInfo.processInfo.environment["SKILLS_COPILOT_SKILL_STORE_GROUP_COUNT"] ?? "") ?? 64
        try await runAsyncNamed("SkillStoreTests group \(group)") {
            try await SkillStoreTests(selectedGroup: group, groupCount: groupCount).run()
        }
        fputs("SkillsCopilotTests: native SkillStore model group \(group) checks passed\n", stderr)
        fflush(stderr)
        return
    }

    guard suite == "main" else {
        throw NativeModelTestFailure(description: "Unknown native model test suite: \(suite)")
    }

    try runNamed("FindingDisplayModelTests") { try FindingDisplayModelTests().run() }
    try runNamed("FindingExplainabilityModelTests") { try FindingExplainabilityModelTests().run() }
    try runNamed("RuleTuningModelTests") { try RuleTuningModelTests().run() }
    try runNamed("ProviderObservabilityModelTests") { try ProviderObservabilityModelTests().run() }
    try runNamed("TaskCockpitModelTests") { try TaskCockpitModelTests().run() }
    try runNamed("TaskCockpitHistoryStoreTests") { try TaskCockpitHistoryStoreTests().run() }
    try runNamed("TaskInputModelTests") { try TaskInputModelTests().run() }
    try runNamed("AIProviderModelTests") { try AIProviderModelTests().run() }
    try runNamed("LLMModelTests") { try LLMModelTests().run() }
    try runNamed("ScriptExecutionModelTests") { try ScriptExecutionModelTests().run() }
    try runNamed("ToolGlobalModelTests") { try ToolGlobalModelTests().run() }
    try runNamed("SkillManagerModelTests") { try SkillManagerModelTests().run() }
    try runNamed("AgentConfigTimelineModelTests") { try AgentConfigTimelineModelTests().run() }
    try runNamed("ConfigContentRedactorTests") { try ConfigContentRedactorTests().run() }
    try runNamed("LocalizationModelTests") { try LocalizationModelTests().run() }
    try runNamed("UIOptimizationModelTests") { try UIOptimizationModelTests().run() }
    try runNamed("MainWindowModelTests") { try MainWindowModelTests().run() }
    try runNamed("LocalSessionPreviewModelTests") { try LocalSessionPreviewModelTests().run() }
    try runNamed("SkillListModelTests") { try SkillListModelTests().run() }
    fputs("SkillsCopilotTests: native non-store model checks passed\n", stderr)
    fflush(stderr)
}

func runNativeModelTestsMain() async {
    do {
        try await runNativeModelTestsAsync()
        _exit(0)
    } catch {
        fputs("SkillsCopilotTests: \(error)\n", stderr)
        exit(1)
    }
}

@_cdecl("SkillsCopilotRunNativeModelTests")
public func runNativeModelTests() {
    do {
        try runAsyncTest {
            try await runNativeModelTestsAsync()
        }
        _exit(0)
    } catch {
        fputs("SkillsCopilotTests: \(error)\n", stderr)
        exit(1)
    }
}
