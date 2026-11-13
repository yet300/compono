import Compono
import ShowcaseFFI
import SwiftUI

/// 100 000 rows. Only the rows on screen have components.
struct FeedView: View {
    let component: FeedComponent
    @Environment(\.componoTree) private var tree

    var body: some View {
        let _ = tree?.observe(component.node())
        if let model = component.model() {
            NavigationStack {
                ChildList(
                    count: Int(model.rows),
                    onWindow: { range in
                        component.setWindow(first: Int32(range.lowerBound), last: Int32(range.upperBound))
                    }
                ) { index in
                    RowView(feed: component, index: Int32(index))
                }
                .navigationTitle("\(model.rows) rows, \(model.liveRows) live")
                .navigationBarTitleDisplayMode(.inline)
                .toolbar {
                    ToolbarItem(placement: .topBarTrailing) {
                        Toggle("Live", isOn: Binding(get: { model.live }, set: { _ in component.toggleLive() }))
                    }
                }
            }
        }
    }
}
