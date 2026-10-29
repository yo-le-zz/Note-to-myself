use iced::widget::{button, checkbox, column, container, pick_list, row, scrollable, slider, text};
use iced::{Alignment, Border, Color, Element, Length, Theme};

use crate::app::{App, Language, Message};

const ORANGE: Color = Color::from_rgb(0.85, 0.32, 0.16);
const BLACK: Color = Color::from_rgb(0.04, 0.04, 0.04);
const WHITE: Color = Color::WHITE;

pub fn view(app: &App) -> Element<'_, Message> {
    let dark = app.settings.dark_mode;
    let fg = if dark { WHITE } else { BLACK };
    let bg = if dark { BLACK } else { WHITE };
    let panel = if dark {
        Color::from_rgb(0.10, 0.10, 0.10)
    } else {
        Color::from_rgb(0.97, 0.97, 0.97)
    };

    let language_picker = pick_list(
        Language::ALL,
        Some(app.settings.language),
        Message::LanguageChanged,
    )
    .width(Length::Fill);

    let language_row = column![
        text(app.t("language")).size(14).color(fg),
        language_picker,
    ]
    .spacing(8);

    let appearance = text(app.t("appearance")).size(16).color(ORANGE);

    let timestamps = checkbox(app.t("show_timestamps"), app.settings.show_timestamps)
        .on_toggle(Message::ToggleTimestamps);
    let compact = checkbox(app.t("compact_mode"), app.settings.compact_mode)
        .on_toggle(Message::ToggleCompact);
    let confirm_delete = checkbox(app.t("confirm_delete"), app.settings.confirm_delete)
        .on_toggle(Message::ToggleConfirmDelete);
    let dark_mode = checkbox(app.t("dark_mode"), dark).on_toggle(Message::ToggleTheme);

    let font_size_label = row![
        text(app.t("font_size")).size(14).color(fg),
        iced::widget::Space::with_width(Length::Fill),
        text(format!("{} px", app.settings.font_size.clamp(12, 24)))
            .size(13)
            .color(ORANGE),
    ]
    .align_y(Alignment::Center);

    let font_size = slider(
        12..=24,
        app.settings.font_size.clamp(12, 24),
        Message::FontSizeChanged,
    )
    .on_release(Message::FontSizeReleased);

    let content = column![
        text(app.t("settings")).size(25).color(fg),
        language_row,
        appearance,
        dark_mode,
        timestamps,
        compact,
        confirm_delete,
        font_size_label,
        font_size,
    ]
    .spacing(16)
    .width(Length::Fill);

    let save_button = button(text(app.t("save")).size(14).color(fg))
        .padding([8, 16])
        .style(move |_theme: &Theme, status| {
            let hovered = matches!(status, button::Status::Hovered);
            button::Style {
                background: Some(panel.into()),
                text_color: fg,
                border: Border {
                    color: ORANGE,
                    width: if hovered { 2.0 } else { 1.0 },
                    radius: 9.0.into(),
                },
                ..Default::default()
            }
        })
        .on_press(Message::SaveSettings);

    let back_button = button(text(app.t("back")).size(14).color(fg))
        .padding([8, 16])
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
        .on_press(Message::BackHome);

    let mut footer = row![back_button, save_button].spacing(10).align_y(Alignment::Center);
    if !app.status.is_empty() {
        footer = footer.push(text(&app.status).size(12).color(ORANGE));
    }

    let layout = column![
        scrollable(
            container(content)
                .padding(22)
                .width(Length::Fill)
                .style(move |_theme| container::Style {
                    background: Some(panel.into()),
                    text_color: Some(fg),
                    ..Default::default()
                })
        )
        .height(Length::Fill),
        footer,
    ]
    .spacing(12)
    .padding(16)
    .width(Length::Fill)
    .height(Length::Fill);

    container(layout)
        .width(Length::Fill)
        .height(Length::Fill)
        .style(move |_theme| container::Style {
            background: Some(bg.into()),
            text_color: Some(fg),
            ..Default::default()
        })
        .into()
}
