use iced::{
    Color, Element,
    Length::Fill,
    widget::{column, container, text},
};

use super::style;

pub fn chart_card<'a, Message: 'a>(
    title: &'a str,
    subtitle: impl Into<String>,
    chart: impl Into<Element<'a, Message>>,
) -> Element<'a, Message> {
    container(
        column![
            column![text(title).size(18), text(subtitle.into()).size(12)].spacing(4),
            chart.into(),
        ]
        .spacing(16),
    )
    .padding(18)
    .width(Fill)
    .height(Fill)
    .style(style::card)
    .into()
}

pub fn metric_card<'a, Message: 'a>(
    label: &'a str,
    value: impl Into<String>,
    accent: Option<Color>,
) -> Element<'a, Message> {
    let value = match accent {
        Some(color) => text(value.into()).size(21).color(color),
        None => text(value.into()).size(21),
    };

    container(column![text(label).size(12), value].spacing(8))
        .padding([14, 16])
        .width(Fill)
        .style(style::card)
        .into()
}
