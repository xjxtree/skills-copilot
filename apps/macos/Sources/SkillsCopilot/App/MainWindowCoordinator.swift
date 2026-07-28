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

    @discardableResult
    static func restoreMainWindow(
        in app: NSApplication = .shared,
        createIfMissing: Bool = false
    ) -> Bool {
        activateApplication(app)

        guard let window = preferredMainWindow(in: app.windows) else {
            return createIfMissing && requestNewMainWindow(in: app)
        }

        configureWindow(window)
        if window.isMiniaturized {
            window.deminiaturize(nil)
        }
        window.makeKeyAndOrderFront(nil)
        return true
    }

    @discardableResult
    static func requestNewMainWindow(in app: NSApplication = .shared) -> Bool {
        guard let item = newWindowMenuItem(in: app.mainMenu),
              let action = item.action else {
            return false
        }

        return app.sendAction(action, to: item.target, from: item)
    }

    static func newWindowMenuItem(in mainMenu: NSMenu?) -> NSMenuItem? {
        let commandModifiers: NSEvent.ModifierFlags = [.command, .shift, .option, .control]

        return mainMenu?.items
            .compactMap(\.submenu)
            .flatMap(\.items)
            .first { item in
                item.isEnabled
                    && item.action != nil
                    && item.keyEquivalent.caseInsensitiveCompare("n") == .orderedSame
                    && item.keyEquivalentModifierMask.intersection(commandModifiers) == .command
            }
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

    static func preferredMainWindow(in windows: [NSWindow]) -> NSWindow? {
        windows
            .filter { $0.canBecomeMain && isMainWindowCandidate($0) }
            .max {
                mainWindowScore(identifier: $0.identifier, title: $0.title, canBecomeMain: $0.canBecomeMain)
                    < mainWindowScore(identifier: $1.identifier, title: $1.title, canBecomeMain: $1.canBecomeMain)
            }
    }
}
