use iced::widget::{button, column, container, row, text};
use iced::{Alignment, Element, Length, Theme};
use lucide_icons::iced::{
    icon_eye, icon_lock, icon_moon, icon_settings, icon_square_pen, icon_sun,
};
use ntm_core::plugin::host::Zone;

use super::style::{pal, Pal};
use crate::app::{App, Message, PluginCall};

fn tile<'a>(
    p: Pal,
    icon: Element<'a, Message>,
    title: String,
    subtitle: String,
    message: Message,
) -> Element<'a, Message> {
    let content = column![icon, p.label(title, 16), p.muted(subtitle, 11)]
        .spacing(7)
        .align_x(Alignment::Center);
    button(
        container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill)
            .center_y(Length::Fill),
    )
    .width(150)
    .height(112)
    .style(p.button_style(true))
    .on_press(message)
    .into()
}

fn round_button<'a>(p: Pal, icon: Element<'a, Message>, message: Message) -> Element<'a, Message> {
    button(
        container(icon)
            .center_x(Length::Fill)
            .center_y(Length::Fill),
    )
    .width(40)
    .height(40)
    .style(move |theme: &Theme, status| {
        let mut style = p.button_style(false)(theme, status);
        style.border.radius = 20.0.into();
        style
    })
    .on_press(message)
    .into()
}

pub fn plugin_buttons<'a>(app: &'a App, p: Pal, zone: Zone) -> Element<'a, Message> {
    let mut buttons = row![].spacing(8).align_y(Alignment::Center);
    for b in app.plugin_buttons.iter().filter(|b| b.zone == zone) {
        buttons = buttons.push(p.button(
            p.label(b.label.clone(), 13),
            [6, 12],
            Some(Message::PluginCall(PluginCall {
                plugin_id: b.plugin_id.clone(),
                callback: b.callback.clone(),
                note_uid: None,
            })),
        ));
    }
    buttons.into()
}

pub fn view(app: &App) -> Element<'_, Message> {
    let p = pal(app);
    let dark = app.palette().dark;
    let accent = p.accent;

    let new_tile = tile(
        p,
        icon_square_pen().size(25).color(accent).into(),
        app.t("new"),
        app.t("write_something"),
        Message::NewNote,
    );
    let see_tile = tile(
        p,
        icon_eye().size(25).color(accent).into(),
        app.t("see"),
        app.t("your_notes"),
        Message::SeeNotes,
    );

    let theme_icon = if dark { icon_moon() } else { icon_sun() };
    let mut top = row![
        round_button(
            p,
            theme_icon.size(20).color(accent).into(),
            Message::ToggleTheme(!dark)
        ),
        round_button(
            p,
            icon_settings().size(20).color(accent).into(),
            Message::OpenSettings
        ),
    ]
    .spacing(8)
    .align_y(Alignment::Center);
    if app.settings.security.enabled {
        top = top.push(round_button(
            p,
            icon_lock().size(20).color(accent).into(),
            Message::LockNow,
        ));
    }

    let mut center = column![row![new_tile, see_tile]
        .spacing(14)
        .align_y(Alignment::Center)]
    .spacing(18)
    .align_x(Alignment::Center);
    center = center.push(plugin_buttons(app, p, Zone::Home));
    if !app.status.is_empty() {
        center = center.push(
            text(app.status.clone())
                .size(12)
                .color(p.accent)
                .font(p.font),
        );
    }

    let top_left = container(top)
        .padding(12)
        .width(Length::Fill)
        .align_x(Alignment::Start);
    let center = container(center)
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill);
    p.page(iced::widget::stack![center, top_left])
}
