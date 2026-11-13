package compono.showcase.ui.menu

import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import compono.android.observe
import compono.showcase.ffi.DialogComponent

/** A dialog whose text and dismissal live in its component. */
@Composable
fun DialogContent(component: DialogComponent) {
    observe(component.node())
    AlertDialog(
        onDismissRequest = component::dismiss,
        confirmButton = { TextButton(component::dismiss) { Text("Dismiss") } },
        title = { Text("Slot") },
        text = { Text(component.text().orEmpty()) },
    )
}
