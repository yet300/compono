package compono.compare

private fun usedBytes(): Long {
    repeat(4) { System.gc() }
    val runtime = Runtime.getRuntime()
    return runtime.totalMemory() - runtime.freeMemory()
}

fun main() {
    runAll(::println, ::usedBytes)
}
