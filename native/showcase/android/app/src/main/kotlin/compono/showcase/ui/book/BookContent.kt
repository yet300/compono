package compono.showcase.ui.book

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.automirrored.filled.KeyboardArrowLeft
import androidx.compose.material.icons.automirrored.filled.KeyboardArrowRight
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.FilledTonalIconButton
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.LinearProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import compono.android.ChildPages
import compono.android.observe
import compono.android.rememberComponent
import compono.showcase.ffi.BookComponent
import compono.showcase.ffi.ChildRef

/** A hundred pages, of which only the selected one and its neighbours exist. */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun BookContent(component: BookComponent, modifier: Modifier = Modifier) {
    observe(component.node())
    val model = component.model() ?: return
    Scaffold(
        modifier,
        topBar = {
            TopAppBar(
                title = { Text("Page ${model.selected + 1} of ${model.pages}") },
                navigationIcon = {
                    IconButton(component::dismiss) {
                        Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = "Close")
                    }
                },
                actions = { TextButton({ component.select(49) }) { Text("Jump to 50") } },
            )
        },
    ) { padding ->
        Column(Modifier.fillMaxSize().padding(padding)) {
            LinearProgressIndicator(
                progress = { (model.selected + 1f) / model.pages },
                modifier = Modifier.fillMaxWidth(),
            )
            Text(
                "${model.alive} pages alive",
                Modifier.padding(16.dp),
                style = MaterialTheme.typography.labelLarge,
            )
            ChildPages(
                component.active(),
                ChildRef::key,
                Modifier.weight(1f).fillMaxWidth(),
                MaterialTheme.colorScheme.background,
            ) { page -> PageContent(rememberComponent(page.node) { component.page(page) }) }
            Row(
                Modifier.fillMaxWidth().padding(16.dp),
                horizontalArrangement = Arrangement.SpaceBetween,
                verticalAlignment = Alignment.CenterVertically,
            ) {
                FilledTonalIconButton(component::previous) {
                    Icon(Icons.AutoMirrored.Filled.KeyboardArrowLeft, contentDescription = "Previous")
                }
                FilledTonalIconButton(component::next) {
                    Icon(Icons.AutoMirrored.Filled.KeyboardArrowRight, contentDescription = "Next")
                }
            }
        }
    }
}
