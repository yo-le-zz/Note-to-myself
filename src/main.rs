mod app;
mod i18n;
mod ui;

use app::{App, Message, Screen};
use iced::{keyboard, Element, Subscription, Task};
use lucide_icons::lucide_font_bytes;

fn main() -> iced::Result {
    iced::application("Note to Myself", update, view)
        .theme(|app: &App| {
            if app.settings.dark_mode {
                iced::Theme::Dark
            } else {
                iced::Theme::Light
            }
        })
        .subscription(subscription)
        .window(window_settings())

        .font(lucide_font_bytes())
        .centered()
        .run_with(|| (App::new(), Task::none()))
}

fn window_settings() -> iced::window::Settings {
    // Icône 64x64 RGBA brute, générée depuis packaging/icons/note-to-myself-64.png.
    let icon = iced::window::icon::from_rgba(
        include_bytes!("../packaging/icons/window-64.rgba").to_vec(),
        64,
        64,
    )
    .ok();
    iced::window::Settings {
        size: iced::Size::new(900.0, 680.0),
        icon,
        platform_specific: iced::window::settings::PlatformSpecific {
            // Doit correspondre à StartupWMClass du fichier .desktop.
            application_id: "note-to-myself".to_string(),
            ..Default::default()
        },
        ..Default::default()
    }
}

fn update(app: &mut App, message: Message) -> Task<Message> {
    match message {
        Message::NewNote => app.start_new_note(),
        Message::SeeNotes => {
            // Les notes sont déjà chargées au démarrage et après chaque sauvegarde.
            // Éviter une requête SQLite à chaque retour sur cet écran.
            app.status.clear();
            app.screen = Screen::See;
        }
        Message::BackHome => {
            app.status.clear();
            app.screen = Screen::Home;
        }
        Message::OpenSettings => {
            app.status.clear();
            app.screen = Screen::Settings;
        }
        Message::ToggleTheme(value) => {
            app.settings.dark_mode = value;
            app.update_settings();
        }
        Message::TitleChanged(value) => {
            app.title = value;
            app.status.clear();
        }
        Message::BodyEdited(action) => {
            app.body.perform(action);
            app.status.clear();
        }
        Message::SaveNote => {
            if let Some(request) = app.prepare_save() {
                return Task::perform(
                    async move {
                        std::thread::spawn(move || App::save_request(request))
                            .join()
                            .unwrap_or_else(|_| Err("save worker panicked".to_string()))
                    },
                    Message::SaveFinished,
                );
            }
        }
        Message::SaveFinished(result) => app.finish_save(result),
        Message::SaveSettings => {
            app.update_settings();
            app.status = app.t("settings_saved");
        }
        Message::LanguageChanged(language) => {
            app.settings.language = language;
            app.update_settings();
        }
        Message::ToggleTimestamps(value) => {
            app.settings.show_timestamps = value;
            app.update_settings();
        }
        Message::ToggleCompact(value) => {
            app.settings.compact_mode = value;
            app.update_settings();
        }
        Message::ToggleConfirmDelete(value) => {
            app.settings.confirm_delete = value;
            app.update_settings();
        }
        Message::FontSizeChanged(value) => {
            // La taille reste dans une plage sûre, même si un message inattendu arrive.
            // Pas d'écriture disque pendant le glissement : voir FontSizeReleased.
            app.set_font_size(value);
        }
        Message::FontSizeReleased => app.update_settings(),
        Message::BackFromWrite => {
            app.status.clear();
            app.screen = if app.is_editing() { Screen::See } else { Screen::Home };
        }
        Message::SearchChanged(value) => app.search = value,
        Message::ToggleExpand(id) => {
            if !app.expanded.remove(&id) {
                app.expanded.insert(id);
            }
        }
        Message::ToggleMenu(id) => {
            app.pending_delete = None;
            app.menu_open = if app.menu_open == Some(id) { None } else { Some(id) };
        }
        Message::EditNote(id) => app.open_note(id),
        Message::CopyNote(id) => {
            app.menu_open = None;
            if let Some(text) = app.note_text(id) {
                app.status = app.t("copied");
                return iced::clipboard::write(text);
            }
        }
        Message::AskDelete(id) => {
            app.menu_open = None;
            if app.settings.confirm_delete {
                app.pending_delete = Some(id);
            } else {
                return delete_task(app, id);
            }
        }
        Message::ConfirmDelete(id) => return delete_task(app, id),
        Message::CancelDelete => app.pending_delete = None,
        Message::DeleteFinished(result) => app.finish_delete(result),
    }

    Task::none()
}

fn delete_task(app: &mut App, id: i64) -> Task<Message> {
    app.pending_delete = None;
    let path = app.database_path();
    Task::perform(
        async move {
            std::thread::spawn(move || App::delete_request(path, id))
                .join()
                .unwrap_or_else(|_| Err("delete worker panicked".to_string()))
        },
        Message::DeleteFinished,
    )
}

fn subscription(app: &App) -> Subscription<Message> {
    if app.screen == Screen::Write {
        // Ctrl+S / Cmd+S enregistre la note.
        keyboard::on_key_press(|key, modifiers| {
            if modifiers.command() && matches!(key.as_ref(), keyboard::Key::Character("s")) {
                Some(Message::SaveNote)
            } else {
                None
            }
        })
    } else {
        Subscription::none()
    }
}

fn view(app: &App) -> Element<'_, Message> {
    match app.screen {
        Screen::Home => ui::home::view(app),
        Screen::Write => ui::write::view(app),
        Screen::See => ui::notes::view(app),
        Screen::Settings => ui::config::view(app),
    }
}
