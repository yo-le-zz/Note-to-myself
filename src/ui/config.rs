use iced::widget::{checkbox, column, pick_list, row, slider, Space};
use iced::{Alignment, Element, Length};
use ntm_core::settings::{Language, Screen};

use super::style::{pal, section, settings_page};
use crate::app::{App, Message};

pub fn view(app: &App) -> Element<'_, Message> {
    let p = pal(app);
    let dark = app.palette().dark;

    let language_row = column![
        p.label(app.t("language"), 14),
        pick_list(
            Language::ALL,
            Some(app.settings.language),
            Message::LanguageChanged
        )
        .width(Length::Fill),
    ]
    .spacing(8);

    let font_size = app.settings.font_size.clamp(12, 24);
    let font_label = row![
        p.label(app.t("font_size"), 14),
        Space::with_width(Length::Fill),
        p.accent(format!("{font_size} px"), 13),
    ]
    .align_y(Alignment::Center);

    let nav = |key: &str, screen: Screen| p.text_button(app.t(key), Message::OpenScreen(screen));
    let navigation = row![
        nav("themes", Screen::Themes),
        nav("security", Screen::Security),
        nav("sync", Screen::Sync),
        nav("plugins", Screen::Plugins),
    ]
    .spacing(10);

    let content = column![
        language_row,
        section(&p, app.t("appearance")),
        checkbox(app.t("dark_mode"), dark).on_toggle(Message::ToggleTheme),
        checkbox(app.t("show_timestamps"), app.settings.show_timestamps)
            .on_toggle(Message::ToggleTimestamps),
        checkbox(app.t("compact_mode"), app.settings.compact_mode)
            .on_toggle(Message::ToggleCompact),
        checkbox(app.t("confirm_delete"), app.settings.confirm_delete)
            .on_toggle(Message::ToggleConfirmDelete),
        font_label,
        slider(12..=24, font_size, Message::FontSizeChanged).on_release(Message::FontSizeReleased),
        navigation,
        p.text_button(app.t("save"), Message::SaveSettings),
    ];
    settings_page(app, &p, app.t("settings"), content, Message::BackHome)
}
