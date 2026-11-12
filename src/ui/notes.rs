use iced::widget::{column, container, row, scrollable, Space};
use iced::{Alignment, Element, Length};
use lucide_icons::iced::{
    icon_chevron_down, icon_chevron_up, icon_copy, icon_ellipsis_vertical, icon_pencil,
    icon_trash_2,
};
use ntm_core::notes::Note;
use ntm_core::plugin::host::Zone;

use super::home::plugin_buttons;
use super::style::{pal, Pal};
use crate::app::{App, Message, PluginCall};

fn matches_search(note: &Note, query: &str) -> bool {
    query.is_empty()
        || note.title.to_lowercase().contains(query)
        || note.body.to_lowercase().contains(query)
}

fn note_card<'a>(app: &'a App, note: &'a Note, p: Pal) -> Element<'a, Message> {
    let expanded = app.expanded.contains(&note.uid);
    let title = if note.unreadable {
        app.t("unreadable_note")
    } else if note.title.trim().is_empty() {
        app.t("untitled")
    } else {
        note.title.clone()
    };

    let chevron = if expanded {
        icon_chevron_up()
    } else {
        icon_chevron_down()
    };
    let header = row![
        p.accent(format!("{} {}", app.t("note_n"), note.number), 12)
            .width(Length::Fixed(78.0)),
        p.label(title, 16).width(Length::Fill),
        p.button(
            chevron.size(14).color(p.fg),
            [5, 8],
            Some(Message::ToggleExpand(note.uid.clone()))
        ),
        p.button(
            icon_ellipsis_vertical().size(14).color(p.fg),
            [5, 8],
            Some(Message::ToggleMenu(note.uid.clone()))
        ),
    ]
    .spacing(8)
    .align_y(Alignment::Center);

    let mut card = column![header].spacing(p.sp(10.0)).width(Length::Fill);

    if app.menu_open.as_deref() == Some(note.uid.as_str()) {
        let item = |icon: Element<'a, Message>, label: String, message: Message| {
            p.button(
                row![icon, p.label(label, 13)]
                    .spacing(8)
                    .align_y(Alignment::Center),
                [6, 12],
                Some(message),
            )
        };
        let mut menu = row![].spacing(8);
        if !note.unreadable {
            menu = menu
                .push(item(
                    icon_pencil().size(14).color(p.fg).into(),
                    app.t("edit"),
                    Message::EditNote(note.uid.clone()),
                ))
                .push(item(
                    icon_copy().size(14).color(p.fg).into(),
                    app.t("copy"),
                    Message::CopyNote(note.uid.clone()),
                ));
            for action in &app.note_actions {
                menu = menu.push(p.button(
                    p.label(action.label.clone(), 13),
                    [6, 12],
                    Some(Message::PluginCall(PluginCall {
                        plugin_id: action.plugin_id.clone(),
                        callback: action.callback.clone(),
                        note_uid: Some(note.uid.clone()),
                    })),
                ));
            }
        }
        menu = menu.push(item(
            icon_trash_2().size(14).color(p.accent).into(),
            app.t("delete"),
            Message::AskDelete(note.uid.clone()),
        ));
        card = card.push(
            scrollable(menu).direction(scrollable::Direction::Horizontal(
                scrollable::Scrollbar::new(),
            )),
        );
    }

    if app.pending_delete.as_deref() == Some(note.uid.as_str()) {
        card = card.push(
            row![
                p.accent(app.t("delete_confirm"), 13),
                Space::with_width(Length::Fill),
                p.button(
                    p.label(app.t("yes"), 13),
                    [6, 14],
                    Some(Message::ConfirmDelete(note.uid.clone()))
                ),
                p.button(
                    p.label(app.t("no"), 13),
                    [6, 14],
                    Some(Message::CancelDelete)
                ),
            ]
            .spacing(8)
            .align_y(Alignment::Center),
        );
    }

    if expanded {
        if !note.body.trim().is_empty() {
            card = card.push(p.label(note.body.clone(), 14));
        }
        if app.settings.show_timestamps {
            card = card.push(p.muted(format!("{}: {}", app.t("updated"), note.updated_at), 10));
        }
    }

    container(card)
        .padding(if app.settings.compact_mode {
            p.sp(8.0)
        } else {
            p.sp(12.0)
        } as u16)
        .width(Length::Fill)
        .style(p.panel_style())
        .into()
}

pub fn view(app: &App) -> Element<'_, Message> {
    let p = pal(app);
    let search_placeholder = app.t("search");
    let search = p
        .input(&search_placeholder, &app.search)
        .on_input(Message::SearchChanged);

    let mut content = column![p.label(app.t("your_notes"), 25)]
        .spacing(p.sp(12.0))
        .width(Length::Fill);
    let query = app.search.trim().to_lowercase();
    if app.notes.is_empty() {
        content = content.push(
            container(p.muted(app.t("no_notes"), 14))
                .padding(20)
                .width(Length::Fill),
        );
    } else {
        // Une liste de plusieurs milliers de cartes ralentirait l'interface : on en affiche 200 au plus.
        const MAX_SHOWN: usize = 200;
        let mut shown = 0;
        let mut hidden = false;
        for note in app.notes.iter().filter(|n| matches_search(n, &query)) {
            if shown == MAX_SHOWN {
                hidden = true;
                break;
            }
            content = content.push(note_card(app, note, p));
            shown += 1;
        }
        if hidden {
            content = content.push(p.muted(app.t("more_results"), 12));
        }
        if shown == 0 {
            content = content.push(
                container(p.muted(app.t("no_results"), 14))
                    .padding(20)
                    .width(Length::Fill),
            );
        }
    }

    let mut footer = row![
        p.text_button(app.t("back"), Message::BackHome),
        plugin_buttons(app, p, Zone::Notes)
    ]
    .spacing(12)
    .align_y(Alignment::Center);
    if !app.status.is_empty() {
        footer = footer.push(p.accent(app.status.clone(), 12));
    }

    let mut layout = column![]
        .spacing(p.sp(12.0))
        .padding(p.sp(18.0) as u16)
        .width(Length::Fill)
        .height(Length::Fill);
    if !app.notes.is_empty() {
        layout = layout.push(search);
    }
    p.page(
        layout
            .push(scrollable(content).height(Length::Fill))
            .push(footer),
    )
}
