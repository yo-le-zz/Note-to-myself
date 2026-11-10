use iced::widget::{column, container, row, Space};
use iced::{Alignment, Element, Length};
use ntm_core::plugin::Risk;
use ntm_core::settings::Screen;

use super::style::{pal, settings_page, Pal};
use crate::app::{App, Message, PluginEntry};

fn risk_key(risk: Risk) -> &'static str {
    match risk {
        Risk::Low => "risk_low",
        Risk::Medium => "risk_medium",
        Risk::High => "risk_high",
    }
}

fn plugin_card<'a>(app: &App, p: Pal, entry: &PluginEntry) -> Element<'a, Message> {
    let m = &entry.plugin.manifest;
    let id = m.id.clone();
    let kind = app.t(if entry.plugin.builtin {
        "builtin"
    } else {
        "community"
    });

    let toggle = if entry.enabled {
        p.button(
            p.label(app.t("disable"), 12),
            [5, 10],
            Some(Message::PluginDisable(id.clone())),
        )
    } else {
        p.button(
            p.label(app.t("enable"), 12),
            [5, 10],
            Some(Message::PluginEnable(id.clone())),
        )
    };
    let mut header = row![
        p.label(format!("{}  v{}", m.name, m.version), 15),
        p.muted(format!("({kind})"), 12),
        Space::with_width(Length::Fill),
        p.accent(app.t(risk_key(entry.plugin.analysis.risk())), 12),
        toggle,
    ]
    .spacing(8)
    .align_y(Alignment::Center);
    if !entry.plugin.builtin {
        header = header.push(p.button(
            p.label(app.t("remove_plugin"), 12),
            [5, 10],
            Some(Message::PluginRemove(id)),
        ));
    }

    let mut card = column![header].spacing(p.sp(6.0));
    if !m.description.is_empty() {
        card = card.push(p.muted(m.description.clone(), 12));
    }
    if !m.author.is_empty() {
        card = card.push(p.muted(format!("by {}", m.author), 11));
    }
    if entry.changed {
        card = card.push(p.accent(app.t("plugin_changed"), 12));
    }
    let mut permissions = column![p.label(app.t("permissions"), 12)].spacing(2);
    if entry.plugin.analysis.permissions.is_empty() {
        permissions = permissions.push(p.muted("—", 11));
    }
    for permission in &entry.plugin.analysis.permissions {
        permissions = permissions.push(p.muted(
            format!("• {} — {}", permission.id(), permission.describe()),
            11,
        ));
    }
    card = card.push(permissions);
    for warning in &entry.plugin.analysis.warnings {
        card = card.push(p.accent(format!("⚠ {warning}"), 11));
    }
    container(card)
        .padding(p.sp(10.0) as u16)
        .width(Length::Fill)
        .style(p.panel_style())
        .into()
}

pub fn view(app: &App) -> Element<'_, Message> {
    let p = pal(app);
    let enabled = app.plugins.iter().filter(|e| e.enabled).count();
    let mut content = column![
        p.muted(app.t("plugins_intro"), 13),
        row![
            p.button(
                p.label(app.t("enable_builtin"), 13),
                [8, 14],
                Some(Message::PluginEnableBuiltin)
            ),
            p.button(
                p.label(app.t("install_plugin"), 13),
                [8, 14],
                Some(Message::PluginInstall)
            ),
            p.muted(
                format!("{enabled}/{} {}", app.plugins.len(), app.t("enabled_count")),
                12
            ),
        ]
        .spacing(10)
        .align_y(Alignment::Center),
        p.muted(
            format!(
                "{}: {}",
                app.t("plugins_folder"),
                app.paths.plugins.display()
            ),
            11
        ),
    ];
    for entry in &app.plugins {
        content = content.push(plugin_card(app, p, entry));
    }
    settings_page(
        app,
        &p,
        app.t("plugins"),
        content,
        Message::OpenScreen(Screen::Settings),
    )
}
