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
        resultQueue.sync { result = completed }
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

private let mainNativeModelSuites: [(String, () throws -> Void)] = [
    ("FindingDisplayModelTests", { try FindingDisplayModelTests().run() }),
    ("FindingExplainabilityModelTests", { try FindingExplainabilityModelTests().run() }),
    ("RuleTuningModelTests", { try RuleTuningModelTests().run() }),
    ("ProviderObservabilityModelTests", { try ProviderObservabilityModelTests().run() }),
    ("TaskCockpitModelTests", { try TaskCockpitModelTests().run() }),
    ("TaskInputModelTests", { try TaskInputModelTests().run() }),
    ("AIProviderModelTests", { try AIProviderModelTests().run() }),
    ("LLMModelTests", { try LLMModelTests().run() }),
    ("LegacyPrivacyCleanupTests", { try runAsyncTest { try await LegacyPrivacyCleanupTests().run() } }),
    ("ConfirmedMutationLaneTests", { try runAsyncTest { try await ConfirmedMutationLaneTests().run() } }),
    ("ScriptExecutionModelTests", { try ScriptExecutionModelTests().run() }),
    ("ToolGlobalModelTests", { try ToolGlobalModelTests().run() }),
    ("SkillManagerModelTests", { try SkillManagerModelTests().run() }),
    ("SkillManagerRequestGenerationTests", { try runAsyncTest { try await SkillManagerRequestGenerationTests().run() } }),
    ("AgentConfigTimelineModelTests", { try AgentConfigTimelineModelTests().run() }),
    ("ConfigContentRedactorTests", { try ConfigContentRedactorTests().run() }),
    ("LocalizationModelTests", { try LocalizationModelTests().run() }),
    ("ScanResultCompatibilityTests", { try ScanResultCompatibilityTests().run() }),
    ("UIOptimizationModelTests", { try UIOptimizationModelTests().run() }),
    ("MainWindowModelTests", { try MainWindowModelTests().run() }),
    ("LocalSessionPreviewModelTests", { try LocalSessionPreviewModelTests().run() }),
    ("LocalSessionCacheTests", { try runAsyncTest { try await LocalSessionCacheTests().run() } }),
    ("SkillListModelTests", { try SkillListModelTests().run() }),
    ("ListCompletenessModelTests", { try ListCompletenessModelTests().run() }),
    ("ProductReadProjectionModelTests", {
        try runAsyncTest { try await ProductReadProjectionModelTests().run() }
    }),
    ("AppContextStoreTests", {
        try runAsyncTest { try await AppContextStoreTests.run() }
    }),
    ("SkillWorkspaceStoreTests", {
        try runAsyncTest { try await SkillWorkspaceStoreTests.run() }
    }),
    ("SessionWorkspaceStoreTests", {
        try runAsyncTest { try await SessionWorkspaceStoreTests.runRegisteredSuite() }
    }),
    ("SessionWorkspaceListPresentationTests", {
        try SessionWorkspaceListPresentationTests.run()
    }),
    ("SessionWorkspaceDetailPresentationTests", {
        try SessionWorkspaceDetailPresentationTests.run()
    }),
    ("ProjectOverviewPresentationTests", {
        try ProjectOverviewPresentationTests.run()
    }),
    ("SkillsWorkspaceListPresentationTests", {
        try SkillsWorkspaceListPresentationTests.run()
    }),
    ("SkillAggregateDetailPresentationTests", {
        try SkillAggregateDetailPresentationTests.run()
    }),
    ("SkillManagerEntryContextTests", {
        try SkillManagerEntryContextTests().run()
    }),
]

struct NativeModelSuiteSummary: Equatable {
    let serviceSuiteCount: Int
    let mainSuiteCount: Int
    let skillStoreGroupCount: Int
    let namedExecutionCount: Int
}

func runAllNativeModelTestsAsync() async throws -> NativeModelSuiteSummary {
    fputs("SkillsCopilotTests: native model runner start\n", stderr)
    fflush(stderr)
    var namedExecutionCount = 0

    try await runAsyncNamed("ServiceClientProcessTests") {
        try await ServiceClientProcessTests().run()
    }
    namedExecutionCount += 1
    try await runAsyncNamed("ServiceClientRPCTests") {
        try await ServiceClientRPCTests().run()
    }
    namedExecutionCount += 1

    for (name, run) in mainNativeModelSuites {
        try runNamed(name, run)
        namedExecutionCount += 1
    }

    let groupCount = 64
    for group in 0..<groupCount {
        try await runAsyncNamed("SkillStoreTests group \(group)") {
            try await SkillStoreTests(selectedGroup: group, groupCount: groupCount).run()
        }
        namedExecutionCount += 1
    }

    let summary = NativeModelSuiteSummary(
        serviceSuiteCount: 2,
        mainSuiteCount: mainNativeModelSuites.count,
        skillStoreGroupCount: groupCount,
        namedExecutionCount: namedExecutionCount
    )
    fputs(
        "SkillsCopilotTests: full-suite-complete service=\(summary.serviceSuiteCount) main=\(summary.mainSuiteCount) skill-store-groups=\(summary.skillStoreGroupCount) named=\(summary.namedExecutionCount)\n",
        stderr
    )
    fflush(stderr)
    return summary
}

@_cdecl("SkillsCopilotRunNativeModelTests")
public func runNativeModelTestsFromSwiftPMFallback() {
#if !canImport(XCTest)
    do {
        try runAsyncTest {
            let summary = try await runAllNativeModelTestsAsync()
            try expectEqual(summary.serviceSuiteCount, 2, "Service suite count")
            try expectEqual(summary.mainSuiteCount, 34, "Main suite count")
            try expectEqual(summary.skillStoreGroupCount, 64, "SkillStore group count")
            try expectEqual(summary.namedExecutionCount, 100, "Named execution count")
        }
    } catch {
        fputs("SkillsCopilotTests: \(error)\n", stderr)
        exit(1)
    }
#endif
}
