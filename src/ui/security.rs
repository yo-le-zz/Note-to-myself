use iced::widget::{column, pick_list, row, text_input, Space};
use iced::{Alignment, Element, Length};
use ntm_core::crypto::{self, Cipher, KdfPreset};
use ntm_core::settings::Screen;
use std::fmt;

use super::style::{pal, section, settings_page, Pal};
use crate::app::{App, KeyfileTarget, Message, SecField};

#[derive(Clone, Copy, PartialEq, Eq)]
struct Minutes(u32);

impl fmt::Display for Minutes {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.0 == 0 {
            write!(f, "Off")
        } else {
            write!(f, "{} min", self.0)
        }
    }
}

fn secret_input<'a>(
    app: &App,
    p: Pal,
    key: &str,
    value: &str,
    field: SecField,
) -> Element<'a, Message> {
    text_input(&app.t(key), value)
        .secure(true)
        .on_input(move |v| Message::SecField(field, v))
        .padding(p.sp(10.0) as u16)
        .size(14)
        .style(p.input_style())
        .width(Length::Fill)
        .into()
}

fn strength_line<'a>(app: &App, p: Pal, password: &str) -> Element<'a, Message> {
    let level = crypto::password_strength(password);
    p.muted(
        format!(
            "{}: {}",
            app.t("strength"),
            app.t(&format!("strength_{level}"))
        ),
        12,
    )
    .into()
}

fn keyfile_row<'a>(app: &App, p: Pal) -> Element<'a, Message> {
    let name = app
        .form
        .keyfile
        .as_ref()
        .map(|(n, _)| n.clone())
        .unwrap_or_else(|| app.t("no_keyfile"));
    let mut line = row![
        p.muted(format!("{}: {}", app.t("keyfile"), name), 13),
        p.button(
            p.label(app.t("pick_keyfile"), 13),
            [6, 12],
            Some(Message::PickKeyfile(KeyfileTarget::Security))
        ),
    ]
    .spacing(10)
    .align_y(Alignment::Center);
    if app.form.keyfile.is_some() {
        line = line.push(p.button(
            p.label("×", 13),
            [6, 10],
            Some(Message::ClearKeyfile(KeyfileTarget::Security)),
        ));
    }
    line.into()
}

fn algorithm_pickers<'a>(app: &App, p: Pal) -> Element<'a, Message> {
    column![
        p.label(app.t("cipher"), 13),
        pick_list(Cipher::ALL, Some(app.form.cipher), Message::SecCipher).width(Length::Fill),
        p.label(app.t("kdf"), 13),
        pick_list(KdfPreset::ALL, Some(app.form.preset), Message::SecKdf).width(Length::Fill),
    ]
    .spacing(6)
    .into()
}

fn new_password_block<'a>(app: &App, p: Pal) -> Element<'a, Message> {
    column![
        secret_input(
            app,
            p,
            "new_password",
            &app.form.password,
            SecField::Password
        ),
        secret_input(
            app,
            p,
            "confirm_password",
            &app.form.confirm,
            SecField::Confirm
        ),
        row![
            strength_line(app, p, &app.form.password),
            Space::with_width(Length::Fill),
            p.button(
                p.label(app.t("generate"), 13),
                [6, 12],
                Some(Message::GeneratePassword)
            ),
        ]
        .align_y(Alignment::Center),
        keyfile_row(app, p),
        algorithm_pickers(app, p),
    ]
    .spacing(8)
    .into()
}

pub fn view(app: &App) -> Element<'_, Message> {
    let p = pal(app);
    let enabled = app.settings.security.enabled;
    let idle = (!app.busy).then_some(());

    let mut content = column![p.muted(app.t("security_intro"), 13)].spacing(p.sp(14.0));
    content = content.push(p.accent(app.t(if enabled { "vault_on" } else { "vault_off" }), 15));

    if !enabled {
        content = content.push(new_password_block(app, p));
        content = content.push(p.button(
            p.label(
                if app.busy {
                    app.t("busy")
                } else {
                    app.t("enable_encryption")
                },
                14,
            ),
            [8, 16],
            idle.map(|_| Message::EnableVault),
        ));
    } else {
        let options = [0u32, 1, 5, 10, 15, 30, 60].map(Minutes);
        content = content.push(
            row![
                p.label(app.t("auto_lock"), 14),
                pick_list(
                    options,
                    Some(Minutes(app.settings.security.auto_lock_minutes)),
                    |m| Message::AutoLock(m.0)
                ),
                Space::with_width(Length::Fill),
                p.button(
                    p.label(app.t("lock_now"), 13),
                    [6, 12],
                    Some(Message::LockNow)
                ),
            ]
            .spacing(10)
            .align_y(Alignment::Center),
        );
        content = content.push(section(&p, app.t("change_password")));
        content = content.push(secret_input(
            app,
            p,
            "current_password",
            &app.form.current,
            SecField::Current,
        ));
        content = content.push(new_password_block(app, p));
        content = content.push(p.button(
            p.label(
                if app.busy {
                    app.t("busy")
                } else {
                    app.t("change_password")
                },
                14,
            ),
            [8, 16],
            idle.map(|_| Message::ChangeVault),
        ));
        content = content.push(section(&p, app.t("disable_encryption")));
        content = content.push(p.button(
            p.label(app.t("disable_encryption"), 14),
            [8, 16],
            idle.map(|_| Message::DisableVault),
        ));
    }

    let can_backup = !app.settings.security.enabled || app.vault.is_some();
    content = content.push(section(&p, app.t("backup")));
    content = content.push(p.muted(app.t("backup_hint"), 12));
    content = content.push(secret_input(
        app,
        p,
        "backup_password",
        &app.form.backup,
        SecField::Backup,
    ));
    content = content.push(
        row![
            p.button(
                p.label(app.t("export_backup"), 14),
                [8, 16],
                can_backup.then_some(Message::ExportBackup)
            ),
            p.button(
                p.label(app.t("import_backup"), 14),
                [8, 16],
                can_backup.then_some(Message::ImportBackup)
            ),
        ]
        .spacing(10),
    );
    settings_page(
        app,
        &p,
        app.t("security"),
        content,
        Message::OpenScreen(Screen::Settings),
    )
}
