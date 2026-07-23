import AppKit
import Foundation

enum SettingsNavigation {
    static let selectionStorageKey = "settings.selectedTab"
    static let providerObservabilityRequested = Notification.Name(
        "dev.agent-copilot.settings.provider-observability-requested"
    )

    @MainActor
    static func openProviderObservability() {
        UserDefaults.standard.set(
            SettingsTab.providerObservability.rawValue,
            forKey: selectionStorageKey
        )
        NotificationCenter.default.post(
            name: providerObservabilityRequested,
            object: nil
        )
        if !NSApp.sendAction(Selector(("showSettingsWindow:")), to: nil, from: nil) {
            NSApp.activate(ignoringOtherApps: true)
        }
    }
}
