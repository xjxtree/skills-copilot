import AppKit
import Foundation

enum SettingsNavigation {
    static let selectionStorageKey = "settings.selectedTab"
    static let providerRequested = Notification.Name(
        "dev.agent-copilot.settings.provider-requested"
    )
    static let providerObservabilityRequested = Notification.Name(
        "dev.agent-copilot.settings.provider-observability-requested"
    )
    static let advancedRequested = Notification.Name(
        "dev.agent-copilot.settings.advanced-requested"
    )

    @MainActor
    static func openProvider() {
        open(
            tab: .provider,
            notification: providerRequested
        )
    }

    @MainActor
    static func openProviderObservability() {
        open(
            tab: .providerObservability,
            notification: providerObservabilityRequested
        )
    }

    @MainActor
    static func openAdvanced() {
        open(
            tab: .advanced,
            notification: advancedRequested
        )
    }

    @MainActor
    private static func open(tab: SettingsTab, notification: Notification.Name) {
        UserDefaults.standard.set(
            tab.rawValue,
            forKey: selectionStorageKey
        )
        NotificationCenter.default.post(
            name: notification,
            object: nil
        )
        if !NSApp.sendAction(Selector(("showSettingsWindow:")), to: nil, from: nil) {
            NSApp.activate(ignoringOtherApps: true)
        }
    }
}
