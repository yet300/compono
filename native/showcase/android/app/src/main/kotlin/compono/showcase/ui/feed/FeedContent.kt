package compono.showcase.ui.feed

import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Switch
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import compono.android.ChildList
import compono.android.observe
import compono.showcase.ffi.FeedComponent

/** 100 000 rows. Only the rows on screen have components. */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun FeedContent(component: FeedComponent, modifier: Modifier = Modifier) {
    observe(component.node())
    val model = component.model() ?: return
    Scaffold(
        modifier,
        topBar = {
            TopAppBar(
                title = { Text("${model.rows} rows, ${model.liveRows} live") },
                actions = {
                    Switch(model.live, { component.toggleLive() }, Modifier.padding(end = 16.dp))
                },
            )
        },
    ) { padding ->
        ChildList(
            count = model.rows,
            onWindow = component::setWindow,
            modifier = Modifier.fillMaxSize().padding(padding),
        ) { index -> RowContent(component, index) }
    }
}
