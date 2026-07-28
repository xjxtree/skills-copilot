import Foundation

enum SkillStatusKind: Int {
    case broken
    case missing
    case disabled
    case enabled
    case shadowed
    case unknown
}

enum DisplayText {
    static let screenshotPrivacyModeStorageKey = "privacy.screenshotMode"

    private static let snapshotDateFormatter: DateFormatter = {
        let formatter = DateFormatter()
        formatter.dateStyle = .medium
        formatter.timeStyle = .short
        return formatter
    }()

    static func scope(_ value: String) -> String {
        switch value {
        case "agent-global":
            return UIStrings.text("scope.agentGlobal", "Agent Global")
        case "agent-project":
            return UIStrings.text("scope.project", "Project")
        case "agent-external":
            return UIStrings.text("scope.agentExternal", "External")
        case "tool-global":
            return UIStrings.text("scope.toolGlobal", "Tool Global")
        default:
            if value.trimmingCharacters(in: .whitespacesAndNewlines).lowercased().contains("external") {
                return UIStrings.text("scope.external", "External")
            }
            return value
        }
    }

    static func scope(_ value: String, agent: String) -> String {
        let normalizedAgent = agent
            .trimmingCharacters(in: .whitespacesAndNewlines)
            .lowercased()
            .replacingOccurrences(of: "_", with: "-")
        let normalizedScope = value
            .trimmingCharacters(in: .whitespacesAndNewlines)
            .lowercased()
            .replacingOccurrences(of: "_", with: "-")
        if normalizedAgent == "openclaw", normalizedScope.contains("project") {
            return UIStrings.openClawWorkspaceScope
        }
        return scope(value)
    }

    static func scope(for skill: SkillRecord) -> String {
        scope(skill.scope, agent: skill.agent)
    }

    static func agent(_ value: String) -> String {
        switch value {
        case "claude-code":
            return UIStrings.claudeCode
        case "codex":
            return UIStrings.codex
        case "opencode":
            return UIStrings.opencode
        case "pi":
            return UIStrings.pi
        case "hermes":
            return UIStrings.hermes
        case "openclaw":
            return UIStrings.openclaw
        default:
            return value
        }
    }

    static func state(_ value: String, enabled: Bool) -> String {
        switch statusKind(value, enabled: enabled) {
        case .enabled:
            return UIStrings.stateEnabled
        case .disabled:
            return UIStrings.stateDisabled
        case .broken:
            return UIStrings.stateBroken
        case .missing:
            return UIStrings.stateMissing
        case .shadowed:
            return UIStrings.stateShadowed
        case .unknown:
            let trimmed = value.trimmingCharacters(in: .whitespacesAndNewlines)
            if trimmed.isEmpty {
                return UIStrings.stateUnknown
            }
            return UIStrings.stateUnknownValue(trimmed)
        }
    }

    static func statusKind(_ value: String, enabled: Bool) -> SkillStatusKind {
        switch value.trimmingCharacters(in: .whitespacesAndNewlines).lowercased() {
        case "broken":
            return .broken
        case "missing":
            return .missing
        case "disabled":
            return .disabled
        case "loaded":
            return enabled ? .enabled : .disabled
        case "shadowed":
            return .shadowed
        default:
            return .unknown
        }
    }

    static func stateSortRank(_ value: String, enabled: Bool) -> Int {
        statusKind(value, enabled: enabled).rawValue
    }

    static func stateSystemImage(_ value: String, enabled: Bool) -> String {
        switch statusKind(value, enabled: enabled) {
        case .enabled:
            return "checkmark.circle.fill"
        case .disabled:
            return "pause.circle"
        case .broken:
            return "exclamationmark.triangle.fill"
        case .missing:
            return "questionmark.folder.fill"
        case .shadowed:
            return "rectangle.stack.badge.minus"
        case .unknown:
            return "questionmark.circle"
        }
    }

    static func catalogToggleDisabledReason(for skill: SkillRecord, isWriting: Bool) -> String? {
        if isWriting {
            return UIStrings.toggleUnavailableBusy
        }

        switch statusKind(skill.state, enabled: skill.enabled) {
        case .enabled, .disabled:
            break
        case .broken:
            return UIStrings.toggleUnavailableBroken
        case .missing:
            return UIStrings.toggleUnavailableMissing
        case .shadowed:
            return UIStrings.toggleUnavailableShadowed
        case .unknown:
            return UIStrings.toggleUnavailableUnknown
        }

        if isToolGlobal(skill) {
            return UIStrings.toggleUnavailableToolGlobal
        }

        return nil
    }

    static func toggleDisabledReason(for skill: SkillRecord, isWriting: Bool) -> String? {
        if let catalogReason = catalogToggleDisabledReason(for: skill, isWriting: isWriting) {
            return catalogReason
        }

        if requiresGuardedToggleCapability(skill.agent) {
            return guardedToggleBoundary(for: skill.agent)
        }

        if isReadOnlyAdapter(skill.agent) {
            return UIStrings.toggleUnavailableReadOnlyAdapter(agent(skill.agent))
        }

        return nil
    }

    static func isReadOnlyAdapter(_ agent: String) -> Bool {
        !["claude-code", "codex", "opencode", "pi", "hermes", "openclaw"].contains(normalizedAgent(agent))
    }

    static func requiresGuardedToggleCapability(_ agent: String) -> Bool {
        ["pi", "hermes", "openclaw"].contains(normalizedAgent(agent))
    }

    static func guardedToggleBoundary(for agent: String) -> String {
        switch normalizedAgent(agent) {
        case "hermes":
            return UIStrings.hermesGuardedToggleBoundary
        case "openclaw":
            return UIStrings.openClawGuardedToggleBoundary
        case "pi":
            return UIStrings.piGuardedToggleBoundary
        default:
            return UIStrings.guardedToggleBoundary(DisplayText.agent(agent))
        }
    }

    static func isToolGlobal(_ skill: SkillRecord) -> Bool {
        skill.scope == "tool-global"
    }

    static func isReadOnlyPreview(_ skill: SkillRecord) -> Bool {
        isToolGlobal(skill) || isReadOnlyAdapter(skill.agent)
    }

    private static func normalizedAgent(_ agent: String) -> String {
        agent
            .trimmingCharacters(in: .whitespacesAndNewlines)
            .lowercased()
            .replacingOccurrences(of: "_", with: "-")
    }

    static func timestamp(_ milliseconds: Int64) -> String {
        let date = Date(timeIntervalSince1970: TimeInterval(milliseconds) / 1_000)
        return snapshotDateFormatter.string(from: date)
    }

    static func privacyPath(
        _ value: String,
        privacyModeEnabled: Bool,
        revealFull: Bool = false,
        limit: Int? = nil
    ) -> String {
        let trimmed = value.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmed.isEmpty else { return value }

        if privacyModeEnabled && !revealFull {
            return collapsePath(redactLocalPath(trimmed), limit: limit ?? 84)
        }

        if revealFull {
            return trimmed
        }

        return collapsePath(trimmed, limit: limit ?? 96)
    }

    static func configPathSummary(_ value: String) -> String {
        let trimmed = value.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmed.isEmpty else { return value }

        let redacted = redactLocalPath(trimmed)
        if redacted.hasPrefix("$HOME")
            || redacted.hasPrefix("~")
            || redacted.hasPrefix("<") {
            return collapsePath(redacted, limit: 84)
        }

        let parts = redacted
            .split(separator: "/", omittingEmptySubsequences: true)
            .map(String.init)
        guard parts.count > 2 else { return redacted }
        return ".../" + parts.suffix(2).joined(separator: "/")
    }

    static func redactLocalPath(_ value: String) -> String {
        var redacted = value
        let environment = ProcessInfo.processInfo.environment
        let replacements: [(String?, String)] = [
            (environment["SKILLS_COPILOT_PROJECT_ROOT"], "<project-root>"),
            (environment["SKILLS_COPILOT_PROJECT_CWD"], "<project-cwd>"),
            (environment["SKILLS_COPILOT_APP_DATA_DIR"], "<app-data-dir>"),
            (environment["SKILLS_COPILOT_HOME"], "$HOME"),
            (FileManager.default.homeDirectoryForCurrentUser.path, "$HOME"),
        ]

        for (prefix, token) in replacements {
            guard let prefix else { continue }
            let normalized = prefix.trimmingCharacters(in: CharacterSet(charactersIn: "/"))
            guard !normalized.isEmpty else { continue }
            let absolutePrefix = "/" + normalized
            if redacted == absolutePrefix {
                redacted = token
            } else if redacted.hasPrefix(absolutePrefix + "/") {
                redacted = token + String(redacted.dropFirst(absolutePrefix.count))
            } else if redacted.contains(absolutePrefix + "/") {
                redacted = redacted.replacingOccurrences(of: absolutePrefix, with: token)
            }
        }

        let macHomePattern = "/" + "Users" + #"/[^/\s]+"#
        let varFoldersPattern = "/" + "var" + #"/folders/[^/\s]+/[^/\s]+/[^/\s]+"#
        let privateVarFoldersPattern = "/" + "private" + varFoldersPattern

        redacted = redacted.replacingOccurrences(
            of: macHomePattern,
            with: "$HOME",
            options: .regularExpression
        )
        redacted = redacted.replacingOccurrences(
            of: privateVarFoldersPattern,
            with: "<temp>",
            options: .regularExpression
        )
        redacted = redacted.replacingOccurrences(
            of: varFoldersPattern,
            with: "<temp>",
            options: .regularExpression
        )
        return redacted
    }

    static func collapsePath(_ value: String, limit: Int = 84) -> String {
        let characters = Array(value)
        guard characters.count > limit, limit >= 24 else { return value }
        let headCount = max(10, limit / 2 - 4)
        let tailCount = max(10, limit - headCount - 5)
        let head = String(characters.prefix(headCount))
        let tail = String(characters.suffix(tailCount))
        return "\(head)/.../\(tail)"
    }

    static func isLikelyPath(_ value: String) -> Bool {
        let trimmed = value.trimmingCharacters(in: .whitespacesAndNewlines)
        let macHomeMarker = "/" + "Users" + "/"
        let varFoldersMarker = "/" + "var" + "/folders/"
        let privateVarFoldersMarker = "/" + "private" + varFoldersMarker
        return trimmed.hasPrefix("/")
            || trimmed.hasPrefix("~/")
            || trimmed.hasPrefix("$HOME/")
            || trimmed.hasPrefix("<project-root>")
            || trimmed.hasPrefix("<project-cwd>")
            || trimmed.hasPrefix("<app-data-dir>")
            || trimmed.contains(macHomeMarker)
            || trimmed.contains(varFoldersMarker)
            || trimmed.contains(privateVarFoldersMarker)
            || trimmed.contains("$HOME/")
            || trimmed.contains("~/")
            || trimmed.contains("<project-root>")
            || trimmed.contains("<project-cwd>")
            || trimmed.contains("<app-data-dir>")
            || trimmed.contains("/SKILL.md")
            || trimmed.contains("\\")
    }
}
