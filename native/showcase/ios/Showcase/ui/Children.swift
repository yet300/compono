import Compono
import ShowcaseFFI

// Router children as the router views see them: a key and a component's address.

extension ChildRef: @retroactive ComponoChild {}

extension ScreenChild: @retroactive ComponoChild {
    public var key: Int64 { child.key }
    public var node: Int64 { child.node }
}

extension TabsChild: @retroactive ComponoChild {
    public var key: Int64 { child.key }
    public var node: Int64 { child.node }
}

extension PanelChild: @retroactive ComponoChild {
    public var key: Int64 { child.key }
    public var node: Int64 { child.node }
}

extension FrameChild: @retroactive ComponoChild {
    public var key: Int64 { child.key }
    public var node: Int64 { child.node }
}
