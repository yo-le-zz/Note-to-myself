use iced::widget::{
    button, column, container, row, scrollable, text, text_input, Space,
};
use iced::{Alignment, Border, Color, Element, Length, Theme};
use lucide_icons::iced::{
    icon_chevron_down, icon_chevron_up, icon_copy, icon_ellipsis_vertical, icon_pencil,
    icon_trash_2,
};

use crate::app::{App, Message, Note};

const ORANGE: Color = Color::from_rgb(0.85, 0.32, 0.16);
const BLACK: Color = Color::from_rgb(0.04, 0.04, 0.04);
const WHITE: Color = Color::WHITE;
const MUTED: Color = Color::from_rgb(0.55, 0.55, 0.55);

struct Palette {
    fg: Color,
    bg: Color,
    panel: Color,
}

fn outlined_button<'a>(
    content: impl Into<Element<'a, Message>>,
    colors: &Palette,
    padding: [u16; 2],
    message: Message,
) -> Element<'a, Message> {
    let (fg, bg) = (colors.fg, colors.bg);
    button(content)
        .padding(padding)
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
        .on_press(message)
        .into()
}

fn matches_search(note: &Note, query: &str) -> bool {
    query.is_empty()
        || note.title.to_lowercase().contains(query)
        || note.body.to_lowercase().contains(query)
}

fn note_card<'a>(app: &'a App, note: &'a Note, colors: &Palette) -> Element<'a, Message> {
    let (fg, panel) = (colors.fg, colors.panel);
    let expanded = app.expanded.contains(&note.id);
    let title = if note.title.trim().is_empty() {
        app.t("untitled")
    } else {
        note.title.clone()
    };

    // Ligne principale : « Note x » à gauche, titre, bouton dérouler, bouton menu.
    let chevron = if expanded {
        icon_chevron_up().size(14).color(fg)
    } else {
        icon_chevron_down().size(14).color(fg)
    };
    let header = row![
        text(format!("{} {}", app.t("note_n"), note.number))
            .size(12)
            .color(ORANGE)
            .width(Length::Fixed(78.0)),
        text(title).size(16).color(fg).width(Length::Fill),
        outlined_button(chevron, colors, [5, 8], Message::ToggleExpand(note.id)),
        outlined_button(
            icon_ellipsis_vertical().size(14).color(fg),
            colors,
            [5, 8],
            Message::ToggleMenu(note.id)
        ),
    ]
    .spacing(8)
    .align_y(Alignment::Center);

    let mut card_content = column![header].spacing(10).width(Length::Fill);

    if app.menu_open == Some(note.id) {
        let item = |icon: Element<'a, Message>, label: String, message: Message| {
            outlined_button(
                row![icon, text(label).size(13).color(fg)]
                    .spacing(8)
                    .align_y(Alignment::Center),
                colors,
                [6, 12],
                message,
            )
        };
        card_content = card_content.push(
            row![
                item(
                    icon_pencil().size(14).color(fg).into(),
                    app.t("edit"),
                    Message::EditNote(note.id)
                ),
                item(
                    icon_copy().size(14).color(fg).into(),
                    app.t("copy"),
                    Message::CopyNote(note.id)
                ),
                item(
                    icon_trash_2().size(14).color(ORANGE).into(),
                    app.t("delete"),
                    Message::AskDelete(note.id)
                ),
            ]
            .spacing(8),
        );
    }

    if app.pending_delete == Some(note.id) {
        card_content = card_content.push(
            row![
                text(app.t("delete_confirm")).size(13).color(ORANGE),
                Space::with_width(Length::Fill),
                outlined_button(
                    text(app.t("yes")).size(13).color(fg),
                    colors,
                    [6, 14],
                    Message::ConfirmDelete(note.id)
                ),
                outlined_button(
                    text(app.t("no")).size(13).color(fg),
                    colors,
                    [6, 14],
                    Message::CancelDelete
                ),
            ]
            .spacing(8)
            .align_y(Alignment::Center),
        );
    }

    if expanded {
        if !note.body.trim().is_empty() {
            card_content = card_content.push(text(&note.body).size(14).color(fg));
        }
        if app.settings.show_timestamps {
            card_content = card_content.push(
                text(format!("{}: {}", app.t("updated"), note.updated_at))
                    .size(10)
                    .color(MUTED),
            );
        }
    }

    container(card_content)
        .padding(if app.settings.compact_mode { 8 } else { 12 })
        .width(Length::Fill)
        .style(move |_theme| container::Style {
            background: Some(panel.into()),
            text_color: Some(fg),
            border: Border {
                color: ORANGE,
                width: 1.0,
                radius: 10.0.into(),
            },
            ..Default::default()
        })
        .into()
}

pub fn view(app: &App) -> Element<'_, Message> {
    let dark = app.settings.dark_mode;
    let colors = Palette {
        fg: if dark { WHITE } else { BLACK },
        bg: if dark { BLACK } else { WHITE },
        panel: if dark {
            Color::from_rgb(0.10, 0.10, 0.10)
        } else {
            Color::from_rgb(0.97, 0.97, 0.97)
        },
    };
    let (fg, bg, panel) = (colors.fg, colors.bg, colors.panel);

    let search_placeholder = app.t("search");
    let search = text_input(&search_placeholder, &app.search)
        .on_input(Message::SearchChanged)
        .padding(10)
        .size(14)
        .style(move |_theme: &Theme, _status| text_input::Style {
            background: panel.into(),
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

    let mut content = column![text(app.t("your_notes")).size(25).color(fg)]
        .spacing(12)
        .width(Length::Fill);

    let query = app.search.trim().to_lowercase();
    if app.notes.is_empty() {
        content = content.push(
            container(text(app.t("no_notes")).size(14).color(MUTED))
                .padding(20)
                .width(Length::Fill),
        );
    } else {
        let mut shown = 0;
        for note in app.notes.iter().filter(|n| matches_search(n, &query)) {
            content = content.push(note_card(app, note, &colors));
            shown += 1;
        }
        if shown == 0 {
            content = content.push(
                container(text(app.t("no_results")).size(14).color(MUTED))
                    .padding(20)
                    .width(Length::Fill),
            );
        }
    }

    let mut footer = row![outlined_button(
        text(app.t("back")).size(14).color(fg),
        &colors,
        [8, 16],
        Message::BackHome
    )]
    .spacing(12)
    .align_y(Alignment::Center);
    if !app.status.is_empty() {
        footer = footer.push(text(&app.status).size(12).color(ORANGE));
    }

    let mut layout = column![].spacing(12).padding(18).width(Length::Fill).height(Length::Fill);
    if !app.notes.is_empty() {
        layout = layout.push(search);
    }
    let layout = layout
        .push(scrollable(content).height(Length::Fill))
        .push(footer);

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
