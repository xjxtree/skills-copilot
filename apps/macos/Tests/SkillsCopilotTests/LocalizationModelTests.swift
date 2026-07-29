import Testing
import Foundation
@testable import SkillsCopilot

@Suite("LocalizationModelTests")
struct LocalizationModelTests {
    @Test("LocalizationModelTests")
    func run() throws {
        defer {
            UIStrings.use(.english)
        }

        try expectEqual(AppLanguage.fromStorage(nil), .english, "Missing language storage should default to English")
        try expectEqual(AppLanguage.fromStorage("en"), .english, "English language storage should parse")
        try expectEqual(AppLanguage.fromStorage("zh-Hans"), .simplifiedChinese, "Simplified Chinese language storage should parse")
        try expectEqual(AppLanguage.fromStorage("fr"), .english, "Unsupported language storage should default to English")
        try expectEqual(AppTheme.fromStorage(nil), .system, "Missing theme storage should follow the system")
        try expectEqual(AppTheme.fromStorage("light"), .light, "Light theme storage should parse")
        try expectEqual(AppTheme.fromStorage("dark"), .dark, "Dark theme storage should parse")
        try expectEqual(AppTheme.fromStorage("blue"), .system, "Unsupported theme storage should follow the system")
        try expectEqual(
            AgentIconCandidates.codex(bundlePath: "/Applications/ChatGPT.app").prefix(3).map(\.path),
            [
                "/Applications/ChatGPT.app/Contents/Resources/icon-codex-dark-color.png",
                "/Applications/ChatGPT.app/Contents/Resources/icon-codex-light.png",
                "/Applications/ChatGPT.app"
            ],
            "Codex icon candidates should prefer ChatGPT's dedicated Codex artwork and app bundle."
        )

        UIStrings.use(.english)
        try expectEqual(UIStrings.scan, "Scan", "English scan label should load from en resources")
        try expectEqual(UIStrings.appearanceSettings, "Appearance", "English appearance settings label should load")
        try expectEqual(UIStrings.themeFollowSystem, "Follow System", "English system theme label should load")
        try expectEqual(UIStrings.themeDark, "Dark", "English dark theme label should load")
        try expectEqual(
            UIStrings.configPreviewLine(3, "\"paths\": [\"<temp>\"]"),
            "Line 3: \"paths\": [\"<temp>\"]",
            "English config preview accessibility should expose the redacted line and its number."
        )
        try expectEqual(
            UIStrings.codexRestartRequired,
            "ChatGPT Codex or the Codex CLI may need to restart to read config.toml changes.",
            "English Codex restart guidance should name both current runtimes."
        )
        try expectEqual(UIStrings.scannedSkills(2), "Refreshed 2 skills across supported adapters.", "English formatted refresh summary should preserve arguments")

        UIStrings.use(.simplifiedChinese)
        let diagnostics = UIStrings.localizationResourceDiagnostics(for: .simplifiedChinese)
        if diagnostics.count == 0 {
            throw NativeModelTestFailure(description: "Chinese localization resources should be visible at runtime: paths=\(diagnostics.paths)")
        }
        try expectEqual(UIStrings.scan, "扫描", "Chinese scan label should load from zh-Hans resources")
        try expectEqual(UIStrings.appearanceSettings, "外观", "Chinese appearance settings label should load")
        try expectEqual(UIStrings.themeFollowSystem, "跟随系统", "Chinese system theme label should load")
        try expectEqual(UIStrings.themeDark, "黑色主题", "Chinese dark theme label should load")
        try expectEqual(
            UIStrings.codexRestartRequired,
            "ChatGPT 中的 Codex 或 Codex CLI 可能需要重启才能读取 config.toml 变更。",
            "Chinese Codex restart guidance should name both current runtimes."
        )
        try expectEqual(UIStrings.aiProviderSettings, "AI 提供方", "Chinese provider settings label should load")
        try expectEqual(UIStrings.service, "服务", "Chinese service label should load")
        try expectEqual(
            UIStrings.configPreviewLine(3, "\"paths\": [\"<temp>\"]"),
            "第 3 行：\"paths\": [\"<temp>\"]",
            "Chinese config preview accessibility should expose the redacted line and its number."
        )
        try expectEqual(UIStrings.scannedSkills(2), "已刷新受支持 adapter 中的 2 个技能。", "Chinese formatted refresh summary should preserve arguments")
        try expectEqual(
            UIStrings.localizedServiceMessage("Single request token limit is lower than the redacted prompt estimate."),
            "单次请求 token 限制低于脱敏提示词估算值。",
            "Provider token-limit blocker should be localized."
        )
        try expectEqual(
            UIStrings.localizedServiceMessage("budget_blocked: Single request token limit is lower than the prompt estimate."),
            "budget_blocked: 单次请求 token 限制低于提示词估算值。",
            "Provider error-code prefixes should preserve the code and localize the message."
        )
        try expectEqual(
            UIStrings.localizedServiceMessage("Provider profile `openai` exists but is disabled."),
            "提供方配置 `openai` 已存在，但当前已禁用。",
            "Provider profile status messages should preserve profile ids while localizing."
        )
        try expectEqual(
            UIStrings.localizedServiceMessage("Service call timed out before the sidecar returned a complete response."),
            "服务调用超时：sidecar 未在限定时间内返回完整响应。",
            "Sidecar timeout messages from stored history should be localized."
        )
        try expectEqual(
            ServiceClient.ClientError.processTimedOut.localizedDescription,
            "服务调用超时：sidecar 未在限定时间内返回完整响应。",
            "Sidecar timeout errors should use the selected app language."
        )
        try expectEqual(
            ServiceClient.ClientError.responseTooLarge(maxBytes: 16 * 1_024 * 1_024).localizedDescription,
            "服务响应超过 16 MiB 限制。",
            "Oversized sidecar responses should use the selected app language."
        )
        try expectEqual(
            ServiceDiagnosticSanitizer.displayMessage(" \n "),
            "服务退出，且没有可安全显示的诊断信息。",
            "Empty sidecar diagnostics should use a localized safe fallback."
        )
        try skillManagerChineseLocalizationDoesNotFallBackToEnglish()
        try taskCockpitElapsedSecondsHandlesBoundaries()

        UIStrings.use(.english)
        try expectEqual(UIStrings.scan, "Scan", "Switching back to English should not reuse cached Chinese values")
        try expectEqual(
            UIStrings.localizedServiceMessage("Single request token limit is lower than the redacted prompt estimate."),
            "Single request token limit is lower than the redacted prompt estimate.",
            "English service messages should stay readable."
        )
        try expectEqual(
            ServiceClient.ClientError.processTimedOut.localizedDescription,
            "Service call timed out before the sidecar returned a complete response.",
            "English sidecar timeout message should stay readable."
        )
        try expectEqual(
            ServiceClient.ClientError.responseTooLarge(maxBytes: 16 * 1_024 * 1_024).localizedDescription,
            "The service response exceeded the 16 MiB limit.",
            "English oversized-response errors should stay readable."
        )
    }

    private func taskCockpitElapsedSecondsHandlesBoundaries() throws {
        UIStrings.use(.english)
        try expectEqual(UIStrings.taskCockpitElapsedSeconds(-3), "Elapsed: 0 seconds.", "Negative elapsed time should clamp to zero.")
        try expectEqual(UIStrings.taskCockpitElapsedSeconds(0), "Elapsed: 0 seconds.", "Zero elapsed time should be readable.")
        try expectEqual(UIStrings.taskCockpitElapsedSeconds(1), "Elapsed: 1 second.", "Singular elapsed time should not use a plural noun.")
        try expectEqual(UIStrings.taskCockpitElapsedSeconds(2), "Elapsed: 2 seconds.", "Plural elapsed time should stay readable.")
    }

    private func skillManagerChineseLocalizationDoesNotFallBackToEnglish() throws {
        let requiredKeys = [
            "skillManager.targets",
            "skillManager.workflow.searchInstall",
            "skillManager.workflow.installedUpdates",
            "skillManager.workflow.localLibrary",
            "skillManager.toolUnavailable.title",
            "skillManager.toolUnavailable.message",
            "skillManager.searchInstall",
            "skillManager.search.networkBlocked",
            "skillManager.previewSummary.search",
            "skillManager.previewSummary.listInstalled",
            "skillManager.previewSummary.install",
            "skillManager.previewSummary.remove",
            "skillManager.previewSummary.update",
            "skillManager.previewSummary.localCreate",
            "skillManager.installed",
            "skillManager.removeSelected",
            "skillManager.localLibrary",
            "skillManager.previewInstall",
            "skillManager.previewRemove",
            "skillManager.previewUpdate",
            "skillManager.previewCreate",
            "skillManager.installSkillName",
            "skillManager.removeSkillName",
            "skillManager.localName"
        ]

        for key in requiredKeys {
            let value = UIStrings.text(key, "__missing__")
            try expectFalse(value == "__missing__", "\(key) should have a Chinese localization")
            try expectFalse(value.range(of: #"[A-Za-z]{4,}"#, options: .regularExpression) != nil, "\(key) should not fall back to English in Chinese UI: \(value)")
        }
    }
}
