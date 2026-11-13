//! Building blocks every screen shares, made of iced and `iced_aw` widgets.

use compono_iced::Action;
use iced::widget::{button, center, container, opaque, row, stack, text};
use iced::{Color, Element, Fill, Length};
use iced_aw::Card;

/// What every view returns.
pub type View<'a> = Element<'a, Action>;

/// A button that runs `action` on the tree.
pub fn action<'a>(label: &'a str, action: Action) -> View<'a> {
    button(text(label)).on_press(action).into()
}

/// A secondary button.
pub fn subtle<'a>(label: &'a str, action: Action) -> View<'a> {
    button(text(label))
        .style(button::secondary)
        .on_press(action)
        .into()
}

/// A screen title with actions on the right.
pub fn title<'a>(label: String, actions: Vec<View<'a>>) -> View<'a> {
    let actions = actions
        .into_iter()
        .fold(row![].spacing(8), iced::widget::Row::push);
    container(
        row![text(label).size(24).width(Fill), actions]
            .spacing(12)
            .align_y(iced::Alignment::Center),
    )
    .padding([8, 0])
    .into()
}

/// An `iced_aw` card with a heading and a body.
pub fn card<'a>(head: impl Into<View<'a>>, body: impl Into<View<'a>>) -> View<'a> {
    Card::new(head, body)
        .width(Length::Shrink)
        .style(iced_aw::style::card::primary)
        .into()
}

/// Shows `dialog` above `base` and keeps input away from `base`.
pub fn modal<'a>(base: View<'a>, dialog: View<'a>) -> View<'a> {
    let scrim = center(dialog).style(|_theme| container::Style {
        background: Some(
            Color {
                a: 0.55,
                ..Color::BLACK
            }
            .into(),
        ),
        ..container::Style::default()
    });
    stack![base, opaque(scrim)].into()
}
