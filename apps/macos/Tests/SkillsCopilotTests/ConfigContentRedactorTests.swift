import Testing
@testable import SkillsCopilot

@Suite("ConfigContentRedactorTests")
struct ConfigContentRedactorTests {
    @Test("ConfigContentRedactorTests")
    func run() throws {
        try redactsNestedJSONSecretKeys()
        try redactsSimpleAssignmentSecretKeys()
        try redactsNestedJSONLocalPathsWithoutChangingNetworkURLs()
        try redactsLocalPathsInNonJSONFallbackContent()
    }

    private func redactsNestedJSONSecretKeys() throws {
        let authTokenKey = ["ANTHROPIC", "_AUTH", "_TOKEN"].joined()
        let tokenValue = "fixture-token-value"
        let apiValue = "fixture-api-value"
        let content = """
        {
          "env": {
            "\(authTokenKey)": "\(tokenValue)",
            "ANTHROPIC_BASE_URL": "https://example.invalid/v1"
          },
          "apiKey": "\(apiValue)"
        }
        """

        let redacted = ConfigContentRedactor.redactedForDisplay(content)

        try expectFalse(redacted.contains(tokenValue), "JSON config preview must hide token values.")
        try expectFalse(redacted.contains(apiValue), "JSON config preview must hide apiKey values.")
        try expectContains(redacted, ConfigContentRedactor.redactedValue, "JSON config preview should show redaction placeholders.")
        try expectContains(redacted, "ANTHROPIC_BASE_URL", "Non-sensitive config keys should remain visible.")
    }

    private func redactsSimpleAssignmentSecretKeys() throws {
        let apiKey = ["OPENAI", "_API", "_KEY"].joined()
        let accessTokenKey = ["access", "_token"].joined()
        let apiValue = "fixture-key-value"
        let tokenValue = "fixture-token-value"
        let content = """
        \(apiKey)=\(apiValue)
        profile: local
        \(accessTokenKey): \(tokenValue),
        """

        let redacted = ConfigContentRedactor.redactedForDisplay(content)

        try expectFalse(redacted.contains(apiValue), "Assignment config preview must hide API key values.")
        try expectFalse(redacted.contains(tokenValue), "Assignment config preview must hide token values.")
        try expectContains(redacted, "profile: local", "Non-sensitive assignment lines should remain visible.")
    }

    private func redactsNestedJSONLocalPathsWithoutChangingNetworkURLs() throws {
        let homePath = "/" + "Users" + "/privacy-review/private-project/.claude/skills"
        let tempPath = [
            "", "private", "var", "folders", "aa", "bb", "T",
            "private-fixture", "config.json",
        ].joined(separator: "/")
        let arbitraryAbsolutePath = "/" + "opt" + "/private-tool/config.json"
        let windowsPath = "C:" + #"\"# + "Users" + #"\"# + "privacy-review" + #"\"# + "config.json"
        let networkURL = "https://example.invalid/v1/models"
        let content = """
        {
          "skills": {
            "paths": [
              "\(homePath)",
              "\(tempPath)",
              "\(arbitraryAbsolutePath)"
            ]
          },
          "windowsPath": "\(windowsPath.replacingOccurrences(of: #"\"#, with: #"\\"#))",
          "endpoint": "\(networkURL)"
        }
        """

        let redacted = ConfigContentRedactor.redactedForDisplay(content)

        try expectFalse(redacted.contains(homePath), "JSON config preview must hide absolute home paths.")
        try expectFalse(redacted.contains(tempPath), "JSON config preview must hide temporary fixture paths.")
        try expectFalse(redacted.contains(arbitraryAbsolutePath), "JSON config preview must hide arbitrary absolute local paths.")
        try expectFalse(redacted.contains("privacy-review"), "JSON config preview must not retain local account or fixture names.")
        try expectFalse(redacted.contains("private-fixture"), "JSON config preview must not retain unique temporary directory names.")
        try expectContains(redacted, "$HOME", "Known home roots should retain a useful privacy-safe placeholder.")
        try expectContains(redacted, "<temp>", "Temporary roots should collapse to a stable placeholder.")
        try expectContains(redacted, ConfigContentRedactor.redactedPathValue, "Other local roots should use a stable placeholder.")
        try expectContains(redacted, networkURL, "Network URLs are destinations, not local filesystem paths.")
    }

    private func redactsLocalPathsInNonJSONFallbackContent() throws {
        let localPath = [
            "", "private", "tmp", "private-fixture", "settings.yaml",
        ].joined(separator: "/")
        let localFileURL = "file://" + localPath
        let networkURL = "https://example.invalid/config"
        let content = """
        config_path: \(localPath)
        config_url: \(localFileURL)
        endpoint: \(networkURL)
        profile: local
        """

        let redacted = ConfigContentRedactor.redactedForDisplay(content)

        try expectFalse(redacted.contains(localPath), "Fallback config preview must hide absolute local paths.")
        try expectFalse(redacted.contains(localFileURL), "Fallback config preview must hide local file URLs.")
        try expectContains(redacted, ConfigContentRedactor.redactedPathValue, "Fallback path values should use the local-path placeholder.")
        try expectContains(redacted, ConfigContentRedactor.redactedFileURLValue, "Local file URLs should use a distinct placeholder.")
        try expectContains(redacted, networkURL, "Fallback redaction must preserve network URLs.")
        try expectContains(redacted, "profile: local", "Fallback redaction must preserve ordinary config content.")
    }
}
