package compono.showcase.ui.gallery

import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import compono.android.observe
import compono.showcase.ffi.PhotoComponent

/** One photo, which develops while it is visible. */
@Composable
fun PhotoContent(component: PhotoComponent, selected: Boolean) {
    observe(component.node())
    val model = component.model() ?: return
    val colors = MaterialTheme.colorScheme
    Card(
        Modifier.fillMaxWidth().height(96.dp),
        colors = CardDefaults.cardColors(
            containerColor = if (selected) colors.tertiaryContainer else colors.surfaceContainerHigh,
        ),
    ) {
        Column(Modifier.padding(12.dp)) {
            Text("#${model.number}", style = MaterialTheme.typography.titleMedium)
            Text("${model.developed}", style = MaterialTheme.typography.bodySmall)
        }
    }
}
