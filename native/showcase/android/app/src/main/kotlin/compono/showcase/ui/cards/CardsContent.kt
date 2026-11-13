package compono.showcase.ui.cards

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Add
import androidx.compose.material.icons.filled.Delete
import androidx.compose.material.icons.filled.Refresh
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.ExtendedFloatingActionButton
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import compono.android.observe
import compono.android.rememberComponent
import compono.showcase.ffi.CardsComponent

/** A pile of cards kept in any order. Only the top card ticks. */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun CardsContent(component: CardsComponent, modifier: Modifier = Modifier) {
    observe(component.node())
    // Read here and not inside LazyColumn, whose block does not rerun on its own.
    val cards = component.cards()
    Scaffold(
        modifier,
        topBar = {
            TopAppBar(
                title = { Text("Cards") },
                actions = {
                    IconButton(component::sendTopToBack) {
                        Icon(Icons.Filled.Refresh, contentDescription = "Send top to back")
                    }
                    IconButton(component::remove) {
                        Icon(Icons.Filled.Delete, contentDescription = "Remove")
                    }
                },
            )
        },
        floatingActionButton = {
            ExtendedFloatingActionButton(
                onClick = component::add,
                icon = { Icon(Icons.Filled.Add, contentDescription = null) },
                text = { Text("Add") },
            )
        },
    ) { padding ->
        LazyColumn(
            Modifier.fillMaxSize().padding(padding),
            contentPadding = PaddingValues(16.dp),
            verticalArrangement = Arrangement.spacedBy(8.dp),
        ) {
            items(cards, key = { it.key }) { child ->
                CardContent(rememberComponent(child.node) { component.card(child) }, focused = child.phase == 2)
            }
        }
    }
}
