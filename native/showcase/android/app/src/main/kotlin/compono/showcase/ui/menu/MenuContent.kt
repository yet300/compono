package compono.showcase.ui.menu

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.Button
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.FilledTonalButton
import androidx.compose.material3.LargeTopAppBar
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import compono.android.observe
import compono.android.rememberComponent
import compono.showcase.ffi.MenuComponent

/** Links to the other screens, and a dialog held in a slot. */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun MenuContent(component: MenuComponent, modifier: Modifier = Modifier) {
    observe(component.node())
    Scaffold(modifier, topBar = { LargeTopAppBar(title = { Text("Showcase") }) }) { padding ->
        Column(
            Modifier.fillMaxSize().padding(padding).padding(16.dp),
            verticalArrangement = Arrangement.spacedBy(12.dp),
        ) {
            Button(component::openBook, Modifier.fillMaxWidth()) { Text("Open pages") }
            FilledTonalButton(component::openGallery, Modifier.fillMaxWidth()) {
                Text("Open custom navigation")
            }
            OutlinedButton(component::showDialog, Modifier.fillMaxWidth()) { Text("Show dialog") }
        }
    }
    component.dialog()?.let { child ->
        DialogContent(rememberComponent(child.node) { component.dialogComponent(child) })
    }
}
