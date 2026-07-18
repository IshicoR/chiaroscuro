use iced::{
    Element,
    Length::Fill,
    mouse,
    widget::{column, container, mouse_area, row, text},
};

use super::style;
use chiaro_theme::typography;

pub fn chart_card<'a, Message: Clone + 'a>(
    title: &'a str,
    chart: impl Into<Element<'a, Message>>,
    header_action: impl Into<Element<'a, Message>>,
    on_title_double_click: Message,
    highlighted: bool,
    lift: f32,
) -> Element<'a, Message> {
    container(
        column![
            row![
                mouse_area(
                    container(text(title).size(22).font(typography::SANS_SEMIBOLD)).width(Fill),
                )
                .on_double_click(on_title_double_click)
                .interaction(mouse::Interaction::Pointer),
                header_action.into(),
            ]
            .align_y(iced::Alignment::Center),
            chart.into(),
        ]
        .spacing(8),
    )
    .padding(18)
    .width(Fill)
    .height(Fill)
    .style(move |theme| style::card(theme, highlighted, lift))
    .into()
}
