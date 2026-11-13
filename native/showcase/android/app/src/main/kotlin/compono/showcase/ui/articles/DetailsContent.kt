package compono.showcase.ui.articles

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Close
import androidx.compose.material.icons.filled.Person
import androidx.compose.material3.AssistChip
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import compono.android.observe
import compono.showcase.ffi.ArticleDetailsComponent

/** An article. Its author opens in the extra panel. */
@Composable
fun DetailsContent(component: ArticleDetailsComponent) {
    observe(component.node())
    val text = component.text() ?: return
    Column(
        Modifier.verticalScroll(rememberScrollState()).padding(16.dp),
        verticalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        Row(verticalAlignment = Alignment.CenterVertically) {
            Text(text.title, Modifier.weight(1f), style = MaterialTheme.typography.headlineSmall)
            IconButton(component::dismiss) { Icon(Icons.Filled.Close, contentDescription = "Close") }
        }
        AssistChip(
            onClick = component::showAuthor,
            label = { Text("Author") },
            leadingIcon = { Icon(Icons.Filled.Person, contentDescription = null) },
        )
        Text(text.body, style = MaterialTheme.typography.bodyLarge)
    }
}
