import Compono
import ShowcaseFFI
import SwiftUI

/// An article. Its author opens in the extra panel.
struct DetailsView: View {
    let component: ArticleDetailsComponent
    @Environment(\.componoTree) private var tree

    var body: some View {
        let _ = tree?.observe(component.node())
        if let text = component.text() {
            NavigationStack {
                ScrollView {
                    VStack(alignment: .leading, spacing: 16) {
                        Button("Author", systemImage: "person.crop.circle") { component.showAuthor() }
                            .glassButton()
                        Text(text.body).font(.body)
                    }
                    .padding()
                }
                .navigationTitle(text.title)
                .navigationBarTitleDisplayMode(.inline)
                .toolbar {
                    ToolbarItem(placement: .topBarTrailing) {
                        Button("Close", systemImage: "xmark") { component.dismiss() }
                    }
                }
            }
        }
    }
}
