import AppKit
import SwiftUI

struct MainWindowConfigurator: NSViewRepresentable {
    let theme: AppTheme

    func makeNSView(context: Context) -> NSView {
        let view = NSView(frame: .zero)
        view.isHidden = true
        configure(windowFor: view)
        return view
    }

    func updateNSView(_ nsView: NSView, context: Context) {
        configure(windowFor: nsView)
    }

    private func configure(windowFor view: NSView) {
        DispatchQueue.main.async {
            if let window = view.window {
                MainWindowCoordinator.configureWindow(window, theme: theme)
            }
        }
    }
}
