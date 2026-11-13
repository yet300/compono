package compono.showcase.ui.articles

import androidx.compose.foundation.layout.statusBarsPadding
import androidx.compose.material3.adaptive.ExperimentalMaterial3AdaptiveApi
import androidx.compose.material3.adaptive.currentWindowAdaptiveInfo
import androidx.compose.material3.adaptive.layout.AnimatedPane
import androidx.compose.material3.adaptive.layout.ListDetailPaneScaffold
import androidx.compose.material3.adaptive.layout.PaneAdaptedValue
import androidx.compose.material3.adaptive.layout.ThreePaneScaffoldValue
import androidx.compose.material3.adaptive.layout.calculatePaneScaffoldDirective
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalConfiguration
import compono.android.observe
import compono.showcase.ffi.ArticlesComponent
import compono.showcase.ffi.PanelChild
import compono.showcase.ffi.PanelKind
import compono.showcase.ffi.PanelsLayout
import compono.showcase.ui.rememberChild

/** Below this width, in dp, two panels fit but not three. */
private const val TRIPLE_WIDTH = 1200

/**
 * A list, its details and the author, in Material's list-detail scaffold.
 *
 * The window decides how many panels fit, and the `Panels` router in Rust decides
 * which ones show, so Back closes the deepest panel on every window size.
 */
@OptIn(ExperimentalMaterial3AdaptiveApi::class)
@Composable
fun ArticlesContent(component: ArticlesComponent, modifier: Modifier = Modifier) {
    observe(component.node())
    val directive = calculatePaneScaffoldDirective(currentWindowAdaptiveInfo())
    val wide = LocalConfiguration.current.screenWidthDp >= TRIPLE_WIDTH
    val layout = when {
        directive.maxHorizontalPartitions >= 2 && wide -> PanelsLayout.TRIPLE
        directive.maxHorizontalPartitions >= 2 -> PanelsLayout.DUAL
        else -> PanelsLayout.SINGLE
    }
    LaunchedEffect(layout) { component.setLayout(layout) }

    val panels = component.panels()
    fun shown(kind: PanelKind) = panels.firstOrNull { it.kind == kind }
    fun value(panel: PanelChild?) = if (panel != null) PaneAdaptedValue.Expanded else PaneAdaptedValue.Hidden
    val list = shown(PanelKind.MAIN)
    val details = shown(PanelKind.DETAILS)
    val author = shown(PanelKind.EXTRA)

    ListDetailPaneScaffold(
        directive = directive,
        value = ThreePaneScaffoldValue(
            primary = value(details),
            secondary = value(list),
            tertiary = value(author),
        ),
        listPane = {
            AnimatedPane {
                list?.let { panel -> rememberChild(panel.child.node) { component.list(panel) }?.let { ArticleListContent(it) } }
            }
        },
        detailPane = {
            AnimatedPane {
                details?.let { panel -> rememberChild(panel.child.node) { component.details(panel) }?.let { DetailsContent(it) } }
            }
        },
        extraPane = {
            AnimatedPane {
                author?.let { panel -> rememberChild(panel.child.node) { component.author(panel) }?.let { AuthorContent(it) } }
            }
        },
        modifier = modifier.statusBarsPadding(),
    )
}
