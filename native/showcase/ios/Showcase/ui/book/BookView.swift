import Compono
import ShowcaseFFI
import SwiftUI

/// A hundred pages, of which only the selected one and its neighbours exist.
struct BookView: View {
    let component: BookComponent
    @Environment(\.componoTree) private var tree

    var body: some View {
        let _ = tree?.observe(component.node())
        if let model = component.model() {
            NavigationStack {
                VStack(spacing: 12) {
                    ProgressView(value: Double(model.selected + 1), total: Double(model.pages))
                        .padding(.horizontal)
                    Text("\(model.alive) pages alive").font(.footnote).foregroundStyle(.secondary)
                    ChildPages(active: component.active()) { page in
                        PageView(component: component.page(child: page))
                    }
                    .frame(maxWidth: .infinity, maxHeight: .infinity)
                }
                .safeAreaInset(edge: .bottom) {
                    GlassGroup {
                        HStack {
                            Button("Previous", systemImage: "chevron.left") { component.previous() }
                                .glassButton()
                            Spacer()
                            Button("Next", systemImage: "chevron.right") { component.next() }
                                .glassButton()
                        }
                        .labelStyle(.iconOnly)
                    }
                    .padding()
                }
                .navigationTitle("Page \(model.selected + 1) of \(model.pages)")
                .navigationBarTitleDisplayMode(.inline)
                .toolbar {
                    ToolbarItem(placement: .topBarLeading) {
                        Button("Close", systemImage: "xmark") { component.dismiss() }
                    }
                    ToolbarItem(placement: .topBarTrailing) {
                        Button("Jump to 50") { component.select(index: 49) }
                    }
                }
            }
        }
    }
}
