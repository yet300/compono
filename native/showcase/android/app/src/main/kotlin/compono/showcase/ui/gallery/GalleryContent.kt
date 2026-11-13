package compono.showcase.ui.gallery

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.grid.GridCells
import androidx.compose.foundation.lazy.grid.LazyVerticalGrid
import androidx.compose.foundation.lazy.grid.items
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.automirrored.filled.KeyboardArrowLeft
import androidx.compose.material.icons.automirrored.filled.KeyboardArrowRight
import androidx.compose.material.icons.filled.Delete
import androidx.compose.material3.BottomAppBar
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.FilterChip
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
import compono.showcase.ffi.GalleryComponent

/** A filmstrip: a custom router written with only the public API. */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun GalleryContent(component: GalleryComponent, modifier: Modifier = Modifier) {
    observe(component.node())
    val contact = component.isContact()
    val frames = component.frames()
    val shown = if (contact) frames else frames.filter { it.selected }
    Scaffold(
        modifier,
        topBar = {
            TopAppBar(
                title = { Text("Filmstrip") },
                navigationIcon = {
                    IconButton(component::dismiss) {
                        Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = "Close")
                    }
                },
                actions = {
                    FilterChip(contact, component::toggleMode, label = { Text("Contact sheet") })
                },
            )
        },
        bottomBar = {
            BottomAppBar(
                actions = {
                    IconButton(component::previous) {
                        Icon(Icons.AutoMirrored.Filled.KeyboardArrowLeft, contentDescription = "Previous")
                    }
                    IconButton(component::next) {
                        Icon(Icons.AutoMirrored.Filled.KeyboardArrowRight, contentDescription = "Next")
                    }
                    IconButton(component::removeSelected) {
                        Icon(Icons.Filled.Delete, contentDescription = "Remove")
                    }
                },
            )
        },
    ) { padding ->
        LazyVerticalGrid(
            GridCells.Adaptive(104.dp),
            Modifier.fillMaxSize().padding(padding),
            contentPadding = PaddingValues(16.dp),
            verticalArrangement = Arrangement.spacedBy(8.dp),
            horizontalArrangement = Arrangement.spacedBy(8.dp),
        ) {
            items(shown, key = { it.child.key }) { frame ->
                PhotoContent(rememberComponent(frame.child.node) { component.photo(frame) }, frame.selected)
            }
        }
    }
}
