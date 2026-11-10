//! Styles partagés : tout vient de la palette du thème actif (couleurs, arrondi, bordures, espacement, police).

use crate::app::{App, Message};
use iced::widget::{
    button, column, container, row, scrollable, text, text_editor, text_input, Column, Space, Text,
};
use iced::{Alignment, Border, Color, Element, Font, Length, Theme};
use ntm_core::theme::{Palette, Rgb};

#[derive(Clone, Copy)]
pub struct Pal {
    pub fg: Color,
    pub bg: Color,
    pub panel: Color,
    pub muted: Color,
    pub accent: Color,
    pub radius: f32,
    pub border: f32,
    pub spacing: f32,
    pub font: Font,
    pub editor_font: Font,
}

pub fn color(rgb: Rgb) -> Color {
    let [r, g, b] = rgb.rgb_f32();
    Color::from_rgb(r, g, b)
}

impl Pal {
    pub fn new(palette: &Palette) -> Pal {
        let mono = |on: bool| if on { Font::MONOSPACE } else { Font::DEFAULT };
        Pal {
            fg: color(palette.fg),
            bg: color(palette.bg),
            panel: color(palette.panel),
            muted: color(palette.muted),
            accent: color(palette.accent),
            radius: palette.radius,
            border: palette.border_width,
            spacing: palette.spacing,
            font: mono(palette.ui_monospace),
            editor_font: mono(palette.editor_monospace || palette.ui_monospace),
        }
    }

    /// Thème iced équivalent, pour les widgets standards (cases à cocher, curseurs, listes…).
    pub fn iced_theme(palette: &Palette) -> Theme {
        Theme::custom(
            "Note to Myself".to_string(),
            iced::theme::Palette {
                background: color(palette.bg),
                text: color(palette.fg),
                primary: color(palette.accent),
                success: color(palette.accent),
                danger: color(palette.accent),
            },
        )
    }

    pub fn sp(&self, base: f32) -> f32 {
        (base * self.spacing).round()
    }

    pub fn label<'a>(&self, content: impl text::IntoFragment<'a>, size: u16) -> Text<'a> {
        text(content).size(size).color(self.fg).font(self.font)
    }

    pub fn muted<'a>(&self, content: impl text::IntoFragment<'a>, size: u16) -> Text<'a> {
        text(content).size(size).color(self.muted).font(self.font)
    }

    pub fn accent<'a>(&self, content: impl text::IntoFragment<'a>, size: u16) -> Text<'a> {
        text(content).size(size).color(self.accent).font(self.font)
    }

    pub fn button_style(
        &self,
        filled: bool,
    ) -> impl Fn(&Theme, button::Status) -> button::Style + 'static {
        let p = *self;
        move |_theme, status| {
            let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
            let disabled = matches!(status, button::Status::Disabled);
            button::Style {
                background: Some(if filled { p.panel } else { p.bg }.into()),
                text_color: if disabled { p.muted } else { p.fg },
                border: Border {
                    color: if disabled { p.muted } else { p.accent },
                    width: if hovered { p.border + 1.0 } else { p.border },
                    radius: p.radius.into(),
                },
                ..Default::default()
            }
        }
    }

    /// Bouton à contour d'accent ; sans `message`, il est grisé.
    pub fn button<'a>(
        &self,
        content: impl Into<Element<'a, Message>>,
        padding: [u16; 2],
        message: Option<Message>,
    ) -> Element<'a, Message> {
        let b = button(content)
            .padding([
                self.sp(padding[0] as f32) as u16,
                self.sp(padding[1] as f32) as u16,
            ])
            .style(self.button_style(false));
        match message {
            Some(m) => b.on_press(m).into(),
            None => b.into(),
        }
    }

    pub fn text_button<'a>(
        &self,
        label: impl text::IntoFragment<'a>,
        message: Message,
    ) -> Element<'a, Message> {
        self.button(self.label(label, 14), [8, 16], Some(message))
    }

    pub fn input_style(
        &self,
    ) -> impl Fn(&Theme, text_input::Status) -> text_input::Style + 'static {
        let p = *self;
        move |_theme, _status| text_input::Style {
            background: p.panel.into(),
            border: Border {
                color: p.accent,
                width: p.border,
                radius: p.radius.into(),
            },
            icon: p.fg,
            placeholder: p.muted,
            value: p.fg,
            selection: p.accent,
        }
    }

    pub fn input<'a>(&self, placeholder: &str, value: &str) -> text_input::TextInput<'a, Message> {
        text_input(placeholder, value)
            .padding(self.sp(10.0) as u16)
            .size(14)
            .font(self.font)
            .style(self.input_style())
    }

    pub fn editor_style(
        &self,
    ) -> impl Fn(&Theme, text_editor::Status) -> text_editor::Style + 'static {
        let p = *self;
        move |_theme, _status| text_editor::Style {
            background: p.panel.into(),
            border: Border {
                color: p.accent,
                width: p.border,
                radius: p.radius.into(),
            },
            icon: p.fg,
            placeholder: p.muted,
            value: p.fg,
            selection: p.accent,
        }
    }

    pub fn panel_style(&self) -> impl Fn(&Theme) -> container::Style + 'static {
        let p = *self;
        move |_theme| container::Style {
            background: Some(p.panel.into()),
            text_color: Some(p.fg),
            border: Border {
                color: p.accent,
                width: p.border,
                radius: p.radius.into(),
            },
            ..Default::default()
        }
    }

    pub fn page_style(&self) -> impl Fn(&Theme) -> container::Style + 'static {
        let p = *self;
        move |_theme| container::Style {
            background: Some(p.bg.into()),
            text_color: Some(p.fg),
            ..Default::default()
        }
    }

    pub fn page<'a>(&self, content: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
        container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(self.page_style())
            .into()
    }

    /// Petit carré de couleur (aperçu d'un thème).
    pub fn swatch<'a>(&self, rgb: Rgb) -> Element<'a, Message> {
        let fill = color(rgb);
        let outline = self.muted;
        container(Space::new(18, 18))
            .style(move |_theme: &Theme| container::Style {
                background: Some(fill.into()),
                border: Border {
                    color: outline,
                    width: 1.0,
                    radius: 4.0.into(),
                },
                ..Default::default()
            })
            .into()
    }
}

pub fn pal(app: &App) -> Pal {
    Pal::new(&app.palette())
}

/// Écran de réglages : titre, contenu défilant dans un panneau, puis retour et message d'état.
pub fn settings_page<'a>(
    app: &'a App,
    p: &Pal,
    title: String,
    content: Column<'a, Message>,
    back: Message,
) -> Element<'a, Message> {
    let p = *p;
    let mut footer = row![p.text_button(app.t("back"), back)]
        .spacing(10)
        .align_y(Alignment::Center);
    if !app.status.is_empty() {
        footer = footer.push(p.accent(app.status.clone(), 12));
    }
    let panel =
        container(column![p.label(title, 25), content.spacing(p.sp(14.0))].spacing(p.sp(16.0)))
            .padding(p.sp(22.0) as u16)
            .width(Length::Fill)
            .style(move |_theme: &Theme| container::Style {
                background: Some(p.panel.into()),
                text_color: Some(p.fg),
                ..Default::default()
            });
    let layout = column![scrollable(panel).height(Length::Fill), footer]
        .spacing(p.sp(12.0))
        .padding(p.sp(16.0) as u16)
        .width(Length::Fill)
        .height(Length::Fill);
    p.page(layout)
}

/// Titre de section.
pub fn section<'a>(p: &Pal, title: String) -> Element<'a, Message> {
    p.accent(title, 16).into()
}
