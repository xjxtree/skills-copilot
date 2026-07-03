import SwiftUI

struct NativePanelSurface: ViewModifier {
    func body(content: Content) -> some View {
        content
            .background {
                RoundedRectangle(cornerRadius: CGFloat(UIOptimizationPresentation.surfaceCornerRadius))
                    .fill(Color.white)
            }
            .overlay {
                RoundedRectangle(cornerRadius: CGFloat(UIOptimizationPresentation.surfaceCornerRadius))
                    .strokeBorder(.separator.opacity(0.35), lineWidth: 1)
            }
    }
}

extension View {
    func nativePanelSurface() -> some View {
        modifier(NativePanelSurface())
    }
}
