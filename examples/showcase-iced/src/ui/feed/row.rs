use super::super::widgets::View;
use compono::{Handle, Tree};
use compono_iced::send;
use iced::widget::{button, container, row, text};
use iced::{Alignment, Fill};
use showcase::feature::feed::{Row, RowMsg};

/// One row. It has a component only while it is on screen.
pub fn view(tree: &Tree, index: usize, handle: Option<Handle<Row>>, height: f32) -> View<'_> {
    let Some((handle, model)) = handle.and_then(|handle| tree.get(handle).map(|row| (handle, row)))
    else {
        return container(text(format!("Row {index}")).style(text::secondary))
            .height(height)
            .into();
    };
    let like = if model.liked {
        button(text("Liked")).style(button::primary)
    } else {
        button(text("Like")).style(button::secondary)
    };
    row![
        text(format!("Row {}  value {}", model.id, model.value)).width(Fill),
        like.on_press(send(handle, RowMsg::Like)),
    ]
    .align_y(Alignment::Center)
    .height(height)
    .into()
}
