import Compono
import ShowcaseFFI
import SwiftUI

/// One page, which keeps its tap count while it stays alive.
struct PageView: View {
    let component: PageComponent
    @Environment(\.componoTree) private var tree

    var body: some View {
        let _ = tree?.observe(component.node())
        if let model = component.model() {
            VStack(spacing: 14) {
                Text("Page \(model.number)").font(.largeTitle.weight(.semibold))
                Text("Tapped \(model.taps) times").foregroundStyle(.secondary)
                Button("Tap") { component.tap() }
                    .glassButton(prominent: true)
            }
            .padding(36)
            .glassSurface()
            .frame(maxWidth: .infinity, maxHeight: .infinity)
        }
    }
}
