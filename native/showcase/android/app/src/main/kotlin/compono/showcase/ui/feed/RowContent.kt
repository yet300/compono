package compono.showcase.ui.feed

import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Favorite
import androidx.compose.material.icons.filled.FavoriteBorder
import androidx.compose.material3.Icon
import androidx.compose.material3.IconToggleButton
import androidx.compose.material3.ListItem
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import compono.android.observe
import compono.android.rememberComponent
import compono.showcase.ffi.FeedComponent

/** One row. It has a component only while it is on screen. */
@Composable
fun RowContent(feed: FeedComponent, index: Int) {
    // The feed reports when this row's component appears, and the row's own
    // component reports when its value changes.
    observe(feed.node())
    val row = feed.rowAt(index)
    if (row == null) {
        ListItem(headlineContent = { Text("Row $index") })
        return
    }
    observe(row.node)
    val component = rememberComponent(row.node) { feed.row(row) }
    ListItem(
        headlineContent = { Text("Row ${row.id}") },
        supportingContent = { Text("Value ${row.value}") },
        trailingContent = {
            IconToggleButton(row.liked, { component.like() }) {
                Icon(
                    if (row.liked) Icons.Filled.Favorite else Icons.Filled.FavoriteBorder,
                    contentDescription = if (row.liked) "Liked" else "Like",
                )
            }
        },
    )
}
