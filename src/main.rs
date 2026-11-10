// Pas de console derrière la fenêtre sous Windows (sauf en développement).
#![cfg_attr(all(not(debug_assertions), windows), windows_subsystem = "windows")]

mod app;
mod ui;

use app::{App, Message};
use iced::{keyboard, time, Element, Subscription, Task};
use lucide_icons::lucide_font_bytes;
use ntm_core::settings::Screen;
use std::time::Duration;

fn main() -> iced::Result {
    iced::application("Note to Myself", App::update, view)
        .theme(|app: &App| ui::style::Pal::iced_theme(&app.palette()))
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

fn subscription(app: &App) -> Subscription<Message> {
    let mut subscriptions = Vec::new();
    if app.screen == Screen::Write && !app.locked {
        // Ctrl+S / Cmd+S enregistre la note.
        subscriptions.push(keyboard::on_key_press(|key, modifiers| {
            if modifiers.command() && matches!(key.as_ref(), keyboard::Key::Character("s")) {
                Some(Message::SaveNote)
            } else {
                None
            }
        }));
    }
    if app.settings.security.enabled
        && app.vault.is_some()
        && app.settings.security.auto_lock_minutes > 0
    {
        subscriptions.push(time::every(Duration::from_secs(5)).map(|_| Message::Tick));
    }
    if app.settings.sync.enabled {
        subscriptions.push(time::every(Duration::from_secs(2)).map(|_| Message::Poll));
    }
    if let Some(interval) = app.sync_interval() {
        subscriptions.push(time::every(interval).map(|_| Message::SyncTick));
    }
    Subscription::batch(subscriptions)
}

fn view(app: &App) -> Element<'_, Message> {
    let screen = if app.locked {
        ui::lock::view(app)
    } else {
        match app.screen {
            Screen::Home => ui::home::view(app),
            Screen::Write => ui::write::view(app),
            Screen::See => ui::notes::view(app),
            Screen::Settings => ui::config::view(app),
            Screen::Themes => ui::themes::view(app),
            Screen::Security => ui::security::view(app),
            Screen::Sync => ui::sync::view(app),
            Screen::Plugins => ui::plugins::view(app),
        }
    };
    match ui::themes::preview_banner(app) {
        Some(banner) => iced::widget::column![banner, screen].into(),
        None => screen,
    }
}
