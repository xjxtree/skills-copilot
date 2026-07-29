import AppKit
import SwiftUI

struct WindowChromeTitlebarAccessoryLayout: Equatable {
    let contentWidth: CGFloat
    let contentHeight: CGFloat
    let trailingPadding: CGFloat

    static let windowLayoutAttribute: NSLayoutConstraint.Attribute = .right

    var accessoryWidth: CGFloat {
        contentWidth + trailingPadding
    }

    var accessoryHeight: CGFloat {
        contentHeight + 4
    }
}

struct WindowChromeTitlebarAccessory<Content: View>: NSViewRepresentable {
    let layout: WindowChromeTitlebarAccessoryLayout
    let content: Content

    init(
        layout: WindowChromeTitlebarAccessoryLayout,
        @ViewBuilder content: () -> Content
    ) {
        self.layout = layout
        self.content = content()
    }

    func makeCoordinator() -> Coordinator {
        Coordinator()
    }

    func makeNSView(context: Context) -> NSView {
        let view = NSView(frame: .zero)
        DispatchQueue.main.async {
            context.coordinator.installIfNeeded(
                in: view.window,
                layout: layout,
                content: content
            )
        }
        return view
    }

    func updateNSView(_ nsView: NSView, context: Context) {
        context.coordinator.update(layout: layout, content: content)
        DispatchQueue.main.async {
            context.coordinator.installIfNeeded(
                in: nsView.window,
                layout: layout,
                content: content
            )
        }
    }

    static func dismantleNSView(_ nsView: NSView, coordinator: Coordinator) {
        coordinator.removeAccessory()
    }

    @MainActor
    final class Coordinator {
        private weak var window: NSWindow?
        private var accessory: NSTitlebarAccessoryViewController?
        private var container: TransparentTitlebarAccessoryContainer?
        private var hostingView: FirstMouseTitlebarHostingView<Content>?
        private var containerWidthConstraint: NSLayoutConstraint?
        private var containerHeightConstraint: NSLayoutConstraint?
        private var contentWidthConstraint: NSLayoutConstraint?
        private var contentHeightConstraint: NSLayoutConstraint?

        func installIfNeeded(
            in window: NSWindow?,
            layout: WindowChromeTitlebarAccessoryLayout,
            content: Content
        ) {
            guard let window else { return }
            if self.window === window,
               let accessory,
               window.titlebarAccessoryViewControllers.contains(
                   where: { $0 === accessory }
               )
            {
                update(layout: layout, content: content)
                return
            }

            removeAccessory()

            let hostingView = FirstMouseTitlebarHostingView(rootView: content)
            hostingView.translatesAutoresizingMaskIntoConstraints = false
            hostingView.setContentHuggingPriority(.required, for: .horizontal)
            hostingView.setContentHuggingPriority(.required, for: .vertical)
            hostingView.setContentCompressionResistancePriority(.required, for: .horizontal)
            hostingView.setContentCompressionResistancePriority(.required, for: .vertical)

            let container = TransparentTitlebarAccessoryContainer(layout: layout)
            container.addSubview(hostingView)

            let containerWidthConstraint = container.widthAnchor.constraint(
                equalToConstant: layout.accessoryWidth
            )
            let containerHeightConstraint = container.heightAnchor.constraint(
                equalToConstant: layout.accessoryHeight
            )
            let contentWidthConstraint = hostingView.widthAnchor.constraint(
                equalToConstant: layout.contentWidth
            )
            let contentHeightConstraint = hostingView.heightAnchor.constraint(
                equalToConstant: layout.contentHeight
            )

            NSLayoutConstraint.activate([
                containerWidthConstraint,
                containerHeightConstraint,
                contentWidthConstraint,
                contentHeightConstraint,
                hostingView.trailingAnchor.constraint(
                    equalTo: container.trailingAnchor,
                    constant: -layout.trailingPadding
                ),
                hostingView.centerYAnchor.constraint(equalTo: container.centerYAnchor),
            ])

            let accessory = NSTitlebarAccessoryViewController()
            accessory.view = container
            accessory.layoutAttribute = WindowChromeTitlebarAccessoryLayout.windowLayoutAttribute
            window.addTitlebarAccessoryViewController(accessory)

            self.window = window
            self.accessory = accessory
            self.container = container
            self.hostingView = hostingView
            self.containerWidthConstraint = containerWidthConstraint
            self.containerHeightConstraint = containerHeightConstraint
            self.contentWidthConstraint = contentWidthConstraint
            self.contentHeightConstraint = contentHeightConstraint
        }

        func update(
            layout: WindowChromeTitlebarAccessoryLayout,
            content: Content
        ) {
            hostingView?.rootView = content
            container?.layout = layout
            containerWidthConstraint?.constant = layout.accessoryWidth
            containerHeightConstraint?.constant = layout.accessoryHeight
            contentWidthConstraint?.constant = layout.contentWidth
            contentHeightConstraint?.constant = layout.contentHeight
        }

        func removeAccessory() {
            if let accessory,
               let window,
               let index = window.titlebarAccessoryViewControllers.firstIndex(
                   where: { $0 === accessory }
               )
            {
                window.removeTitlebarAccessoryViewController(at: index)
            }
            accessory = nil
            container = nil
            hostingView = nil
            containerWidthConstraint = nil
            containerHeightConstraint = nil
            contentWidthConstraint = nil
            contentHeightConstraint = nil
            window = nil
        }
    }
}

private final class TransparentTitlebarAccessoryContainer: NSView {
    var layout: WindowChromeTitlebarAccessoryLayout {
        didSet {
            frame.size = intrinsicContentSize
            invalidateIntrinsicContentSize()
        }
    }

    init(layout: WindowChromeTitlebarAccessoryLayout) {
        self.layout = layout
        super.init(
            frame: NSRect(
                origin: .zero,
                size: NSSize(
                    width: layout.accessoryWidth,
                    height: layout.accessoryHeight
                )
            )
        )
        wantsLayer = true
        layer?.backgroundColor = NSColor.clear.cgColor
    }

    @available(*, unavailable)
    required init?(coder: NSCoder) {
        nil
    }

    override var intrinsicContentSize: NSSize {
        NSSize(width: layout.accessoryWidth, height: layout.accessoryHeight)
    }

    override func acceptsFirstMouse(for event: NSEvent?) -> Bool {
        true
    }
}

private final class FirstMouseTitlebarHostingView<Content: View>: NSHostingView<Content> {
    required init(rootView: Content) {
        super.init(rootView: rootView)
        wantsLayer = true
        layer?.backgroundColor = NSColor.clear.cgColor
    }

    @available(*, unavailable)
    required init(rootView: Content, ignoreSafeArea: Bool) {
        fatalError("init(rootView:ignoreSafeArea:) is unavailable")
    }

    @available(*, unavailable)
    required init?(coder: NSCoder) {
        nil
    }

    override func acceptsFirstMouse(for event: NSEvent?) -> Bool {
        true
    }
}
