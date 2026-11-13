package compono.showcase.ui.counters

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.ElevatedCard
import androidx.compose.material3.FilledTonalButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import compono.android.observe
import compono.showcase.ffi.CounterComponent

/** One counter. It ticks only while it is the focused one. */
@Composable
fun CounterContent(component: CounterComponent) {
    observe(component.node())
    val model = component.model() ?: return
    Box(Modifier.fillMaxSize().padding(16.dp), contentAlignment = Alignment.Center) {
        ElevatedCard {
            Column(
                Modifier.padding(24.dp),
                horizontalAlignment = Alignment.CenterHorizontally,
                verticalArrangement = Arrangement.spacedBy(8.dp),
            ) {
                Text("Counter ${model.index}", style = MaterialTheme.typography.titleMedium)
                Text("${model.value}", style = MaterialTheme.typography.displayMedium)
                Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                    FilledTonalButton(component::previous) { Text("Previous") }
                    FilledTonalButton(component::next) { Text("Next") }
                }
            }
        }
    }
}
