package compono.showcase.ui.articles

import androidx.compose.foundation.clickable
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.ListItem
import androidx.compose.material3.ListItemDefaults
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import compono.android.observe
import compono.showcase.ffi.ArticleListComponent

/** A thousand articles. Selecting one opens its details. */
@Composable
fun ArticleListContent(component: ArticleListComponent) {
    observe(component.node())
    val selected = component.selected()
    val count = component.count()
    val colors = MaterialTheme.colorScheme
    LazyColumn {
        items(count) { id ->
            ListItem(
                headlineContent = { Text(component.title(id)) },
                modifier = Modifier.clickable { component.select(id) },
                colors = ListItemDefaults.colors(
                    containerColor = if (id == selected) colors.secondaryContainer else colors.surface,
                ),
            )
            HorizontalDivider()
        }
    }
}
