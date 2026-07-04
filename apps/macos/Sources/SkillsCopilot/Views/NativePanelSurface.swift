import SwiftUI

struct NativePanelSurface: ViewModifier {
    func body(content: Content) -> some View {
        content
            .background {
                RoundedRectangle(cornerRadius: CGFloat(UIOptimizationPresentation.surfaceCornerRadius))
                    .fill(Color.agentCopilotPanelBackground)
            }
            .overlay {
                RoundedRectangle(cornerRadius: CGFloat(UIOptimizationPresentation.surfaceCornerRadius))
                    .strokeBorder(.separator.opacity(0.35), lineWidth: 1)
            }
    }
}

extension Color {
    static var agentCopilotPanelBackground: Color {
        Color(nsColor: .controlBackgroundColor)
    }

    static var agentCopilotWindowBackground: Color {
        Color(nsColor: .windowBackgroundColor)
    }
}

extension View {
    func nativePanelSurface() -> some View {
        modifier(NativePanelSurface())
    }
}
