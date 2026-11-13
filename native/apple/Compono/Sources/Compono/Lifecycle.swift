import SwiftUI

#if os(iOS)
private let edgeWidth: CGFloat = 24
#endif

private struct ComponoLifecycle: ViewModifier {
    let tree: ComponoTree
    let onBackground: (Data) -> Void
    @Environment(\.scenePhase) private var scenePhase

    #if os(iOS)
    @State private var tracking = false
    /// False again after the system cancels a drag, which skips `onEnded`.
    @GestureState private var dragging = false
    @State private var width: CGFloat = 1

    /// Back from the left edge of the screen, as every iOS app has.
    private var edgeSwipe: some Gesture {
        DragGesture(minimumDistance: 10, coordinateSpace: .global)
            .updating($dragging) { _, state, _ in state = true }
            .onChanged { value in
                if !tracking {
                    guard value.startLocation.x <= edgeWidth, value.translation.width > 0, tree.canBack else {
                        return
                    }
                    tracking = tree.beginGesture()
                }
                if tracking {
                    tree.updateGesture(progress: value.translation.width / width)
                }
            }
            .onEnded { value in
                guard tracking else { return }
                tracking = false
                let flung = value.predictedEndTranslation.width / width > 0.6
                tree.finishGesture(commit: flung || value.translation.width / width > 0.4)
            }
    }

    func body(content: Content) -> some View {
        tracked(content)
            .simultaneousGesture(edgeSwipe)
            .onGeometryChange(for: CGFloat.self, of: \.size.width) { width = max($0, 1) }
            .onChange(of: dragging) { _, stillDragging in
                if !stillDragging && tracking {
                    tracking = false
                    tree.finishGesture(commit: false)
                }
            }
    }
    #else
    func body(content: Content) -> some View {
        tracked(content)
    }
    #endif

    private func tracked(_ content: Content) -> some View {
        content.onChange(of: scenePhase, initial: true) { _, phase in
            switch phase {
            case .active:
                tree.setPhase(2)
            case .inactive:
                tree.setPhase(1)
            case .background:
                tree.setPhase(0)
                onBackground(tree.save())
            @unknown default:
                tree.setPhase(1)
            }
        }
    }
}

extension View {
    /// Follows the scene's phase, hands over saved state on entering the background,
    /// and on iOS turns a swipe from the left edge into Back.
    public func componoLifecycle(
        _ tree: ComponoTree,
        onBackground: @escaping (Data) -> Void = { _ in }
    ) -> some View {
        modifier(ComponoLifecycle(tree: tree, onBackground: onBackground))
    }
}
