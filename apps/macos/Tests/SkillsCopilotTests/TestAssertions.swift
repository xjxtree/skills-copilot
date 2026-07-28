import Foundation

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
        throw NativeModelTestFailure(
            description: "\(label): expected \(String(describing: value)) to contain \(expected)"
        )
    }
}
