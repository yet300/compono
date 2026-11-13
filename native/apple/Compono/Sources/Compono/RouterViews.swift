import SwiftUI

/// How a stack moves between entries.
public enum StackAnimation: Sendable {
    /// The new entry slides over the old one, and Back slides it off.
    case slide
    case fade
    case none
}

private let transitionDuration = 0.28
/// How far a covered entry sits to the left, as a share of the width.
private let parallax: CGFloat = 0.3

/// Draws the active entry of a stack, with transitions and an interactive Back.
///
/// A component is destroyed as soon as it leaves the stack. The view on its way out
/// is the last one drawn, so it needs nothing further from the tree.
public struct ChildStack<Child: ComponoChild, Content: View>: View {
    private let tree: ComponoTree?
    private let owner: Int64?
    private let active: Child
    private let previous: Child?
    private let animation: StackAnimation
    private let content: (Child) -> Content

    @State private var shown: Child
    @State private var outgoing: Child?
    @State private var progress: CGFloat = 0
    @State private var forward = true
    @State private var generation = 0

    /// Creates a stack that moves between entries but has no Back gesture.
    public init(
        active: Child,
        animation: StackAnimation = .slide,
        @ViewBuilder content: @escaping (Child) -> Content
    ) {
        self.tree = nil
        self.owner = nil
        self.active = active
        self.previous = nil
        self.animation = animation
        self.content = content
        _shown = State(initialValue: active)
    }

    /// Creates a stack that follows Back gestures aimed at `owner`.
    ///
    /// `previous` is the entry that Back would reveal.
    public init(
        tree: ComponoTree,
        owner: Int64,
        active: Child,
        previous: Child?,
        animation: StackAnimation = .slide,
        @ViewBuilder content: @escaping (Child) -> Content
    ) {
        self.tree = tree
        self.owner = owner
        self.active = active
        self.previous = previous
        self.animation = animation
        self.content = content
        _shown = State(initialValue: active)
    }

    private struct Layer: Identifiable {
        let child: Child
        let shift: CGFloat
        let opacity: Double
        var id: Int64 { child.key }
    }

    private var top: Child { active.key == shown.key ? active : shown }

    private var gestureProgress: CGFloat? {
        guard let gesture = tree?.gesture, gesture.target == owner else { return nil }
        return gesture.progress
    }

    /// Returns the entries to draw, from bottom to top.
    private var layers: [Layer] {
        if let gesture = gestureProgress {
            let under = previous.map { Layer(child: $0, shift: -parallax * (1 - gesture), opacity: 1) }
            return (under.map { [$0] } ?? []) + [Layer(child: top, shift: gesture, opacity: 1)]
        }
        guard let outgoing else { return [Layer(child: top, shift: 0, opacity: 1)] }
        if animation == .fade {
            return [
                Layer(child: outgoing, shift: 0, opacity: 1 - progress),
                Layer(child: top, shift: 0, opacity: progress),
            ]
        }
        if forward {
            return [
                Layer(child: outgoing, shift: -parallax * progress, opacity: 1),
                Layer(child: top, shift: 1 - progress, opacity: 1),
            ]
        }
        return [
            Layer(child: top, shift: -parallax * (1 - progress), opacity: 1),
            Layer(child: outgoing, shift: progress, opacity: 1),
        ]
    }

    public var body: some View {
        GeometryReader { geometry in
            let layers = layers
            ZStack {
                ForEach(layers) { layer in
                    content(layer.child)
                        .frame(maxWidth: .infinity, maxHeight: .infinity)
                        .background(.background)
                        .offset(x: layer.shift * geometry.size.width)
                        .opacity(layer.opacity)
                        .allowsHitTesting(layer.id == layers.last?.id)
                }
            }
        }
        // No clipping: iOS 26 draws tab bars and toolbars in the safe area, outside
        // this frame, and a clip would hide them. A layer moving off screen is cut
        // by the window instead.
        .onAppear { if let owner { tree?.registerStack(owner) } }
        .onDisappear { if let owner { tree?.unregisterStack(owner) } }
        .onChange(of: active.key) { _, _ in moveToActive() }
    }

    private func moveToActive() {
        let committed = owner.map { tree?.takeCommitted($0) ?? false } ?? false
        guard !committed, animation != .none else {
            shown = active
            outgoing = nil
            return
        }
        forward = active.key >= shown.key
        outgoing = shown
        shown = active
        generation += 1
        let finished = generation
        withAnimation(.easeInOut(duration: transitionDuration), completionCriteria: .logicallyComplete) {
            progress = 1
        } completion: {
            guard finished == generation else { return }
            outgoing = nil
            progress = 0
        }
    }
}

/// Draws the entry of a slot, if one is open, fading it in and out.
public struct ChildSlot<Child: ComponoChild, Content: View>: View {
    private let active: Child?
    private let content: (Child) -> Content

    public init(active: Child?, @ViewBuilder content: @escaping (Child) -> Content) {
        self.active = active
        self.content = content
    }

    public var body: some View {
        ZStack {
            if let active {
                content(active).id(active.key).transition(.opacity)
            }
        }
        .animation(.easeInOut(duration: 0.2), value: active?.key)
    }
}

/// Draws the selected page, sliding between pages by their order.
public struct ChildPages<Child: ComponoChild, Content: View>: View {
    private let active: Child?
    private let content: (Child) -> Content

    public init(active: Child?, @ViewBuilder content: @escaping (Child) -> Content) {
        self.active = active
        self.content = content
    }

    public var body: some View {
        if let active {
            ChildStack(active: active, animation: .slide, content: content)
        }
    }
}

/// Draws the panels on screen side by side.
public struct ChildPanels<Child: ComponoChild, Content: View>: View {
    private let panels: [Child]
    private let content: (Child) -> Content

    public init(panels: [Child], @ViewBuilder content: @escaping (Child) -> Content) {
        self.panels = panels
        self.content = content
    }

    public var body: some View {
        HStack(alignment: .top, spacing: 12) {
            ForEach(panels, id: \.key) { panel in
                content(panel).frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
            }
        }
    }
}

/// How many spare rows a list keeps alive on each side of the rows on screen, so a
/// fast scroll reaches rows whose components already exist.
public let componoOverscan = 10

/// Returns the window to report, or `nil` while `current` still covers the rows on
/// screen. The window is the visible rows plus `overscan` rows on each side.
func nextWindow(visible: Range<Int>, current: Range<Int>?, overscan: Int, count: Int) -> Range<Int>? {
    let end = min(visible.upperBound, count)
    let start = min(visible.lowerBound, end)
    if let current, current.lowerBound <= start, end <= current.upperBound, current.upperBound <= count {
        return nil
    }
    let window = max(start - overscan, 0)..<min(end + overscan, count)
    return window == current ? nil : window
}

/// Tracks which rows are on screen and reports them once per main-loop turn.
@MainActor
final class WindowTracker {
    var report: (Range<Int>) -> Void = { _ in }
    var count = 0
    var overscan = componoOverscan
    private var shown = Set<Int>()
    private var reported: Range<Int>?
    private var flushQueued = false

    func configure(count: Int, overscan: Int) {
        self.count = count
        self.overscan = overscan
    }

    func appear(_ index: Int) {
        shown.insert(index)
        queueFlush()
    }

    func disappear(_ index: Int) {
        shown.remove(index)
        queueFlush()
    }

    private func queueFlush() {
        guard !flushQueued else { return }
        flushQueued = true
        DispatchQueue.main.async { [weak self] in
            MainActor.assumeIsolated {
                self?.flush()
            }
        }
    }

    func flush() {
        flushQueued = false
        guard let first = shown.min(), let last = shown.max() else { return }
        if let window = nextWindow(visible: first..<(last + 1), current: reported, overscan: overscan, count: count) {
            reported = window
            report(window)
        }
    }
}

/// A long list that reports the rows on screen to its owner.
///
/// It reports the rows on screen plus `overscan` spare rows on each side, and
/// reports again only when the rows on screen leave that window.
public struct ChildList<Row: View>: View {
    private let count: Int
    private let overscan: Int
    private let onWindow: (Range<Int>) -> Void
    private let row: (Int) -> Row
    @State private var tracker = WindowTracker()

    public init(
        count: Int,
        overscan: Int = componoOverscan,
        onWindow: @escaping (Range<Int>) -> Void,
        @ViewBuilder row: @escaping (Int) -> Row
    ) {
        self.count = count
        self.overscan = overscan
        self.onWindow = onWindow
        self.row = row
    }

    public var body: some View {
        // The tracker is not observed, so updating it here causes no redraw.
        let _ = tracker.configure(count: count, overscan: overscan)
        ScrollView {
            LazyVStack(spacing: 0) {
                ForEach(0..<count, id: \.self) { index in
                    row(index)
                        .onAppear { tracker.appear(index) }
                        .onDisappear { tracker.disappear(index) }
                }
            }
        }
        .onAppear { tracker.report = onWindow }
    }
}
