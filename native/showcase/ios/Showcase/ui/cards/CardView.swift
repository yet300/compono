import Compono
import ShowcaseFFI
import SwiftUI

/// One card. The focused card is tinted.
struct CardView: View {
    let component: CardComponent
    let focused: Bool
    @Environment(\.componoTree) private var tree

    var body: some View {
        let _ = tree?.observe(component.node())
        if let model = component.model() {
            HStack {
                VStack(alignment: .leading) {
                    Text("Card \(model.number)").font(.headline)
                    Text(focused ? "Focused" : "Visible").font(.subheadline).foregroundStyle(.secondary)
                }
                Spacer()
                Text("\(model.value)").font(.title2.monospacedDigit())
            }
            .padding()
            .glassSurface(cornerRadius: 18)
            .overlay {
                if focused {
                    RoundedRectangle(cornerRadius: 18).stroke(.tint, lineWidth: 2)
                }
            }
        }
    }
}
