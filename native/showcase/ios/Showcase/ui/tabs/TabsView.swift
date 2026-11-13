import Compono
import ShowcaseFFI
import SwiftUI

/// The system tab bar, which is Liquid Glass on iOS 26 and later.
///
/// The component keeps every tab's state; only the selected tab has a view.
struct TabsView: View {
    let component: TabsComponent
    @Environment(\.componoTree) private var tree

    private static let items: [(TabKind, String, String)] = [
        (.menu, "Menu", "house"),
        (.counters, "Counters", "plus.circle"),
        (.cards, "Cards", "rectangle.stack"),
        (.articles, "Articles", "list.bullet"),
        (.feed, "Feed", "bell"),
    ]

    var body: some View {
        let _ = tree?.observe(component.node())
        if let active = component.active() {
            TabView(selection: Binding(get: { active.tab }, set: { component.select(tab: $0) })) {
                ForEach(Self.items, id: \.0) { tab, title, icon in
                    content(for: tab, active: active)
                        .tabItem { Label(title, systemImage: icon) }
                        .tag(tab)
                }
            }
        }
    }

    @ViewBuilder
    private func content(for tab: TabKind, active: TabsChild) -> some View {
        if tab == active.tab {
            switch tab {
            case .menu: if let child = component.menu(child: active) { MenuView(component: child) }
            case .counters: if let child = component.counters(child: active) { CountersView(component: child) }
            case .cards: if let child = component.cards(child: active) { CardsView(component: child) }
            case .articles: if let child = component.articles(child: active) { ArticlesView(component: child) }
            case .feed: if let child = component.feed(child: active) { FeedView(component: child) }
            }
        } else {
            Color.clear
        }
    }
}
