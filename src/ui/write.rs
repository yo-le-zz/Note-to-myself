use iced::widget::{button, column, container, row, text, text_editor, text_input};
use iced::{Alignment, Border, Color, Element, Length, Theme};
use lucide_icons::iced::{icon_arrow_left, icon_save};

use crate::app::{App, Message};

const ORANGE: Color = Color::from_rgb(0.85, 0.32, 0.16);
const BLACK: Color = Color::from_rgb(0.04, 0.04, 0.04);
const WHITE: Color = Color::WHITE;
const MUTED: Color = Color::from_rgb(0.55, 0.55, 0.55);

pub fn view(app: &App) -> Element<'_, Message> {
    let dark = app.settings.dark_mode;
    let fg = if dark { WHITE } else { BLACK };
    let bg = if dark { BLACK } else { WHITE };
    let field = if dark {
        Color::from_rgb(0.10, 0.10, 0.10)
    } else {
        Color::from_rgb(0.97, 0.97, 0.97)
    };

    let title_text = app.t("write");
    let subtitle_text = app.t("subtitle");
    let title_placeholder = app.t("title_placeholder");
    let back_text = app.t("back");
    let save_text = app.t("save");

    let title_input = text_input(&title_placeholder, &app.title)
        .on_input(Message::TitleChanged)
        .padding(12)
        .size(app.settings.font_size.clamp(12, 24))
        .width(Length::Fill)
        .style(move |_theme: &Theme, _status| text_input::Style {
            background: field.into(),
            border: Border {
                color: ORANGE,
                width: 1.0,
                radius: 9.0.into(),
            },
            icon: fg,
            placeholder: MUTED,
            value: fg,
            selection: ORANGE,
        });

    let editor_placeholder = app.t("body_placeholder");
    let editor = text_editor(&app.body)
        .placeholder(editor_placeholder)
        .on_action(Message::BodyEdited)
        .padding(12)
        .size(app.settings.font_size.clamp(12, 24))
        .height(Length::Fill)
        .style(move |_theme: &Theme, _status| text_editor::Style {
            background: field.into(),
            border: Border {
                color: ORANGE,
                width: 1.0,
                radius: 9.0.into(),
            },
            icon: fg,
            placeholder: MUTED,
            value: fg,
            selection: ORANGE,
        });

    let back_button = button(
        row![
            icon_arrow_left().size(16).color(fg),
            text(back_text).size(13).color(fg),
        ]
        .spacing(8)
        .align_y(Alignment::Center),
    )
    .padding([8, 12])
    .style(move |_theme: &Theme, status| {
        let hovered = matches!(status, button::Status::Hovered);
        button::Style {
            background: Some(bg.into()),
            text_color: fg,
            border: Border {
                color: ORANGE,
                width: if hovered { 2.0 } else { 1.0 },
                radius: 9.0.into(),
            },
            ..Default::default()
        }
    })
    .on_press(Message::BackFromWrite);

    // Pas de container en hauteur Fill dans le bouton : sa hauteur suit son contenu.
    let save_button = button(
        row![
            icon_save().size(16).color(fg),
            text(save_text).size(13).color(fg),
        ]
        .spacing(8)
        .align_y(Alignment::Center),
    )
    .padding([8, 14])
    .style(move |_theme: &Theme, status| {
        let hovered = matches!(status, button::Status::Hovered);
        button::Style {
            background: Some(field.into()),
            text_color: fg,
            border: Border {
                color: ORANGE,
                width: if hovered { 2.0 } else { 1.0 },
                radius: 9.0.into(),
            },
            ..Default::default()
        }
    })
    .on_press(Message::SaveNote);

    let body_text = app.body.text();
    let char_count = body_text.strip_suffix('\n').unwrap_or(&body_text).chars().count();
    let actions = row![
        back_button,
        save_button,
        iced::widget::Space::with_width(Length::Fill),
        text(format!("{} {}", char_count, app.t("characters")))
            .size(12)
            .color(MUTED),
    ]
    .spacing(10)
    .align_y(Alignment::Center);

    let mut content = column![
        text(title_text).size(25).color(fg),
        text(subtitle_text).size(13).color(MUTED),
        title_input,
        editor,
        actions,
    ]
    .spacing(if app.settings.compact_mode { 8 } else { 12 })
    .align_x(Alignment::Start)
    .width(Length::Fill)
    .height(Length::Fill);

    if !app.status.is_empty() {
        content = content.push(text(&app.status).size(12).color(ORANGE));
    }

    container(content.padding(22))
        .width(Length::Fill)
        .height(Length::Fill)
        .style(move |_theme| container::Style {
            background: Some(bg.into()),
            text_color: Some(fg),
            ..Default::default()
        })
        .into()
}
