use iced::widget::{column, row, text_input};
use iced::{Alignment, Element, Length};
use lucide_icons::iced::icon_lock;

use super::style::pal;
use crate::app::{App, KeyfileTarget, Message};

pub fn view(app: &App) -> Element<'_, Message> {
    let p = pal(app);
    let placeholder = app.t("password");
    let password = text_input(&placeholder, &app.unlock_password)
        .secure(true)
        .on_input(Message::UnlockPasswordChanged)
        .on_submit(Message::Unlock)
        .padding(p.sp(12.0) as u16)
        .size(16)
        .style(p.input_style())
        .width(Length::Fixed(320.0));

    let mut content = column![
        icon_lock().size(40).color(p.accent),
        p.label(app.t("locked_title"), 22),
        password,
    ]
    .spacing(p.sp(14.0))
    .align_x(Alignment::Center);

    if app.settings.security.uses_keyfile {
        let name = app
            .unlock_keyfile
            .as_ref()
            .map(|(n, _)| n.clone())
            .unwrap_or_else(|| app.t("no_keyfile"));
        content = content.push(
            row![
                p.muted(name, 13),
                p.button(
                    p.label(app.t("pick_keyfile"), 13),
                    [6, 12],
                    Some(Message::PickKeyfile(KeyfileTarget::Unlock))
                ),
            ]
            .spacing(10)
            .align_y(Alignment::Center),
        );
    }
    let label = if app.busy {
        app.t("busy")
    } else {
        app.t("unlock")
    };
    content = content.push(p.button(
        p.label(label, 15),
        [10, 24],
        (!app.busy).then_some(Message::Unlock),
    ));
    if !app.unlock_error.is_empty() {
        content = content.push(p.accent(app.unlock_error.clone(), 13));
    }
    p.page(
        iced::widget::container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill)
            .center_y(Length::Fill),
    )
}
