import Compono
import ShowcaseFFI
import SwiftUI

/// One row. It has a component only while it is on screen.
struct RowView: View {
    let feed: FeedComponent
    let index: Int32
    @Environment(\.componoTree) private var tree

    var body: some View {
        // The feed reports when this row's component appears, and the row's own
        // component reports when its value changes.
        let _ = tree?.observe(feed.node())
        let row = feed.rowAt(index: index)
        let _ = row.map { tree?.observe($0.node) }
        HStack {
            if let row {
                VStack(alignment: .leading) {
                    Text("Row \(row.id)")
                    Text("Value \(row.value)").font(.caption).foregroundStyle(.secondary)
                }
                Spacer()
                Button(row.liked ? "Liked" : "Like", systemImage: row.liked ? "heart.fill" : "heart") {
                    feed.row(model: row).like()
                }
                .labelStyle(.iconOnly)
                .foregroundStyle(row.liked ? .pink : .secondary)
            } else {
                Text("Row \(index)").foregroundStyle(.secondary)
                Spacer()
            }
        }
        .padding(.horizontal)
        .frame(height: 52)
    }
}
