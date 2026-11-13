//! Building blocks every screen shares, made of `gpui-component` widgets.

use crate::ShowcaseView;
use gpui::prelude::*;
use gpui::{App, ClickEvent, Context, Div, StyleRefinement, Window, div, px};
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::group_box::{GroupBox, GroupBoxVariants};
use gpui_component::{ActiveTheme, h_flex, v_flex};

/// The context every view builds in.
pub type Cx<'a> = Context<'a, ShowcaseView>;

/// A style for a cached view that fills its parent.
#[must_use]
pub fn fill() -> StyleRefinement {
    StyleRefinement::default().size_full()
}

/// Text that tests can find by its content.
pub fn label(content: impl Into<String>) -> Div {
    let content = content.into();
    let selector = content.clone();
    div().debug_selector(move || selector).child(content)
}

/// A button named `name`, found by tests under the same name.
fn button(name: &'static str) -> Button {
    Button::new(name)
        .label(name)
        .debug_selector(move || name.to_owned())
}

/// The main action of a screen.
pub fn primary(
    name: &'static str,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> Button {
    button(name).primary().on_click(on_click)
}

/// Any other action.
pub fn action(
    name: &'static str,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> Button {
    button(name).outline().on_click(on_click)
}

/// One of several options, drawn selected or not.
pub fn choice(
    name: &'static str,
    selected: bool,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> Button {
    let button = button(name).on_click(on_click);
    if selected {
        button.primary()
    } else {
        button.ghost()
    }
}

/// A screen title with actions on the right.
pub fn title(text: impl Into<String>, actions: impl IntoIterator<Item = Button>) -> Div {
    h_flex()
        .w_full()
        .justify_between()
        .gap_3()
        .child(label(text).text_2xl())
        .child(h_flex().gap_2().children(actions))
}

/// A card with a heading.
pub fn card(heading: impl IntoElement) -> GroupBox {
    GroupBox::new().fill().title(heading)
}

/// Covers the window and keeps the mouse away from what is underneath.
pub fn modal(content: impl IntoElement, cx: &Cx<'_>) -> Div {
    let theme = cx.theme();
    div()
        .absolute()
        .top_0()
        .left_0()
        .size_full()
        .occlude()
        .flex()
        .items_center()
        .justify_center()
        .bg(theme.overlay)
        .child(
            v_flex()
                .w(px(340.0))
                .p_5()
                .gap_3()
                .rounded(theme.radius_lg)
                .border_1()
                .border_color(theme.border)
                .bg(theme.popover)
                .text_color(theme.popover_foreground)
                .shadow_lg()
                .child(content),
        )
}
