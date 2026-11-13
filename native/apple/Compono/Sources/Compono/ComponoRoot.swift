import Foundation
import SwiftUI

/// The default file for saved state, in the application's caches directory.
public let componoStateFile = URL.cachesDirectory.appending(path: "compono.state")

/// An application's tree and its bindings, created once for the process.
///
/// It reads saved state when it is created and writes it when the scene enters
/// the background, so an application needs no state handling of its own:
///
/// ```swift
/// extension ShowcaseApp: @retroactive TreeBridge {}
/// extension ComponoTree: @retroactive Listener {}
///
/// @main
/// struct ShowcaseMain: App {
///     @State private var root = ComponoRoot { tree, saved in
///         ShowcaseApp(listener: tree, saved: saved)
///     }
///
///     var body: some Scene {
///         WindowGroup { RootView().componoRoot(root) }
///     }
/// }
/// ```
@MainActor
@Observable
public final class ComponoRoot<Bridge: TreeBridge> {
    /// The tree the views observe.
    public let tree = ComponoTree()
    /// The application's bindings, for reading models and sending actions.
    public let app: Bridge
    @ObservationIgnored private let stateFile: URL?

    /// Creates the bindings with `make`, giving it the tree as their listener and
    /// the state saved in `stateFile`, or empty data. A `nil` file saves nothing.
    public init(stateFile: URL? = componoStateFile, make: (ComponoTree, Data) -> Bridge) {
        self.stateFile = stateFile
        let saved = stateFile.flatMap { try? Data(contentsOf: $0) } ?? Data()
        app = make(tree, saved)
        tree.attach(app)
    }

    /// Writes `state` to the state file, replacing it whole.
    public func persist(_ state: Data) {
        guard let stateFile else { return }
        try? state.write(to: stateFile, options: .atomic)
    }
}

extension EnvironmentValues {
    /// The tree of the nearest `.componoRoot`. A view calls `observe` on it for each
    /// component it reads.
    @Entry public var componoTree: ComponoTree? = nil
}

extension View {
    /// Puts `root` into the environment, follows the scene's phase, saves state on
    /// entering the background, and on iOS turns a swipe from the left edge into Back.
    public func componoRoot<Bridge: TreeBridge>(_ root: ComponoRoot<Bridge>) -> some View {
        environment(root)
            .environment(\.componoTree, root.tree)
            .componoLifecycle(root.tree) { root.persist($0) }
    }
}
