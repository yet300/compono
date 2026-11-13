import Compono
import ShowcaseFFI
import SwiftUI

/// A thousand articles. Selecting one opens its details.
struct ArticleListView: View {
    let component: ArticleListComponent
    @Environment(\.componoTree) private var tree

    var body: some View {
        let _ = tree?.observe(component.node())
        let selected = component.selected()
        NavigationStack {
            List(0..<component.count(), id: \.self) { id in
                Button(component.title(id: id)) { component.select(id: id) }
                    .foregroundStyle(.primary)
                    .listRowBackground(id == selected ? Color.accentColor.opacity(0.18) : nil)
            }
            .navigationTitle("Articles")
        }
    }
}
