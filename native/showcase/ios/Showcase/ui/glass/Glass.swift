import SwiftUI

/// Liquid Glass on iOS 26 and later, and the system materials before it.
extension View {
    /// A rounded glass surface behind the content.
    @ViewBuilder
    func glassSurface(cornerRadius: CGFloat = 24) -> some View {
        if #available(iOS 26, *) {
            glassEffect(.regular, in: .rect(cornerRadius: cornerRadius))
        } else {
            background(.regularMaterial, in: .rect(cornerRadius: cornerRadius))
        }
    }

    /// A glass button, or the bordered style before iOS 26.
    @ViewBuilder
    func glassButton(prominent: Bool = false) -> some View {
        if #available(iOS 26, *) {
            if prominent {
                buttonStyle(.glassProminent)
            } else {
                buttonStyle(.glass)
            }
        } else if prominent {
            buttonStyle(.borderedProminent)
        } else {
            buttonStyle(.bordered)
        }
    }
}

/// Lets neighbouring glass shapes blend into each other on iOS 26 and later.
struct GlassGroup<Content: View>: View {
    var spacing: CGFloat = 12
    @ViewBuilder var content: Content

    var body: some View {
        if #available(iOS 26, *) {
            GlassEffectContainer(spacing: spacing) { content }
        } else {
            content
        }
    }
}
