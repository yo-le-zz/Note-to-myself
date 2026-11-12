use iced::widget::{column, row, text_editor, Space};
use iced::{Alignment, Element, Length};
use lucide_icons::iced::{icon_arrow_left, icon_save};
use ntm_core::plugin::host::Zone;

use super::home::plugin_buttons;
use super::style::pal;
use crate::app::{App, Message};

pub fn view(app: &App) -> Element<'_, Message> {
    let p = pal(app);
    let size = app.settings.font_size.clamp(12, 24);

    let title_placeholder = app.t("title_placeholder");
    let title_input = p
        .input(&title_placeholder, &app.title)
        .on_input(Message::TitleChanged)
        .padding(p.sp(12.0) as u16)
        .size(size)
        .font(p.editor_font)
        .width(Length::Fill);

    let editor = text_editor(&app.body)
        .placeholder(app.t("body_placeholder"))
        .on_action(Message::BodyEdited)
        .padding(p.sp(12.0) as u16)
        .size(size)
        .font(p.editor_font)
        .height(Length::Fill)
        .style(p.editor_style());

    let back = p.button(
        row![
            icon_arrow_left().size(16).color(p.fg),
            p.label(app.t("back"), 13)
        ]
        .spacing(8)
        .align_y(Alignment::Center),
        [8, 12],
        Some(Message::BackFromWrite),
    );
    let save = p.button(
        row![icon_save().size(16).color(p.fg), p.label(app.t("save"), 13)]
            .spacing(8)
            .align_y(Alignment::Center),
        [8, 14],
        Some(Message::SaveNote),
    );

    let body_text = app.body.text();
    let char_count = body_text
        .strip_suffix('\n')
        .unwrap_or(&body_text)
        .chars()
        .count();
    let actions = row![
        back,
        save,
        plugin_buttons(app, p, Zone::Write),
        Space::with_width(Length::Fill),
        p.muted(format!("{} {}", char_count, app.t("characters")), 12),
    ]
    .spacing(10)
    .align_y(Alignment::Center);

    let mut content = column![
        p.label(app.t("write"), 25),
        p.muted(app.t("subtitle"), 13),
        title_input,
        editor,
        actions
    ]
    .spacing(if app.settings.compact_mode {
        p.sp(8.0)
    } else {
        p.sp(12.0)
    })
    .align_x(Alignment::Start)
    .width(Length::Fill)
    .height(Length::Fill);
    if !app.status.is_empty() {
        content = content.push(p.accent(app.status.clone(), 12));
    }
    p.page(content.padding(p.sp(22.0) as u16))
}
