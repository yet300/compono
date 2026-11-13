// The Decompose side of the comparison. `rust/src/main.rs` is the other side.
@file:OptIn(ExperimentalDecomposeApi::class)

package compono.compare

import com.arkivanov.decompose.ComponentContext
import com.arkivanov.decompose.DefaultComponentContext
import com.arkivanov.decompose.ExperimentalDecomposeApi
import com.arkivanov.decompose.router.items.Items
import com.arkivanov.decompose.router.items.ItemsNavigation
import com.arkivanov.decompose.router.items.childItems
import com.arkivanov.decompose.router.items.navigate
import com.arkivanov.decompose.router.pages.Pages
import com.arkivanov.decompose.router.pages.PagesNavigation
import com.arkivanov.decompose.router.pages.childPages
import com.arkivanov.decompose.router.pages.select
import com.arkivanov.decompose.router.slot.SlotNavigation
import com.arkivanov.decompose.router.slot.activate
import com.arkivanov.decompose.router.slot.childSlot
import com.arkivanov.decompose.router.slot.dismiss
import com.arkivanov.decompose.router.stack.StackNavigation
import com.arkivanov.decompose.router.stack.childStack
import com.arkivanov.decompose.router.stack.pop
import com.arkivanov.decompose.router.stack.popTo
import com.arkivanov.decompose.router.stack.push
import com.arkivanov.decompose.value.MutableValue
import com.arkivanov.decompose.value.update
import com.arkivanov.essenty.lifecycle.LifecycleRegistry
import com.arkivanov.essenty.lifecycle.destroy
import com.arkivanov.essenty.lifecycle.resume
import kotlin.time.TimeSource

private const val ROUNDS = 31
private const val WARMUP_ROUNDS = 20

private var blackhole = 0L

/** Divides iteration counts on platforms too slow for the full run. */
private var divisor = 1

private fun scaled(iterations: Int): Int = (iterations / divisor).coerceAtLeast(1)

private fun context(): Pair<LifecycleRegistry, ComponentContext> {
    val lifecycle = LifecycleRegistry()
    val context = DefaultComponentContext(lifecycle)
    lifecycle.resume()
    return lifecycle to context
}

private class Leaf(context: ComponentContext) : ComponentContext by context {
    val value = MutableValue(0L)

    fun bump() {
        value.update { it + 1 }
    }
}

private inline fun measure(
    report: (String) -> Unit,
    name: String,
    iterations: Int,
    perCycle: Int = 1,
    body: () -> Unit,
) {
    val count = scaled(iterations)
    repeat(WARMUP_ROUNDS) {
        repeat(count) { body() }
    }
    val samples = DoubleArray(ROUNDS)
    for (round in samples.indices) {
        val start = TimeSource.Monotonic.markNow()
        repeat(count) { body() }
        samples[round] = start.elapsedNow().inWholeNanoseconds.toDouble() / count / perCycle
    }
    samples.sort()
    report("$name\t${format(samples[ROUNDS / 2])}\t${format(samples[ROUNDS / 10])}\t${format(samples[ROUNDS * 9 / 10])}")
}

private fun format(value: Double): String {
    val tenths = (value * 10).toLong()
    return "${tenths / 10}.${tenths % 10}"
}

private class StackRoot(context: ComponentContext) : ComponentContext by context {
    val navigation = StackNavigation<Int>()
    val stack = childStack(
        source = navigation,
        serializer = null,
        initialConfiguration = 0,
        handleBackButton = false,
    ) { _, childContext -> Leaf(childContext) }

    init {
        stack.subscribe { blackhole += it.backStack.size }
    }
}

private fun stackPushPop(report: (String) -> Unit, depth: Int) {
    val (_, context) = context()
    val root = StackRoot(context)
    for (route in 1 until depth) root.navigation.push(route)
    measure(report, "stack_push_pop_$depth", 20_000) {
        root.navigation.push(1_000_000)
        root.navigation.pop()
    }
}

private fun deepStack(report: (String) -> Unit) {
    val depth = 1_000
    measure(report, "deep_stack_per_entry", 20, depth) {
        val (_, context) = context()
        val root = StackRoot(context)
        for (route in 1 until depth) root.navigation.push(route)
        root.navigation.popTo(0)
    }
}

private fun slotOpenClose(report: (String) -> Unit) {
    val (_, context) = context()
    val navigation = SlotNavigation<Int>()
    val slot = context.childSlot(
        source = navigation,
        serializer = null,
        handleBackButton = false,
    ) { _, childContext -> Leaf(childContext) }
    slot.subscribe { blackhole += if (it.child == null) 0 else 1 }
    measure(report, "slot_open_close", 20_000) {
        navigation.activate(1)
        navigation.dismiss()
    }
}

private fun pagesSelect(report: (String) -> Unit) {
    val (_, context) = context()
    val navigation = PagesNavigation<Int>()
    val pages = context.childPages(
        source = navigation,
        serializer = null,
        initialPages = { Pages(items = (0 until 100).toList(), selectedIndex = 0) },
    ) { _, childContext -> Leaf(childContext) }
    pages.subscribe { blackhole += it.selectedIndex }
    var index = 0
    var step = 1
    measure(report, "pages_select_next", 20_000) {
        if (index == 99) step = -1 else if (index == 0) step = 1
        index += step
        navigation.select(index)
    }
}

private fun itemsScroll(report: (String) -> Unit) {
    val (_, context) = context()
    val navigation = ItemsNavigation<Int>()
    val items = context.childItems(
        source = navigation,
        serializer = null,
        initialItems = { Items(items = (0 until 100_000).toList()) },
    ) { _, childContext -> Leaf(childContext) }
    items.subscribe { blackhole += it.items.size }
    fun show(first: Int) {
        navigation.navigate { old ->
            old.copy(activeItems = (first until first + 30).associateWith { Items.ActiveLifecycleState.STARTED })
        }
    }
    show(0)
    // Decompose creates a row when the UI first asks for it.
    for (row in 0 until 30) blackhole += items[row].value.value
    var first = 0
    var step = 1
    measure(report, "items_scroll_one_row", 2_000) {
        if (first == 1_000) step = -1 else if (first == 0) step = 1
        first += step
        show(first)
        val entering = if (step > 0) first + 29 else first
        blackhole += items[entering].value.value
    }
}

private fun updateChild(report: (String) -> Unit) {
    val (_, context) = context()
    val root = StackRoot(context)
    val leaf = root.stack.value.active.instance
    leaf.value.subscribe { blackhole += it }
    measure(report, "update_child", 200_000) { leaf.bump() }
}

private fun createDestroy(report: (String) -> Unit) {
    measure(report, "create_destroy_tree", 20_000) {
        val (lifecycle, context) = context()
        blackhole += StackRoot(context).stack.value.backStack.size
        lifecycle.destroy()
    }
}

private fun memory(report: (String) -> Unit, usedBytes: () -> Long) {
    val entries = 10_000
    val (_, context) = context()
    val root = StackRoot(context)
    val before = usedBytes()
    for (route in 1..entries) root.navigation.push(route)
    val after = usedBytes()
    report("memory_bytes_per_entry\t${(after - before) / entries}")
    blackhole += root.stack.value.backStack.size
}

fun runAll(report: (String) -> Unit, usedBytes: (() -> Long)?, iterationDivisor: Int = 1) {
    divisor = iterationDivisor
    stackPushPop(report, 1)
    stackPushPop(report, 10)
    stackPushPop(report, 100)
    deepStack(report)
    slotOpenClose(report)
    pagesSelect(report)
    itemsScroll(report)
    updateChild(report)
    createDestroy(report)
    if (usedBytes != null) memory(report, usedBytes)
    report("# done $blackhole")
}
