import Compono
import ShowcaseFFI
import SwiftUI

/// A stack of counters that can be a thousand deep. Back pops one.
struct CountersView: View {
    let component: CountersComponent
    @Environment(\.componoTree) private var tree

    var body: some View {
        let _ = tree?.observe(component.node())
        if let tree, let model = component.model(), let active = component.active() {
            NavigationStack {
                ChildStack(tree: tree, owner: component.node(), active: active, previous: component.revealed()) { child in
                    CounterView(component: component.counter(child: child))
                }
                .safeAreaInset(edge: .bottom) {
                    GlassGroup {
                        HStack(spacing: 12) {
                            Button("Push \(model.deepPush) screens") { component.goDeep() }
                                .glassButton()
                            Button("Back to first") { component.popToRoot() }
                                .glassButton()
                        }
                    }
                    .padding()
                }
                .navigationTitle("Stack depth \(model.depth)")
                .navigationBarTitleDisplayMode(.inline)
                .toolbar {
                    if model.depth > 1 {
                        ToolbarItem(placement: .topBarLeading) {
                            Button("Back", systemImage: "chevron.backward") { component.back() }
                        }
                    }
                }
            }
        }
    }
}
