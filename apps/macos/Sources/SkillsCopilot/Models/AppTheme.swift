import Foundation

enum AppTheme: String, CaseIterable, Identifiable {
    case system
    case light
    case dark

    static let storageKey = "app.theme"
    static let defaultTheme = AppTheme.system

    var id: String { rawValue }

    var title: String {
        switch self {
        case .system:
            return UIStrings.themeFollowSystem
        case .light:
            return UIStrings.themeLight
        case .dark:
            return UIStrings.themeDark
        }
    }

    static var current: AppTheme {
        fromStorage(UserDefaults.standard.string(forKey: storageKey))
    }

    static func fromStorage(_ rawValue: String?) -> AppTheme {
        guard let rawValue, let theme = AppTheme(rawValue: rawValue) else {
            return defaultTheme
        }
        return theme
    }
}
