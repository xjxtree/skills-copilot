import AppKit

enum AgentIconProvider {
    static func image(for filter: SkillAgentFilter) -> NSImage? {
        for candidate in candidates(for: filter) {
            if let image = load(candidate: candidate) {
                image.size = NSSize(width: 32, height: 32)
                return image
            }
        }
        return nil
    }

    private static func candidates(for filter: SkillAgentFilter) -> [AgentIconCandidate] {
        switch filter {
        case .claudeCode:
            return [
                .appBundle("/Applications/Claude.app"),
                .resource("/Applications/Claude.app/Contents/Resources/electron.icns"),
                .fileIcon("/opt/homebrew/bin/claude")
            ]
        case .codex:
            let bundlePath = NSWorkspace.shared
                .urlForApplication(withBundleIdentifier: "com.openai.codex")?
                .path
            return AgentIconCandidates.codex(bundlePath: bundlePath)
        case .opencode:
            return [
                .appBundle("/Applications/OpenCode.app"),
                .appBundle("/Applications/opencode.app"),
                .resource("/Applications/OpenCode.app/Contents/Resources/icon.icns"),
                .fileIcon("/opt/homebrew/bin/opencode")
            ]
        case .pi:
            return [
                .bundledResource("PiBadge.svg"),
                .appBundle("/Applications/Pi.app"),
                .appBundle("/Applications/Pi Coding Agent.app"),
                .resource("/opt/homebrew/lib/node_modules/@earendil-works/pi-coding-agent/assets/icon.png"),
                .resource("/opt/homebrew/lib/node_modules/@earendil-works/pi-coding-agent/resources/icon.png"),
                .resource("/opt/homebrew/lib/node_modules/@earendil-works/pi-coding-agent/dist/icon.png"),
                .fileIcon("/opt/homebrew/bin/pi")
            ]
        case .hermes:
            return [
                .bundledResource("HermesIcon.png")
            ]
        case .openclaw:
            return [
                .bundledResource("OpenClawIcon.svg")
            ]
        case .all:
            return []
        }
    }

    private static func load(candidate: AgentIconCandidate) -> NSImage? {
        switch candidate.kind {
        case .bundledResource:
            guard let url = Bundle.module.url(forResource: candidate.path, withExtension: nil) else {
                return nil
            }
            return NSImage(contentsOf: url)
        case .appBundle, .fileIcon:
            guard FileManager.default.fileExists(atPath: candidate.path) else {
                return nil
            }
            return NSWorkspace.shared.icon(forFile: candidate.path)
        case .resource:
            guard FileManager.default.fileExists(atPath: candidate.path) else {
                return nil
            }
            return NSImage(contentsOfFile: candidate.path)
        }
    }
}
