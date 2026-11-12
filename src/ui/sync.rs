use iced::widget::{checkbox, column, container, pick_list, row, text_input, Space};
use iced::{Alignment, Element, Length};
use ntm_core::settings::{Screen, SyncMode};
use std::fmt;

use super::style::{pal, section, settings_page, Pal};
use crate::app::{App, Message};

#[derive(Clone, Copy, PartialEq, Eq)]
struct Seconds(u32);

impl fmt::Display for Seconds {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0 {
            s if s < 60 => write!(f, "{s} s"),
            s if s % 3600 == 0 => write!(f, "{} h", s / 3600),
            s if s % 60 == 0 => write!(f, "{} min", s / 60),
            s => write!(f, "{s} s"),
        }
    }
}

fn peer_row<'a>(app: &App, p: Pal, index: usize, address: &str) -> Element<'a, Message> {
    row![
        p.label(address.to_string(), 13).width(Length::Fill),
        p.button(
            p.label(app.t("pull_config"), 12),
            [5, 10],
            (!app.sync_busy).then_some(Message::PullConfig(index))
        ),
        p.button(
            p.label(app.t("remove"), 12),
            [5, 10],
            Some(Message::RemovePeer(index))
        ),
    ]
    .spacing(8)
    .align_y(Alignment::Center)
    .into()
}

pub fn view(app: &App) -> Element<'_, Message> {
    let p = pal(app);
    let s = &app.settings.sync;

    let secret_placeholder = app.t("secret");
    let secret = row![
        text_input(&secret_placeholder, &s.secret)
            .secure(!app.show_secret)
            .on_input(Message::SyncSecret)
            .padding(p.sp(10.0) as u16)
            .size(14)
            .style(p.input_style())
            .width(Length::Fill),
        p.button(
            p.label(app.t(if app.show_secret { "hide" } else { "show" }), 12),
            [6, 10],
            Some(Message::SyncToggleSecret)
        ),
        p.button(
            p.label(app.t("generate"), 12),
            [6, 10],
            Some(Message::SyncGenerateSecret)
        ),
    ]
    .spacing(8)
    .align_y(Alignment::Center);

    let name_placeholder = app.t("this_pc");
    let port_placeholder = app.t("port");
    let intervals = [10u32, 30, 60, 300, 900, 3600].map(Seconds);

    let mut peers = column![].spacing(p.sp(8.0));
    if s.peers.is_empty() {
        peers = peers.push(p.muted(app.t("no_peers"), 13));
    }
    for (index, address) in s.peers.iter().enumerate() {
        peers = peers.push(
            container(peer_row(app, p, index, address))
                .padding(p.sp(6.0) as u16)
                .style(p.panel_style()),
        );
    }
    let peer_placeholder = "100.x.y.z:47474";
    let add_peer = row![
        p.input(peer_placeholder, &app.peer_input)
            .on_input(Message::PeerInput)
            .on_submit(Message::AddPeer)
            .width(Length::Fill),
        p.button(
            p.label(app.t("add_peer"), 13),
            [8, 14],
            Some(Message::AddPeer)
        ),
    ]
    .spacing(8);

    let mut actions = row![
        p.button(
            p.label(
                if app.sync_busy {
                    app.t("syncing")
                } else {
                    app.t("sync_now")
                },
                14
            ),
            [8, 16],
            (!app.sync_busy && app.sync_ready()).then_some(Message::SyncNow)
        ),
        p.button(
            p.label(app.t("push_config"), 14),
            [8, 16],
            (!app.sync_busy && app.sync_ready()).then_some(Message::PushConfig)
        ),
    ]
    .spacing(10);
    if s.secret.chars().count() < 8 {
        actions = actions.push(p.accent(app.t("sync_needs_secret"), 12));
    }

    let mut log = column![].spacing(2);
    for line in app.sync_log.iter().rev().take(12) {
        log = log.push(p.muted(line.clone(), 11));
    }

    let content = column![
        p.muted(app.t("sync_intro"), 13),
        checkbox(app.t("sync_enabled"), s.enabled).on_toggle(Message::SyncToggle),
        text_input(&name_placeholder, &s.device_name)
            .on_input(Message::SyncName)
            .padding(p.sp(10.0) as u16)
            .size(14)
            .style(p.input_style()),
        checkbox(app.t("primary_pc"), s.is_primary).on_toggle(Message::SyncPrimary),
        section(&p, app.t("secret")),
        secret,
        p.muted(app.t("secret_hint"), 12),
        checkbox(app.t("listen"), s.listen).on_toggle(Message::SyncListen),
        row![
            p.label(app.t("port"), 13),
            p.input(&port_placeholder, &app.port_input)
                .on_input(Message::SyncPort)
                .width(100)
        ]
        .spacing(10)
        .align_y(Alignment::Center),
        checkbox(app.t("trusted_only"), s.trusted_networks_only).on_toggle(Message::SyncTrusted),
        checkbox(app.t("encrypt_transport"), s.encrypt_transport).on_toggle(Message::SyncEncrypt),
        section(&p, app.t("peers")),
        peers,
        add_peer,
        section(&p, app.t("sync_mode")),
        pick_list(SyncMode::ALL, Some(s.mode), Message::SyncModeChanged).width(Length::Fill),
        row![
            p.label(app.t("interval"), 13),
            pick_list(intervals, Some(Seconds(s.interval_secs)), |v| {
                Message::SyncInterval(v.0)
            }),
            Space::with_width(Length::Fill),
        ]
        .spacing(10)
        .align_y(Alignment::Center),
        checkbox(app.t("sync_on_save"), s.sync_on_save).on_toggle(Message::SyncOnSave),
        actions,
        section(&p, app.t("sync_log")),
        log,
    ];
    settings_page(
        app,
        &p,
        app.t("sync"),
        content,
        Message::OpenScreen(Screen::Settings),
    )
}
