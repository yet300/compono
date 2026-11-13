# Jetpack Compose

Add the Compose library (`native/android/compono`, package `compono.android`) and
the Kotlin package BoltFFI generates (here `com.example.ffi`).

## Entry point

```kotlin
class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        enableEdgeToEdge()
        super.onCreate(savedInstanceState)
        val root = retainedTree(
            create = { listener, saved -> MyApp(object : Listener, ComponoListener by listener {}, saved) },
            bridge = { app ->
                treeBridge(
                    app::pump, app::setPhase, app::back, app::backStart,
                    app::backCancel, app::backCommit, app::contains, app::save,
                )
            },
        )
        setContent {
            MaterialTheme {
                ComponoHost(root.tree) {
                    RootContent(rememberComponent(root.app) { root.app.root() })
                }
            }
        }
    }
}
```

- `retainedTree` keeps the tree across configuration changes, saves it in the
  Activity's saved state, and closes it when the Activity finishes. Call it in
  `onCreate`. A tree kept in an `Application` survives the Activity and shows a
  stale screen on relaunch.
- `ComponoHost` provides the tree as `LocalComponoTree`, follows the lifecycle,
  and connects system Back and predictive Back.

## A screen

```kotlin
@Composable
fun CountersContent(component: CountersComponent, modifier: Modifier = Modifier) {
    observe(component.node())
    val model = component.model() ?: return
    val active = component.active() ?: return
    Scaffold(modifier, topBar = { TopAppBar(title = { Text("Stack depth ${model.depth}") }) }) { padding ->
        ChildStack(
            active,
            ChildRef::key,
            Modifier.padding(padding),
            tree = LocalComponoTree.current,
            owner = component.node(),
            previous = component.revealed(),
        ) { child ->
            CounterContent(rememberComponent(child.node) { component.counter(child) })
        }
    }
}
```

- `observe(component.node())` first, then read `model()`.
- `rememberComponent(key) { ... }` creates a component object once per key and
  closes it when it leaves the composition.
- `ChildStack(active, key, tree = ..., owner = ..., previous = ...)` animates and
  follows predictive Back. `ChildSlot`, `ChildPages`, `ChildPanels` and
  `ChildList(count, onWindow) { row -> }` cover the other routers. `ChildList`
  reports the rows on screen plus `overscan` spare rows (10 by default).
- Draw with Material 3: `NavigationSuiteScaffold` for tabs that become a rail on
  wide screens, `ListDetailPaneScaffold` from material3-adaptive for `Panels`,
  `AlertDialog` for a `Slot`.

## Warnings

- Read the model outside a `LazyColumn` block, or the list shows stale rows.
- AGP 9 reads Kotlin sources from `kotlin.directories.add(...)`, not
  `java.srcDir`.
- material3-adaptive 1.3 needs `compileSdk` 37.
