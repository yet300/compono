import Compono
import ShowcaseFFI
import SwiftUI

/// A filmstrip: a custom router written with only the public API.
struct GalleryView: View {
    let component: GalleryComponent
    @Environment(\.componoTree) private var tree

    var body: some View {
        let _ = tree?.observe(component.node())
        let contact = component.isContact()
        let frames = component.frames()
        let shown = contact ? frames : frames.filter(\.selected)
        NavigationStack {
            ScrollView {
                LazyVGrid(columns: [GridItem(.adaptive(minimum: 96), spacing: 10)], spacing: 10) {
                    ForEach(shown, id: \.key) { frame in
                        PhotoView(component: component.photo(frame: frame), selected: frame.selected)
                    }
                }
                .padding()
            }
            .navigationTitle("Filmstrip")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .topBarLeading) {
                    Button("Close", systemImage: "xmark") { component.dismiss() }
                }
                ToolbarItem(placement: .topBarTrailing) {
                    Toggle("Contact sheet", isOn: Binding(get: { contact }, set: { _ in component.toggleMode() }))
                }
                ToolbarItemGroup(placement: .bottomBar) {
                    Button("Previous", systemImage: "chevron.left") { component.previous() }
                    Button("Next", systemImage: "chevron.right") { component.next() }
                    Spacer()
                    Button("Remove", systemImage: "trash") { component.removeSelected() }
                }
            }
        }
    }
}
