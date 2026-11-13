import Compono
import ShowcaseFFI
import SwiftUI

/// One counter. It ticks only while it is the focused one.
struct CounterView: View {
    let component: CounterComponent
    @Environment(\.componoTree) private var tree

    var body: some View {
        let _ = tree?.observe(component.node())
        if let model = component.model() {
            VStack(spacing: 12) {
                Text("Counter \(model.index)").font(.headline)
                Text("\(model.value)")
                    .font(.system(size: 64, weight: .semibold, design: .rounded))
                    .contentTransition(.numericText())
                GlassGroup {
                    HStack {
                        Button("Previous", systemImage: "minus") { component.previous() }
                            .glassButton()
                        Button("Next", systemImage: "plus") { component.next() }
                            .glassButton(prominent: true)
                    }
                }
            }
            .padding(28)
            .glassSurface()
            .frame(maxWidth: .infinity, maxHeight: .infinity)
        }
    }
}
