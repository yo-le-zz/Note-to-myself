use iced::widget::{button, column, container, row, text};
use iced::{Alignment, Border, Color, Element, Length, Theme};
use lucide_icons::iced::{icon_eye, icon_moon, icon_settings, icon_square_pen, icon_sun};

use crate::app::{App, Message};

const ORANGE: Color = Color::from_rgb(0.85, 0.32, 0.16);
const BLACK: Color = Color::from_rgb(0.04, 0.04, 0.04);
const WHITE: Color = Color::WHITE;
const MUTED: Color = Color::from_rgb(0.55, 0.55, 0.55);

pub fn view(app: &App) -> Element<'_, Message> {
    let dark = app.settings.dark_mode;
    let fg = if dark { WHITE } else { BLACK };
    let bg = if dark { BLACK } else { WHITE };
    let card = if dark {
        Color::from_rgb(0.10, 0.10, 0.10)
    } else {
        Color::from_rgb(0.97, 0.97, 0.97)
    };

    let new_content = column![
        icon_square_pen().size(25).color(ORANGE),
        text(app.t("new")).size(16).color(fg),
        text(app.t("write_something")).size(11).color(MUTED),
    ]
    .spacing(7)
    .align_x(Alignment::Center);

    let new_button = button(
        container(new_content)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill)
            .center_y(Length::Fill),
    )
    .width(150)
    .height(112)
    .style(move |_theme: &Theme, status| {
        let hovered = matches!(status, button::Status::Hovered);
        button::Style {
            background: Some(card.into()),
            text_color: fg,
            border: Border {
                color: ORANGE,
                width: if hovered { 2.0 } else { 1.0 },
                radius: 14.0.into(),
            },
            ..Default::default()
        }
    })
    .on_press(Message::NewNote);

    let see_content = column![
        icon_eye().size(25).color(ORANGE),
        text(app.t("see")).size(16).color(fg),
        text(app.t("your_notes")).size(11).color(MUTED),
    ]
    .spacing(7)
    .align_x(Alignment::Center);

    let see_button = button(
        container(see_content)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill)
            .center_y(Length::Fill),
    )
    .width(150)
    .height(112)
    .style(move |_theme: &Theme, status| {
        let hovered = matches!(status, button::Status::Hovered);
        button::Style {
            background: Some(card.into()),
            text_color: fg,
            border: Border {
                color: ORANGE,
                width: if hovered { 2.0 } else { 1.0 },
                radius: 14.0.into(),
            },
            ..Default::default()
        }
    })
    .on_press(Message::SeeNotes);

    let theme_icon = if dark { icon_moon() } else { icon_sun() };

    let theme_button = button(
        container(theme_icon.size(20).color(ORANGE))
            .center_x(Length::Fill)
            .center_y(Length::Fill),
    )
    .width(40)
    .height(40)
    .style(move |_theme: &Theme, status| {
        let hovered = matches!(status, button::Status::Hovered);
        button::Style {
            background: Some(bg.into()),
            text_color: ORANGE,
            border: Border {
                color: ORANGE,
                width: if hovered { 2.0 } else { 1.0 },
                radius: 20.0.into(),
            },
            ..Default::default()
        }
    })
    .on_press(Message::ToggleTheme(!dark));

    let config_button = button(
        container(icon_settings().size(20).color(ORANGE))
            .center_x(Length::Fill)
            .center_y(Length::Fill),
    )
    .width(40)
    .height(40)
    .style(move |_theme: &Theme, status| {
        let hovered = matches!(status, button::Status::Hovered);
        button::Style {
            background: Some(bg.into()),
            text_color: ORANGE,
            border: Border {
                color: ORANGE,
                width: if hovered { 2.0 } else { 1.0 },
                radius: 20.0.into(),
            },
            ..Default::default()
        }
    })
    .on_press(Message::OpenSettings);

    let top_buttons = row![theme_button, config_button]
        .spacing(8)
        .align_y(Alignment::Center);

    let top_left = container(top_buttons)
        .padding(12)
        .width(Length::Fill)
        .align_x(Alignment::Start);

    let center = container(
        row![new_button, see_button]
            .spacing(14)
            .align_y(Alignment::Center),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .center_x(Length::Fill)
    .center_y(Length::Fill);

    container(iced::widget::stack![center, top_left])
        .width(Length::Fill)
        .height(Length::Fill)
        .style(move |_theme| container::Style {
            background: Some(bg.into()),
            text_color: Some(fg),
            ..Default::default()
        })
        .into()
}
