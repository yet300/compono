package compono.showcase.ui.book

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.Button
import androidx.compose.material3.OutlinedCard
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import compono.android.observe
import compono.showcase.ffi.PageComponent

/** One page, which keeps its tap count while it stays alive. */
@Composable
fun PageContent(component: PageComponent) {
    observe(component.node())
    val model = component.model() ?: return
    Box(Modifier.fillMaxSize().padding(16.dp), contentAlignment = Alignment.Center) {
        OutlinedCard {
            Column(
                Modifier.padding(32.dp),
                horizontalAlignment = Alignment.CenterHorizontally,
                verticalArrangement = Arrangement.spacedBy(12.dp),
            ) {
                Text("Page ${model.number}", style = MaterialTheme.typography.displaySmall)
                Text("Tapped ${model.taps} times")
                Button(component::tap) { Text("Tap") }
            }
        }
    }
}
