import Compono
import ShowcaseFFI
import SwiftUI

/// Below this width, in points, two panels fit but not three.
private let tripleWidth: CGFloat = 1100

/// A list, its details and the author. The size class decides how many panels fit,
/// and the `Panels` router in Rust decides which ones show.
struct ArticlesView: View {
    let component: ArticlesComponent
    @Environment(\.componoTree) private var tree
    @Environment(\.horizontalSizeClass) private var sizeClass
    @State private var width: CGFloat = 0

    var body: some View {
        let _ = tree?.observe(component.node())
        // Read in the body itself: a `GeometryReader` closure does not run again
        // when only the component changes.
        let layout: PanelsLayout =
            sizeClass == .compact ? .single : width >= tripleWidth ? .triple : .dual
        ChildPanels(panels: component.panels()) { panel in
            switch panel.kind {
            case .main: if let child = component.list(panel: panel) { ArticleListView(component: child) }
            case .details: if let child = component.details(panel: panel) { DetailsView(component: child) }
            case .extra: if let child = component.author(panel: panel) { AuthorView(component: child) }
            }
        }
        .onGeometryChange(for: CGFloat.self, of: \.size.width) { width = $0 }
        .onChange(of: layout, initial: true) { _, layout in component.setLayout(layout: layout) }
    }
}
