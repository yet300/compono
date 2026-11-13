package compono.showcase.ui.articles

import androidx.compose.foundation.layout.padding
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.AccountCircle
import androidx.compose.material.icons.filled.Close
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.ListItem
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import compono.android.observe
import compono.showcase.ffi.AuthorComponent

/** The author of the open article. */
@Composable
fun AuthorContent(component: AuthorComponent) {
    observe(component.node())
    ListItem(
        headlineContent = { Text(component.name().orEmpty()) },
        supportingContent = { Text("Author") },
        leadingContent = { Icon(Icons.Filled.AccountCircle, contentDescription = null) },
        trailingContent = {
            IconButton(component::dismiss) { Icon(Icons.Filled.Close, contentDescription = "Close author") }
        },
        modifier = Modifier.padding(8.dp),
    )
}
