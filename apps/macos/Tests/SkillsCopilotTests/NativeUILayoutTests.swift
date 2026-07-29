import AppKit
import SwiftUI
import Testing
@testable import SkillsCopilot

@Suite("NativeUILayoutTests", .serialized)
@MainActor
struct NativeUILayoutTests {
    @Test("no-window fallback uses the standard WindowGroup command")
    func newWindowFallbackUsesTheStandardCommand() throws {
        let mainMenu = NSMenu()
        let unrelatedMenu = NSMenu()
        unrelatedMenu.addItem(
            withTitle: "Unrelated",
            action: #selector(NSDocumentController.newDocument(_:)),
            keyEquivalent: "n"
        )
        unrelatedMenu.items[0].keyEquivalentModifierMask = [.command, .shift]
        let unrelatedRoot = NSMenuItem()
        unrelatedRoot.submenu = unrelatedMenu
        mainMenu.addItem(unrelatedRoot)

        let fileMenu = NSMenu()
        let newWindow = fileMenu.addItem(
            withTitle: "New Window",
            action: #selector(NSDocumentController.newDocument(_:)),
            keyEquivalent: "n"
        )
        newWindow.keyEquivalentModifierMask = [.command]
        let fileRoot = NSMenuItem()
        fileRoot.submenu = fileMenu
        mainMenu.addItem(fileRoot)

        try expectEqual(
            MainWindowCoordinator.newWindowMenuItem(in: mainMenu) === newWindow,
            true,
            "The no-window fallback should dispatch the standard Command-N WindowGroup action."
        )
    }

    @Test("reopen ignores unrelated app windows")
    func reopenIgnoresUnrelatedAppWindows() throws {
        let unrelatedWindow = MainCapableTestWindow(
            contentRect: NSRect(x: 0, y: 0, width: 400, height: 300),
            styleMask: [.titled],
            backing: .buffered,
            defer: false
        )
        unrelatedWindow.title = "Settings"
        let mainWindow = MainCapableTestWindow(
            contentRect: NSRect(x: 0, y: 0, width: 1024, height: 700),
            styleMask: [.titled],
            backing: .buffered,
            defer: false
        )
        mainWindow.title = UIStrings.appWindowTitle

        try expectNil(
            MainWindowCoordinator.preferredMainWindow(in: [unrelatedWindow]),
            "An unrelated settings window must not be restyled as the main window."
        )
        try expectEqual(
            MainWindowCoordinator.preferredMainWindow(in: [unrelatedWindow, mainWindow]) === mainWindow,
            true,
            "Reopen should select the titled main window while ignoring unrelated windows."
        )
    }

    @Test("main content lays out and renders at the supported window width")
    func mainContentRendersAtSupportedWindowWidth() async throws {
        let defaults = UserDefaults.standard
        let onboardingKey = FirstRunOnboardingModel.completionStorageKey
        let priorOnboardingValue = defaults.object(forKey: onboardingKey)
        defaults.set(true, forKey: onboardingKey)
        defer {
            if let priorOnboardingValue {
                defaults.set(priorOnboardingValue, forKey: onboardingKey)
            } else {
                defaults.removeObject(forKey: onboardingKey)
            }
        }

        let fake = try FakeServiceScript()
        defer { fake.cleanup() }
        fake.activate(scenario: "normal")

        let store = SkillStore(service: fake.serviceClient())
        await store.loadAppStartupDataIfNeeded()

        let minimumSize = NSSize(
            width: MainWindowModel.minimumWidth,
            height: max(MainWindowModel.minimumHeight, 700)
        )
        let window = NSWindow(
            contentRect: NSRect(origin: .zero, size: minimumSize),
            styleMask: [.titled, .closable, .resizable],
            backing: .buffered,
            defer: false
        )
        window.minSize = NSSize(
            width: MainWindowModel.minimumWidth,
            height: MainWindowModel.minimumHeight
        )

        let rootView = ContentView()
            .environmentObject(store)
            .environmentObject(store.sessionStore)
            .environmentObject(store.providerStore)
            .environmentObject(store.skillManagerStore)
            .environment(\.locale, Locale(identifier: AppLanguage.english.localeIdentifier))
            .frame(
                minWidth: CGFloat(MainWindowModel.minimumWidth),
                minHeight: CGFloat(MainWindowModel.minimumHeight)
            )
        let hostingView = NSHostingView(rootView: rootView)
        window.contentView = hostingView

        try assertRenderedLayout(
            window: window,
            hostingView: hostingView,
            contentSize: NSSize(width: minimumSize.width + 220, height: minimumSize.height)
        )
        try assertRenderedLayout(
            window: window,
            hostingView: hostingView,
            contentSize: minimumSize
        )
    }

    private func assertRenderedLayout<Content: View>(
        window: NSWindow,
        hostingView: NSHostingView<Content>,
        contentSize: NSSize
    ) throws {
        window.setContentSize(contentSize)
        hostingView.needsLayout = true
        hostingView.layoutSubtreeIfNeeded()
        hostingView.displayIfNeeded()

        let bounds = hostingView.bounds
        let contentLayoutSize = window.contentLayoutRect.size
        try expectEqual(
            Int(bounds.width.rounded()),
            Int(contentLayoutSize.width.rounded()),
            "Hosting view should follow the window content-layout width."
        )
        try expectEqual(
            Int(bounds.height.rounded()),
            Int(contentLayoutSize.height.rounded()),
            "Hosting view should follow the window content-layout height."
        )

        let fittingSize = hostingView.fittingSize
        if fittingSize.width > bounds.width + 1 || fittingSize.height > bounds.height + 1 {
            throw NativeModelTestFailure(
                description: "Rendered content requires \(fittingSize) inside \(bounds.size)."
            )
        }

        guard let bitmap = hostingView.bitmapImageRepForCachingDisplay(in: bounds) else {
            throw NativeModelTestFailure(description: "SwiftUI content did not produce a renderable bitmap.")
        }
        hostingView.cacheDisplay(in: bounds, to: bitmap)
        guard let bytes = bitmap.bitmapData else {
            throw NativeModelTestFailure(description: "Rendered bitmap has no pixel storage.")
        }

        let byteCount = bitmap.bytesPerRow * bitmap.pixelsHigh
        let stride = max(bitmap.samplesPerPixel, 1) * 64
        var hasRenderedPixel = false
        var index = 0
        while index < byteCount {
            if bytes[index] != 0 {
                hasRenderedPixel = true
                break
            }
            index += stride
        }
        try expectEqual(
            hasRenderedPixel,
            true,
            "SwiftUI content should render visible pixels at the tested window size."
        )
    }
}

private final class MainCapableTestWindow: NSWindow {
    override var canBecomeMain: Bool { true }
}
