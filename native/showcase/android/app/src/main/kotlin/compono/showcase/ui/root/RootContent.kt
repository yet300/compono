package compono.showcase.ui.root

import androidx.compose.material3.MaterialTheme
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import compono.android.ChildStack
import compono.android.LocalComponoTree
import compono.android.observe
import compono.showcase.ffi.RootComponent
import compono.showcase.ffi.ScreenKind
import compono.showcase.ui.book.BookContent
import compono.showcase.ui.gallery.GalleryContent
import compono.showcase.ui.tabs.TabsContent
import compono.showcase.ui.rememberChild

/** The root stack: tabs, with the book or the gallery opened above them. */
@Composable
fun RootContent(component: RootComponent, modifier: Modifier = Modifier) {
    observe(component.node())
    val active = component.active() ?: return
    ChildStack(
        active,
        { it.child.key },
        modifier,
        tree = LocalComponoTree.current,
        owner = component.node(),
        previous = component.previous(),
        node = { it.child.node },
        background = MaterialTheme.colorScheme.background,
    ) { screen ->
        when (screen.kind) {
            ScreenKind.TABS -> rememberChild(screen.child.node) { component.tabs(screen) }?.let { TabsContent(it) }
            ScreenKind.BOOK -> rememberChild(screen.child.node) { component.book(screen) }?.let { BookContent(it) }
            ScreenKind.GALLERY -> rememberChild(screen.child.node) { component.gallery(screen) }?.let { GalleryContent(it) }
        }
    }
}
