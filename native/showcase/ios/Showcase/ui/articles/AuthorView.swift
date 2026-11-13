import Compono
import ShowcaseFFI
import SwiftUI

/// The author of the open article.
struct AuthorView: View {
    let component: AuthorComponent
    @Environment(\.componoTree) private var tree

    var body: some View {
        let _ = tree?.observe(component.node())
        NavigationStack {
            VStack(spacing: 12) {
                Image(systemName: "person.crop.circle.fill").font(.system(size: 56)).foregroundStyle(.tint)
                Text(component.name() ?? "").font(.title2)
            }
            .padding(32)
            .glassSurface()
            .frame(maxWidth: .infinity, maxHeight: .infinity)
            .navigationTitle("Author")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .topBarTrailing) {
                    Button("Close author", systemImage: "xmark") { component.dismiss() }
                }
            }
        }
    }
}
