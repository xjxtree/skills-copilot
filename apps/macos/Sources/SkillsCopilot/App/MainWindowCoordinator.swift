import AppKit

enum MainWindowCoordinator {
    static let windowIdentifier = NSUserInterfaceItemIdentifier(MainWindowModel.windowIdentifierRawValue)
    static let autosaveName = MainWindowModel.autosaveName
    static let minimumSize = NSSize(width: MainWindowModel.minimumWidth, height: MainWindowModel.minimumHeight)
    static let appAppearance = NSAppearance(named: .aqua)

    static func configureApplicationAppearance(_ app: NSApplication = .shared) {
        app.appearance = appAppearance
    }

    static func activateApplication(_ app: NSApplication = .shared) {
        app.setActivationPolicy(.regular)
        app.activate(ignoringOtherApps: true)
    }

    @discardableResult
    static func restoreMainWindow(in app: NSApplication = .shared) -> Bool {
        activateApplication(app)

        guard let window = preferredMainWindow(in: app.windows) else {
            return false
        }

        configureWindow(window)
        if window.isMiniaturized {
            window.deminiaturize(nil)
        }
        window.makeKeyAndOrderFront(nil)
        return true
    }

    static func configureWindows(_ windows: [NSWindow]) {
        windows.filter(isMainWindowCandidate).forEach(configureWindow)
    }

    static func configureWindow(_ window: NSWindow) {
        window.identifier = windowIdentifier
        _ = window.setFrameAutosaveName(autosaveName)
        window.minSize = minimumSize
        window.appearance = appAppearance
        if window.title.isEmpty {
            window.title = UIStrings.appWindowTitle
        }
        window.titleVisibility = .hidden
        window.titlebarAppearsTransparent = true
        window.styleMask.insert(.fullSizeContentView)
        window.isMovableByWindowBackground = false
    }

    static func mainWindowScore(identifier: NSUserInterfaceItemIdentifier?, title: String, canBecomeMain: Bool) -> Int {
        MainWindowModel.mainWindowScore(
            identifierRawValue: identifier?.rawValue,
            title: title,
            canBecomeMain: canBecomeMain
        )
    }

    private static func isMainWindowCandidate(_ window: NSWindow) -> Bool {
        window.identifier == windowIdentifier || window.title == UIStrings.appWindowTitle
    }

    private static func preferredMainWindow(in windows: [NSWindow]) -> NSWindow? {
        windows
            .filter(\.canBecomeMain)
            .max {
                mainWindowScore(identifier: $0.identifier, title: $0.title, canBecomeMain: $0.canBecomeMain)
                    < mainWindowScore(identifier: $1.identifier, title: $1.title, canBecomeMain: $1.canBecomeMain)
            }
    }
}
