import Compono
import ShowcaseFFI
import SwiftUI

extension ShowcaseApp: @retroactive TreeBridge {}
extension ComponoTree: @retroactive Listener {}

@main
struct ShowcaseMain: App {
    @State private var root = ComponoRoot { tree, saved in
        ShowcaseApp(listener: tree, saved: saved)
    }

    var body: some Scene {
        WindowGroup {
            RootView(component: root.app.root()).componoRoot(root)
        }
    }
}
