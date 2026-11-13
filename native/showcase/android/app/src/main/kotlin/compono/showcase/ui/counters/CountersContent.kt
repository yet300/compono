package compono.showcase.ui.counters

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import compono.android.ChildStack
import compono.android.LocalComponoTree
import compono.android.observe
import compono.android.rememberComponent
import compono.showcase.ffi.ChildRef
import compono.showcase.ffi.CountersComponent

/** A stack of counters that can be a thousand deep. Back pops one. */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun CountersContent(component: CountersComponent, modifier: Modifier = Modifier) {
    observe(component.node())
    val model = component.model() ?: return
    val active = component.active() ?: return
    Scaffold(
        modifier,
        topBar = {
            TopAppBar(
                title = { Text("Stack depth ${model.depth}") },
                navigationIcon = {
                    if (model.depth > 1) {
                        IconButton(component::back) {
                            Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = "Back")
                        }
                    }
                },
            )
        },
    ) { padding ->
        Column(Modifier.fillMaxSize().padding(padding)) {
            ChildStack(
                active,
                ChildRef::key,
                Modifier.weight(1f).fillMaxWidth(),
                tree = LocalComponoTree.current,
                owner = component.node(),
                previous = component.revealed(),
                background = MaterialTheme.colorScheme.background,
            ) { child ->
                CounterContent(rememberComponent(child.node) { component.counter(child) })
            }
            Row(
                Modifier.fillMaxWidth().padding(16.dp),
                horizontalArrangement = Arrangement.spacedBy(12.dp),
            ) {
                OutlinedButton(component::goDeep, Modifier.weight(1f)) {
                    Text("Push ${model.deepPush} screens")
                }
                OutlinedButton(component::popToRoot, Modifier.weight(1f)) { Text("Back to first") }
            }
        }
    }
}
