use iced::widget::{checkbox, column, container, row, scrollable, slider, Space};
use iced::{Alignment, Element, Length};
use ntm_core::settings::Screen;

use super::style::{pal, section, Pal};
use crate::app::{App, Message, ThemeFilter};

fn filter_label(app: &App, filter: ThemeFilter) -> String {
    app.t(match filter {
        ThemeFilter::All => "filter_all",
        ThemeFilter::Dark => "filter_dark",
        ThemeFilter::Light => "filter_light",
    })
}

fn slider_row<'a>(
    p: Pal,
    label: String,
    value: String,
    control: Element<'a, Message>,
) -> Element<'a, Message> {
    column![
        row![
            p.label(label, 13),
            Space::with_width(Length::Fill),
            p.accent(value, 13)
        ]
        .align_y(Alignment::Center),
        control,
    ]
    .spacing(4)
    .into()
}

fn customization<'a>(app: &'a App, p: Pal) -> Element<'a, Message> {
    let t = &app.settings.theme;
    let accent_placeholder = app.t("accent");
    let accent = row![
        p.input(&accent_placeholder, &app.accent_input)
            .on_input(Message::AccentInput)
            .width(Length::Fill),
        p.button(
            p.label(app.t("reset"), 13),
            [8, 12],
            Some(Message::AccentInput(String::new()))
        ),
    ]
    .spacing(8)
    .align_y(Alignment::Center);

    let name_placeholder = app.t("theme_name");
    let save = row![
        p.input(&name_placeholder, &app.custom_name)
            .on_input(Message::CustomNameChanged)
            .width(Length::Fill),
        p.button(
            p.label(app.t("save_theme"), 13),
            [8, 12],
            (!app.custom_name.trim().is_empty()).then_some(Message::SaveCustomTheme)
        ),
    ]
    .spacing(8)
    .align_y(Alignment::Center);

    column![
        section(&p, app.t("customize")),
        accent,
        slider_row(
            p,
            app.t("corner_radius"),
            format!("{} px", t.radius),
            slider(0..=28u16, t.radius, Message::RadiusChanged)
                .on_release(Message::ThemeSliderReleased)
                .into()
        ),
        slider_row(
            p,
            app.t("border_width"),
            format!("{} px", t.border_width),
            slider(0..=4u16, t.border_width, Message::BorderChanged)
                .on_release(Message::ThemeSliderReleased)
                .into()
        ),
        slider_row(
            p,
            app.t("spacing"),
            format!("{} %", t.spacing),
            slider(70..=140u16, t.spacing, Message::SpacingChanged)
                .on_release(Message::ThemeSliderReleased)
                .into()
        ),
        slider_row(
            p,
            app.t("panel_contrast"),
            format!("{:+}", t.panel_contrast),
            slider(0..=20u8, (t.panel_contrast + 10) as u8, |v| {
                Message::PanelContrastChanged(v as i8 - 10)
            })
            .on_release(Message::ThemeSliderReleased)
            .into()
        ),
        checkbox(app.t("editor_mono"), t.editor_monospace).on_toggle(Message::ToggleEditorMono),
        checkbox(app.t("ui_mono"), t.ui_monospace).on_toggle(Message::ToggleUiMono),
        save,
    ]
    .spacing(p.sp(10.0))
    .into()
}

pub fn view(app: &App) -> Element<'_, Message> {
    let p = pal(app);
    let query = app.theme_search.trim().to_lowercase();
    let shown: Vec<_> = app
        .themes
        .iter()
        .filter(|t| match app.theme_filter {
            ThemeFilter::All => true,
            ThemeFilter::Dark => t.dark,
            ThemeFilter::Light => !t.dark,
        })
        .filter(|t| query.is_empty() || t.name.to_lowercase().contains(&query))
        .collect();

    let filters = ThemeFilter::ALL.iter().fold(row![].spacing(8), |r, &f| {
        let label = p.label(filter_label(app, f), 13);
        let message = (app.theme_filter != f).then_some(Message::ThemeFilterChanged(f));
        r.push(p.button(label, [6, 12], message))
    });
    let search_placeholder = app.t("search_themes");
    let controls = row![
        filters,
        p.input(&search_placeholder, &app.theme_search)
            .on_input(Message::ThemeSearch)
            .width(Length::Fill),
    ]
    .spacing(10)
    .align_y(Alignment::Center);

    let current = app.settings.theme.id.clone();
    let mut list = column![].spacing(p.sp(6.0));
    for theme in shown.iter() {
        let is_current = theme.id == current;
        let is_preview = app.preview.as_deref() == Some(theme.id.as_str());
        let name = if is_current {
            format!("{} ✓", theme.name)
        } else {
            theme.name.clone()
        };
        let row_widget = row![
            p.swatch(theme.bg),
            p.swatch(theme.fg),
            p.swatch(theme.accent),
            p.label(name, 14).width(Length::Fill),
            p.button(
                p.label(app.t("preview"), 12),
                [5, 10],
                (!is_preview).then(|| Message::PreviewTheme(theme.id.clone()))
            ),
            p.button(
                p.label(app.t("apply"), 12),
                [5, 10],
                (!is_current).then(|| Message::ApplyTheme(theme.id.clone()))
            ),
        ]
        .spacing(8)
        .align_y(Alignment::Center);
        list = list.push(
            container(row_widget)
                .padding(p.sp(6.0) as u16)
                .width(Length::Fill)
                .style(p.panel_style()),
        );
    }

    let header = column![
        p.label(app.t("themes_title"), 25),
        p.muted(
            format!("{}: {}", app.t("current_theme"), app.current_theme_name()),
            13
        ),
        p.muted(format!("{} {}", shown.len(), app.t("theme_count")), 12),
        controls,
    ]
    .spacing(p.sp(8.0));

    let mut footer = row![p.text_button(app.t("back"), Message::OpenScreen(Screen::Settings))]
        .spacing(10)
        .align_y(Alignment::Center);
    if !app.status.is_empty() {
        footer = footer.push(p.accent(app.status.clone(), 12));
    }
    let body = column![customization(app, p), list]
        .spacing(p.sp(18.0))
        .width(Length::Fill);
    let layout = column![header, scrollable(body).height(Length::Fill), footer]
        .spacing(p.sp(12.0))
        .padding(p.sp(16.0) as u16)
        .width(Length::Fill)
        .height(Length::Fill);
    p.page(layout)
}

/// Bandeau affiché au-dessus de n'importe quel écran pendant l'aperçu d'un thème.
pub fn preview_banner(app: &App) -> Option<Element<'_, Message>> {
    let id = app.preview.as_ref()?;
    let name = app
        .themes
        .iter()
        .find(|t| &t.id == id)
        .map(|t| t.name.clone())?;
    let p = pal(app);
    let content = row![
        p.label(format!("{} : {}", app.t("previewing"), name), 14)
            .width(Length::Fill),
        p.button(
            p.label(app.t("apply"), 13),
            [6, 12],
            Some(Message::ApplyTheme(id.clone()))
        ),
        p.button(
            p.label(app.t("cancel"), 13),
            [6, 12],
            Some(Message::CancelPreview)
        ),
    ]
    .spacing(10)
    .align_y(Alignment::Center);
    Some(
        container(content)
            .padding(p.sp(10.0) as u16)
            .width(Length::Fill)
            .style(p.panel_style())
            .into(),
    )
}
