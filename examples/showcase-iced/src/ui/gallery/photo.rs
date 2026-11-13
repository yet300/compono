use super::super::widgets::View;
use compono::{Handle, Tree};
use iced::widget::{column, container, text};
use iced_aw::Badge;
use showcase::feature::gallery::Photo;

/// One photo, which develops while it is visible. A badge shows how far.
pub fn view(tree: &Tree, key: u64, handle: Handle<Photo>, selected: bool) -> View<'_> {
    let developed = tree.get(handle).map_or(0, |photo| photo.developed);
    let badge = Badge::new(text(developed.to_string())).style(if selected {
        iced_aw::style::badge::primary
    } else {
        iced_aw::style::badge::secondary
    });
    container(column![text(format!("#{key}")).size(18), badge].spacing(6))
        .padding(10)
        .width(88)
        .height(72)
        .style(if selected {
            container::primary
        } else {
            container::rounded_box
        })
        .into()
}
