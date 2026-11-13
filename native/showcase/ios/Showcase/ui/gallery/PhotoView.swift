import Compono
import ShowcaseFFI
import SwiftUI

/// One photo, which develops while it is visible.
struct PhotoView: View {
    let component: PhotoComponent
    let selected: Bool
    @Environment(\.componoTree) private var tree

    var body: some View {
        let _ = tree?.observe(component.node())
        if let model = component.model() {
            VStack(alignment: .leading, spacing: 4) {
                Text("#\(model.number)").font(.headline)
                Text("\(model.developed)").font(.caption).foregroundStyle(.secondary)
            }
            .frame(maxWidth: .infinity, minHeight: 72, alignment: .topLeading)
            .padding(12)
            .glassSurface(cornerRadius: 16)
            .overlay {
                if selected {
                    RoundedRectangle(cornerRadius: 16).stroke(.tint, lineWidth: 2)
                }
            }
        }
    }
}
