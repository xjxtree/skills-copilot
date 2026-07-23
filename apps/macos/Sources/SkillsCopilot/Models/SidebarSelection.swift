enum SidebarSelection: Hashable {
    case session(String)
    case skill(String)
    case configOverview
    case configDocument(String)
    case configSnapshot(String)

    var appRoute: AppRoute {
        switch self {
        case .session:
            return .sessions
        case .skill:
            return .skills
        case .configOverview, .configDocument, .configSnapshot:
            return .advanced
        }
    }

    func belongs(to route: AppRoute) -> Bool {
        appRoute == route
    }

    var isSkill: Bool {
        if case .skill = self {
            return true
        }
        return false
    }

    var isSession: Bool {
        if case .session = self {
            return true
        }
        return false
    }

    var isConfig: Bool {
        switch self {
        case .configOverview, .configDocument, .configSnapshot:
            return true
        default:
            return false
        }
    }
}
