import AppKit
import SwiftUI

@MainActor
final class AppDelegate: NSObject, NSApplicationDelegate {
    private weak var autosaveStore: SkillStore?
    private var terminationFlushTask: Task<Void, Never>?
    private var hasRepliedToTerminationRequest = false

    func configureAutosaveFlusher(store: SkillStore) {
        autosaveStore = store
    }

    func applicationDidFinishLaunching(_ notification: Notification) {
        MainWindowCoordinator.configureApplicationAppearance()
        MainWindowCoordinator.activateApplication()
        DispatchQueue.main.async {
            MainWindowCoordinator.restoreMainWindow()
        }
    }

    func applicationDidBecomeActive(_ notification: Notification) {
        MainWindowCoordinator.configureApplicationAppearance()
        MainWindowCoordinator.configureWindows(NSApp.windows)
    }

    func applicationShouldHandleReopen(_ sender: NSApplication, hasVisibleWindows flag: Bool) -> Bool {
        DispatchQueue.main.async {
            MainWindowCoordinator.restoreMainWindow(in: sender)
        }
        return true
    }

    func applicationShouldTerminate(_ sender: NSApplication) -> NSApplication.TerminateReply {
        guard let autosaveStore else { return .terminateNow }
        guard !hasRepliedToTerminationRequest else { return .terminateNow }
        guard terminationFlushTask == nil else { return .terminateLater }

        terminationFlushTask = Task { @MainActor [weak self, sender, autosaveStore] in
            await autosaveStore.flushPendingAutosaves()
            guard let self, !self.hasRepliedToTerminationRequest else { return }
            self.hasRepliedToTerminationRequest = true
            sender.reply(toApplicationShouldTerminate: true)
        }
        return .terminateLater
    }
}

@main
struct SkillsCopilotApp: App {
    @NSApplicationDelegateAdaptor(AppDelegate.self) private var appDelegate
    @StateObject private var store = SkillStore(service: ServiceClient())
    @AppStorage(AppLanguage.storageKey) private var appLanguageRawValue = AppLanguage.defaultLanguage.rawValue
    @AppStorage(AppTheme.storageKey) private var appThemeRawValue = AppTheme.defaultTheme.rawValue

    var body: some Scene {
        let appLanguage = UIStrings.use(AppLanguage.fromStorage(appLanguageRawValue))
        let appTheme = AppTheme.fromStorage(appThemeRawValue)

        WindowGroup(UIStrings.appWindowTitle) {
            ContentView()
                .environmentObject(store)
                .environment(\.locale, Locale(identifier: appLanguage.localeIdentifier))
                .preferredColorScheme(appTheme.colorScheme)
                .id(appLanguage.rawValue)
                .frame(minWidth: CGFloat(MainWindowModel.minimumWidth), minHeight: CGFloat(MainWindowModel.minimumHeight))
                .background(MainWindowConfigurator(theme: appTheme))
                .onAppear {
                    appDelegate.configureAutosaveFlusher(store: store)
                    MainWindowCoordinator.applyAppearance(appTheme)
                }
                .onChange(of: appThemeRawValue) { newValue in
                    MainWindowCoordinator.applyAppearance(AppTheme.fromStorage(newValue))
                }
        }
        .commands {
            CommandGroup(after: .newItem) {
                Button(UIStrings.menuScanSkills) {
                    Task { await store.scanAll() }
                }
                .keyboardShortcut("r", modifiers: [.command, .shift])
                .disabled(store.isRefreshBusy)

                Button(UIStrings.menuReloadSkills) {
                    Task { await store.reload() }
                }
                .keyboardShortcut("r", modifiers: [.command])
                .disabled(store.isRefreshBusy)
            }

            CommandMenu(UIStrings.menuSkills) {
                Button(UIStrings.text("menu.showSessions", "Show Sessions")) {
                    store.sidebarContentMode = .sessions
                    if let session = store.selectedLocalSession ?? store.localSessionPreviewResult.sessionRows.first {
                        store.selectLocalSession(session)
                    } else {
                        store.selectedSidebarSelection = nil
                    }
                }
                .keyboardShortcut("1", modifiers: [.command])

                Button(UIStrings.menuShowOverview) {
                    store.selectedSidebarSelection = store.selectedSkillID.map(SidebarSelection.skill)
                    store.selectedDetailSection = .overview
                }
                .keyboardShortcut("2", modifiers: [.command])

                Button(UIStrings.menuShowFindings) {
                    store.selectedSidebarSelection = store.selectedSkillID.map(SidebarSelection.skill)
                    store.selectedDetailSection = .findings
                }
                .keyboardShortcut("3", modifiers: [.command])

                Divider()

                Button(UIStrings.menuClearSearch) {
                    store.searchText = ""
                }
                .keyboardShortcut(.delete, modifiers: [.command, .shift])
            }
        }

        Settings {
            SettingsView()
                .environmentObject(store)
                .environment(\.locale, Locale(identifier: appLanguage.localeIdentifier))
                .preferredColorScheme(appTheme.colorScheme)
                .id(appLanguage.rawValue)
                .onAppear {
                    MainWindowCoordinator.applyAppearance(appTheme)
                }
                .onChange(of: appThemeRawValue) { newValue in
                    MainWindowCoordinator.applyAppearance(AppTheme.fromStorage(newValue))
                }
        }
    }
}
