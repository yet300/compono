package compono.showcase.ui.tabs

import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.List
import androidx.compose.material.icons.filled.AddCircle
import androidx.compose.material.icons.filled.Home
import androidx.compose.material.icons.filled.Notifications
import androidx.compose.material.icons.filled.Star
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.adaptive.navigationsuite.NavigationSuiteScaffold
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.vector.ImageVector
import compono.android.ChildStack
import compono.android.StackAnimation
import compono.android.observe
import compono.showcase.ffi.TabKind
import compono.showcase.ffi.TabsComponent
import compono.showcase.ui.articles.ArticlesContent
import compono.showcase.ui.cards.CardsContent
import compono.showcase.ui.counters.CountersContent
import compono.showcase.ui.feed.FeedContent
import compono.showcase.ui.menu.MenuContent
import compono.showcase.ui.rememberChild

private data class TabItem(val tab: TabKind, val title: String, val icon: ImageVector)

private val items = listOf(
    TabItem(TabKind.MENU, "Menu", Icons.Filled.Home),
    TabItem(TabKind.COUNTERS, "Counters", Icons.Filled.AddCircle),
    TabItem(TabKind.CARDS, "Cards", Icons.Filled.Star),
    TabItem(TabKind.ARTICLES, "Articles", Icons.AutoMirrored.Filled.List),
    TabItem(TabKind.FEED, "Feed", Icons.Filled.Notifications),
)

/**
 * The tabs, as a navigation bar on a phone and a rail or drawer on larger windows.
 * Each tab keeps its state, because the component keeps every tab's child.
 */
@Composable
fun TabsContent(component: TabsComponent, modifier: Modifier = Modifier) {
    observe(component.node())
    val active = component.active() ?: return
    NavigationSuiteScaffold(
        navigationSuiteItems = {
            for (item in items) {
                item(
                    selected = active.tab == item.tab,
                    onClick = { component.select(item.tab) },
                    icon = { Icon(item.icon, contentDescription = null) },
                    label = { Text(item.title) },
                )
            }
        },
        modifier = modifier,
    ) {
        ChildStack(
            active,
            { it.child.key },
            animation = StackAnimation.Fade,
            background = MaterialTheme.colorScheme.background,
        ) { tab ->
            val node = tab.child.node
            when (tab.tab) {
                TabKind.MENU -> rememberChild(node) { component.menu(tab) }?.let { MenuContent(it) }
                TabKind.COUNTERS -> rememberChild(node) { component.counters(tab) }?.let { CountersContent(it) }
                TabKind.CARDS -> rememberChild(node) { component.cards(tab) }?.let { CardsContent(it) }
                TabKind.ARTICLES -> rememberChild(node) { component.articles(tab) }?.let { ArticlesContent(it) }
                TabKind.FEED -> rememberChild(node) { component.feed(tab) }?.let { FeedContent(it) }
            }
        }
    }
}
