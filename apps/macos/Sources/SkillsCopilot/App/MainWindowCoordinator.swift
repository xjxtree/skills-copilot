import AppKit

enum MainWindowCoordinator {
    static let windowIdentifier = NSUserInterfaceItemIdentifier(MainWindowModel.windowIdentifierRawValue)
    static let autosaveName = MainWindowModel.autosaveName
    static let minimumSize = NSSize(width: MainWindowModel.minimumWidth, height: MainWindowModel.minimumHeight)

    static func configureApplicationAppearance(_ theme: AppTheme = .current, app: NSApplication = .shared) {
        app.appearance = theme.nsAppearance
    }

    static func applyAppearance(_ theme: AppTheme = .current, app: NSApplication = .shared) {
        configureApplicationAppearance(theme, app: app)
        app.windows.forEach { $0.appearance = theme.nsAppearance }
        configureWindows(app.windows, theme: theme)
    }

    static func activateApplication(_ app: NSApplication = .shared) {
        app.setActivationPolicy(.regular)
        app.activate(ignoringOtherApps: true)
    }

    static func configureWindows(_ windows: [NSWindow], theme: AppTheme = .current) {
        windows.filter(isMainWindowCandidate).forEach { configureWindow($0, theme: theme) }
    }

    static func configureWindow(_ window: NSWindow, theme: AppTheme = .current) {
        window.identifier = windowIdentifier
        _ = window.setFrameAutosaveName(autosaveName)
        window.minSize = minimumSize
        window.appearance = theme.nsAppearance
        if window.title.isEmpty {
            window.title = UIStrings.appWindowTitle
        }
        window.titleVisibility = .hidden
        window.titlebarAppearsTransparent = true
        window.styleMask.insert(.fullSizeContentView)
        window.isMovableByWindowBackground = false
    }

    private static func isMainWindowCandidate(_ window: NSWindow) -> Bool {
        window.identifier == windowIdentifier || window.title == UIStrings.appWindowTitle
    }
}
