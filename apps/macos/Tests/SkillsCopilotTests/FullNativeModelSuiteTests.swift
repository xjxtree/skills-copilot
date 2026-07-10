#if canImport(XCTest)
import XCTest
@testable import SkillsCopilot

final class FullNativeModelSuiteTests: XCTestCase {
    func testCompleteNativeModelRegistry() async throws {
        let summary = try await runAllNativeModelTestsAsync()
        XCTAssertEqual(summary.serviceSuiteCount, 2)
        XCTAssertEqual(summary.mainSuiteCount, 21)
        XCTAssertEqual(summary.skillStoreGroupCount, 64)
        XCTAssertEqual(summary.namedExecutionCount, 87)
    }
}
#endif
