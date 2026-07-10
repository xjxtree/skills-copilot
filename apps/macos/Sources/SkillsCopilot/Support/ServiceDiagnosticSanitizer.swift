import Foundation

enum ServiceDiagnosticSanitizer {
    static let maximumDisplayCharacters = 512

    static func displayMessage(_ raw: String) -> String {
        var sanitized = ConfigContentRedactor.redactedForDisplay(raw)
        // Preserve a following credential assignment for the next non-overlapping match.
        sanitized = replacing(
            pattern: #"(?i)\b(API_KEY|TOKEN|SECRET|PASSWORD)=(?:\s*(?!(?:API_KEY|TOKEN|SECRET|PASSWORD)=)(?:"[^"]*"|'[^']*'|[^\s]+))?"#,
            in: sanitized,
            with: "$1=<redacted>"
        )
        sanitized = replacing(
            pattern: #"(?i)\bsk-[A-Za-z0-9_-]{20,}"#,
            in: sanitized,
            with: "<redacted-token>"
        )
        for pathComponents in [
            ["Users"],
            ["home"],
            ["private", "var"],
            ["var", "folders"]
        ] {
            sanitized = replacing(
                pattern: absolutePathPattern(pathComponents),
                in: sanitized,
                with: "<redacted-path>"
            )
        }

        let collapsed = sanitized
            .components(separatedBy: .whitespacesAndNewlines)
            .filter { !$0.isEmpty }
            .joined(separator: " ")
        guard !collapsed.isEmpty else {
            return UIStrings.text(
                "service.error.sidecarDiagnosticUnavailable",
                "The service exited without a safe diagnostic."
            )
        }
        return String(collapsed.prefix(maximumDisplayCharacters))
    }

    private static func replacing(
        pattern: String,
        in value: String,
        with replacement: String
    ) -> String {
        guard let expression = try? NSRegularExpression(pattern: pattern) else {
            return value
        }
        let range = NSRange(value.startIndex..<value.endIndex, in: value)
        return expression.stringByReplacingMatches(
            in: value,
            range: range,
            withTemplate: replacement
        )
    }

    private static func absolutePathPattern(_ components: [String]) -> String {
        "/" + components.joined(separator: "/") + #"/[^\s]+"#
    }
}
