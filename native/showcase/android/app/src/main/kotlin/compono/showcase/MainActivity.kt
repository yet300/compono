package compono.showcase

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.ui.Modifier
import compono.android.ComponoHost
import compono.android.ComponoListener
import compono.android.rememberComponent
import compono.android.retainedTree
import compono.android.treeBridge
import compono.showcase.ffi.Listener
import compono.showcase.ffi.ShowcaseApp
import compono.showcase.ui.root.RootContent
import compono.showcase.ui.theme.ShowcaseTheme

class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        enableEdgeToEdge()
        super.onCreate(savedInstanceState)
        val root = retainedTree(
            create = { listener, saved -> ShowcaseApp(object : Listener, ComponoListener by listener {}, saved) },
            bridge = { app ->
                treeBridge(
                    app::pump, app::setPhase, app::back, app::backStart,
                    app::backCancel, app::backCommit, app::contains, app::save,
                )
            },
        )
        setContent {
            ShowcaseTheme {
                ComponoHost(root.tree) {
                    RootContent(rememberComponent(root.app) { root.app.root() }, Modifier.fillMaxSize())
                }
            }
        }
    }
}
