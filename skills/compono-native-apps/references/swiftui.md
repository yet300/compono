# SwiftUI

Add the `Compono` Swift package and the package BoltFFI generates (here
`MyAppFFI`).

## Entry point

```swift
import Compono
import MyAppFFI
import SwiftUI

extension MyApp: @retroactive TreeBridge {}
extension ComponoTree: @retroactive Listener {}

@main
struct MyAppMain: App {
    @State private var root = ComponoRoot { tree, saved in
        MyApp(listener: tree, saved: saved)
    }

    var body: some Scene {
        WindowGroup {
            RootView(component: root.app.root()).componoRoot(root)
        }
    }
}
```

`ComponoRoot` creates the tree once, reads saved state from a file, and saves in
the background. `.componoRoot(root)` puts the tree in the environment and follows
the scene phase.

## Router children

Router views take any `ComponoChild` (a `key` and a `node`):

```swift
extension ChildRef: @retroactive ComponoChild {}

extension ScreenChild: @retroactive ComponoChild {
    public var key: Int64 { child.key }
    public var node: Int64 { child.node }
}
```

## A screen

```swift
struct CountersView: View {
    let component: CountersComponent
    @Environment(\.componoTree) private var tree

    var body: some View {
        let _ = tree?.observe(component.node())
        if let tree, let model = component.model(), let active = component.active() {
            NavigationStack {
                ChildStack(tree: tree, owner: component.node(), active: active, previous: component.revealed()) { child in
                    CounterView(component: component.counter(child: child))
                }
                .navigationTitle("Stack depth \(model.depth)")
            }
        }
    }
}
```

- `let _ = tree?.observe(component.node())` subscribes the view to its component.
- `ChildStack(tree:owner:active:previous:)` gives the interactive swipe-back.
  `previous` is the entry Back reveals.
- `ChildSlot(active:)` for dialogs and sheets, `ChildPages(active:)`,
  `ChildPanels(panels:)` and `ChildList(count:overscan:onWindow:)` for long lists;
  it reports the rows on screen plus 10 spare rows by default.
- Draw with system components: `TabView`, `NavigationStack`, `.alert`, `List`.

## Liquid Glass on iOS 26 with an iOS 17 floor

Put the version checks in one helper file and call the helpers from screens:

```swift
extension View {
    @ViewBuilder
    func glassButton(prominent: Bool = false) -> some View {
        if #available(iOS 26, *) {
            if prominent { buttonStyle(.glassProminent) } else { buttonStyle(.glass) }
        } else {
            if prominent { buttonStyle(.borderedProminent) } else { buttonStyle(.bordered) }
        }
    }
}
```

Use `.glassEffect` and `GlassEffectContainer` the same way, with `.regularMaterial`
as the fallback.

## Warnings

- A router view must not clip its content. `.clipped()` hides the system tab bar
  and toolbar on iOS 26.
- Read sizes with `onGeometryChange` into `@State`. A `GeometryReader` closure
  does not re-run when the component changes.
