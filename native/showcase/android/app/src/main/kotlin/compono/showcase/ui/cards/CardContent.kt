package compono.showcase.ui.cards

import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.ListItem
import androidx.compose.material3.ListItemDefaults
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import compono.android.observe
import compono.showcase.ffi.CardComponent

/** One card. The focused card uses the primary container colour. */
@Composable
fun CardContent(component: CardComponent, focused: Boolean) {
    observe(component.node())
    val model = component.model() ?: return
    val colors = MaterialTheme.colorScheme
    Card(
        Modifier.fillMaxWidth(),
        colors = CardDefaults.cardColors(
            containerColor = if (focused) colors.primaryContainer else colors.surfaceContainer,
        ),
    ) {
        ListItem(
            headlineContent = { Text("Card ${model.number}") },
            supportingContent = { Text(if (focused) "Focused" else "Visible") },
            trailingContent = { Text("${model.value}", style = MaterialTheme.typography.titleLarge) },
            colors = ListItemDefaults.colors(containerColor = androidx.compose.ui.graphics.Color.Transparent),
        )
    }
}
