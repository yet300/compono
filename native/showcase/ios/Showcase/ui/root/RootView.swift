import Compono
import ShowcaseFFI
import SwiftUI

/// The root stack: tabs, with the book or the gallery opened above them.
struct RootView: View {
    let component: RootComponent
    @Environment(\.componoTree) private var tree

    var body: some View {
        let _ = tree?.observe(component.node())
        if let tree, let active = component.active() {
            ChildStack(tree: tree, owner: component.node(), active: active, previous: component.previous()) { screen in
                switch screen.kind {
                case .tabs: if let child = component.tabs(child: screen) { TabsView(component: child) }
                case .book: if let child = component.book(child: screen) { BookView(component: child) }
                case .gallery: if let child = component.gallery(child: screen) { GalleryView(component: child) }
                }
            }
        }
    }
}
