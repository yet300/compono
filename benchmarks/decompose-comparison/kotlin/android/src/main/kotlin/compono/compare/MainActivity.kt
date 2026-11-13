package compono.compare

import android.app.Activity
import android.os.Bundle
import android.os.Handler
import android.os.Looper
import android.util.Log

private fun usedBytes(): Long {
    repeat(4) { System.gc() }
    val runtime = Runtime.getRuntime()
    return runtime.totalMemory() - runtime.freeMemory()
}

class MainActivity : Activity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        // Decompose asks for the main thread. Nothing else runs in this app.
        Handler(Looper.getMainLooper()).post {
            runAll({ Log.i("ComponoCompare", it) }, ::usedBytes, iterationDivisor = 10)
        }
    }
}
