import Foundation

struct AgentIconCandidate: Equatable {
    enum Kind: Equatable {
        case appBundle
        case fileIcon
        case bundledResource
        case resource
    }

    let kind: Kind
    let path: String

    static func appBundle(_ path: String) -> AgentIconCandidate {
        AgentIconCandidate(kind: .appBundle, path: path)
    }

    static func fileIcon(_ path: String) -> AgentIconCandidate {
        AgentIconCandidate(kind: .fileIcon, path: path)
    }

    static func bundledResource(_ path: String) -> AgentIconCandidate {
        AgentIconCandidate(kind: .bundledResource, path: path)
    }

    static func resource(_ path: String) -> AgentIconCandidate {
        AgentIconCandidate(kind: .resource, path: path)
    }
}

enum AgentIconCandidates {
    static func codex(bundlePath: String?) -> [AgentIconCandidate] {
        var candidates: [AgentIconCandidate] = []
        for appPath in [bundlePath, "/Applications/ChatGPT.app", "/Applications/Codex.app"].compactMap({ $0 }) {
            let resources = "\(appPath)/Contents/Resources"
            candidates.append(.resource("\(resources)/icon-codex-dark-color.png"))
            candidates.append(.resource("\(resources)/icon-codex-light.png"))
            candidates.append(.appBundle(appPath))
            candidates.append(.resource("\(resources)/app.icns"))
            candidates.append(.resource("\(resources)/electron.icns"))
            candidates.append(.resource("\(resources)/default_app/icon.png"))
        }
        candidates.append(.fileIcon("/opt/homebrew/bin/codex"))
        var seen = Set<String>()
        return candidates.filter { seen.insert("\($0.kind):\($0.path)").inserted }
    }
}
