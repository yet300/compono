import Compono
import ShowcaseFFI
import SwiftUI

/// Links to the other screens, and a dialog held in a slot.
struct MenuView: View {
    let component: MenuComponent
    @Environment(\.componoTree) private var tree

    var body: some View {
        let _ = tree?.observe(component.node())
        let dialog = component.dialog().map { component.dialogComponent(child: $0) }
        NavigationStack {
            GlassGroup {
                VStack(spacing: 14) {
                    Button("Open pages", systemImage: "book") { component.openBook() }
                        .glassButton(prominent: true)
                    Button("Open custom navigation", systemImage: "film") { component.openGallery() }
                        .glassButton()
                    Button("Show dialog", systemImage: "text.bubble") { component.showDialog() }
                        .glassButton()
                }
                .controlSize(.large)
            }
            .frame(maxWidth: .infinity, maxHeight: .infinity)
            .navigationTitle("Showcase")
            .alert(
                "Slot",
                isPresented: Binding(get: { dialog != nil }, set: { shown in if !shown { dialog?.dismiss() } }),
                presenting: dialog
            ) { dialog in
                Button("Dismiss") { dialog.dismiss() }
            } message: { dialog in
                Text(dialog.text() ?? "")
            }
        }
    }
}
