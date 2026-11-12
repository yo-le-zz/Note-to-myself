//! Dessine le panneau d'un plugin à partir de ses composants déclaratifs.

use iced::widget::{
    checkbox, column, container, horizontal_rule, pick_list, progress_bar, row, scrollable, slider,
    Space,
};
use iced::{Alignment, Border, Color, Element, Length, Theme};
use ntm_core::plugin::panel::{PanelValue, Widget};
use ntm_core::theme::parse_hex;

use super::style::{color, pal, Pal};
use crate::app::{App, Message};

fn hex_color(text: &str) -> Color {
    parse_hex(text)
        .map(color)
        .unwrap_or(Color::from_rgb(0.5, 0.5, 0.5))
}

fn labeled<'a>(
    p: Pal,
    label: &str,
    value: String,
    control: Element<'a, Message>,
) -> Element<'a, Message> {
    column![
        row![
            p.label(label.to_string(), 13),
            Space::with_width(Length::Fill),
            p.accent(value, 13)
        ]
        .align_y(Alignment::Center),
        control,
    ]
    .spacing(4)
    .into()
}

/// Maquette de l'application avec les couleurs proposées : texte, bouton et bandeau d'accent.
fn sample<'a>(p: Pal, bg: &str, fg: &str, accent: &str) -> Element<'a, Message> {
    let (bg, fg, accent) = (hex_color(bg), hex_color(fg), hex_color(accent));
    let radius = p.radius;
    let border = p.border.max(1.0);
    let button = container(iced::widget::text("Button").size(13).color(fg))
        .padding([6, 14])
        .style(move |_theme: &Theme| container::Style {
            border: Border {
                color: accent,
                width: border,
                radius: radius.into(),
            },
            ..Default::default()
        });
    let stripe =
        container(Space::new(Length::Fill, 4)).style(move |_theme: &Theme| container::Style {
            background: Some(accent.into()),
            ..Default::default()
        });
    let content = column![
        iced::widget::text("Note to Myself").size(20).color(fg),
        iced::widget::text("A thought, just for you. Aa Bb Cc 123")
            .size(13)
            .color(fg),
        stripe,
        button,
    ]
    .spacing(10);
    container(content)
        .padding(16)
        .width(Length::Fill)
        .style(move |_theme: &Theme| container::Style {
            background: Some(bg.into()),
            text_color: Some(fg),
            border: Border {
                color: accent,
                width: border,
                radius: radius.into(),
            },
            ..Default::default()
        })
        .into()
}

fn widget_view<'a>(app: &'a App, p: Pal, widget: &'a Widget) -> Element<'a, Message> {
    let current = |id: &str| app.panel_state.get(id);
    match widget {
        Widget::Heading(text) => p.accent(text.clone(), 16).into(),
        Widget::Text(text) => p.label(text.clone(), 13).into(),
        Widget::Separator => horizontal_rule(1).into(),
        Widget::Slider {
            id,
            label,
            min,
            max,
            value,
            ..
        } => {
            let shown = match current(id) {
                Some(PanelValue::Int(i)) => *i,
                _ => *value,
            }
            .clamp(*min, *max);
            let id = id.clone();
            let control = slider(*min as i32..=*max as i32, shown as i32, move |v| {
                Message::PanelChanged(id.clone(), PanelValue::Int(v as i64))
            });
            labeled(p, label, shown.to_string(), control.into())
        }
        Widget::Input {
            id, label, value, ..
        } => {
            let shown = match current(id) {
                Some(PanelValue::Text(t)) => t.clone(),
                _ => value.clone(),
            };
            let id = id.clone();
            column![
                p.label(label.clone(), 13),
                p.input("", &shown)
                    .on_input(move |v| Message::PanelChanged(id.clone(), PanelValue::Text(v)))
            ]
            .spacing(4)
            .into()
        }
        Widget::Toggle {
            id, label, value, ..
        } => {
            let shown = match current(id) {
                Some(PanelValue::Bool(b)) => *b,
                _ => *value,
            };
            let id = id.clone();
            checkbox(label.clone(), shown)
                .on_toggle(move |v| Message::PanelChanged(id.clone(), PanelValue::Bool(v)))
                .into()
        }
        Widget::Choice {
            id,
            label,
            options,
            value,
            ..
        } => {
            let shown = match current(id) {
                Some(PanelValue::Text(t)) if options.contains(t) => t.clone(),
                _ => value.clone(),
            };
            let id = id.clone();
            column![
                p.label(label.clone(), 13),
                pick_list(
                    options.clone(),
                    Some(shown),
                    move |v| Message::PanelChanged(id.clone(), PanelValue::Text(v))
                )
                .width(Length::Fill),
            ]
            .spacing(4)
            .into()
        }
        Widget::Color {
            id, label, value, ..
        } => {
            let shown = match current(id) {
                Some(PanelValue::Text(t)) => t.clone(),
                _ => value.clone(),
            };
            let id = id.clone();
            column![
                p.label(label.clone(), 13),
                row![
                    p.swatch_color(hex_color(&shown)),
                    p.input("#rrggbb", &shown)
                        .on_input(move |v| Message::PanelChanged(id.clone(), PanelValue::Text(v))),
                ]
                .spacing(8)
                .align_y(Alignment::Center),
            ]
            .spacing(4)
            .into()
        }
        Widget::Swatches(colors) => colors
            .iter()
            .fold(row![].spacing(8), |r, c| {
                r.push(p.swatch_color(hex_color(c)))
            })
            .into(),
        Widget::Preview { bg, fg, accent } => sample(p, bg, fg, accent),
        Widget::Progress { label, percent } => column![
            p.label(label.clone(), 13),
            progress_bar(0.0..=100.0, *percent as f32),
        ]
        .spacing(4)
        .into(),
        Widget::Button {
            label,
            callback,
            arg,
        } => p.button(
            p.label(label.clone(), 14),
            [8, 16],
            Some(Message::PanelButton(callback.clone(), arg.clone())),
        ),
    }
}

pub fn view(app: &App) -> Element<'_, Message> {
    let p = pal(app);
    let title = app
        .active_panel
        .and_then(|i| app.panels.get(i))
        .map(|panel| panel.title.clone())
        .unwrap_or_default();

    let mut body = column![].spacing(p.sp(12.0)).width(Length::Fill);
    for widget in &app.panel_widgets {
        body = body.push(widget_view(app, p, widget));
    }
    let panel = container(column![p.label(title, 25), body].spacing(p.sp(16.0)))
        .padding(p.sp(22.0) as u16)
        .width(Length::Fill);

    let mut footer = row![p.text_button(app.t("back"), Message::ClosePanel)]
        .spacing(10)
        .align_y(Alignment::Center);
    if !app.status.is_empty() {
        footer = footer.push(p.accent(app.status.clone(), 12));
    }
    let layout = column![scrollable(panel).height(Length::Fill), footer]
        .spacing(p.sp(12.0))
        .padding(p.sp(16.0) as u16)
        .width(Length::Fill)
        .height(Length::Fill);
    p.page(layout)
}
