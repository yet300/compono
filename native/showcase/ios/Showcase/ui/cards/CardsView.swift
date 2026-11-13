import Compono
import ShowcaseFFI
import SwiftUI

/// A pile of cards kept in any order. Only the top card ticks.
struct CardsView: View {
    let component: CardsComponent
    @Environment(\.componoTree) private var tree

    var body: some View {
        let _ = tree?.observe(component.node())
        let cards = component.cards()
        NavigationStack {
            ScrollView {
                LazyVStack(spacing: 10) {
                    ForEach(cards, id: \.key) { child in
                        CardView(component: component.card(child: child), focused: child.phase == 2)
                    }
                }
                .padding()
            }
            .navigationTitle("Cards")
            .toolbar {
                ToolbarItemGroup(placement: .topBarTrailing) {
                    Button("Send top to back", systemImage: "arrow.down.to.line") { component.sendTopToBack() }
                    Button("Remove", systemImage: "trash") { component.remove() }
                    Button("Add", systemImage: "plus") { component.add() }
                }
            }
        }
    }
}
