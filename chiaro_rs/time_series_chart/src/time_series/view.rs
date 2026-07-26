use chiaro_i18n::{Text, tr};
use chiaro_widgets::context_menu as chart_context_menu;
use iced::{
    Background, Element, Length, Theme,
    alignment::{Horizontal, Vertical},
    keyboard,
    widget::{Column, Row, Space, button, container, mouse_area, text},
};
use iced_plot::{Color, PlotOverlay, PointId, Transform};

use crate::style;

use super::{
    TimeSeriesChart, TimeSeriesMessage,
    interaction::{ScrollAction, scroll_action},
};

const TOOLTIP_TEXT_SIZE: f32 = 12.0;
const TOOLTIP_PADDING: f32 = 5.0;
const TOOLTIP_OFFSET: f32 = 6.0;
const TOOLTIP_WIDTH: f32 = 240.0;
const TOOLTIP_ROW_SPACING: f32 = 3.0;
const TOOLTIP_ITEM_SPACING: f32 = 6.0;
const TOOLTIP_MARKER_WIDTH: f32 = 12.0;
const TOOLTIP_MARKER_HEIGHT: f32 = 3.0;
const CONTEXT_MENU_WIDTH: f32 = 168.0;

#[derive(Debug, Clone, PartialEq)]
pub(super) struct TooltipValue {
    pub(super) position: [f64; 2],
    pub(super) label: &'static str,
    pub(super) value: String,
    pub(super) color: Color,
}

impl TimeSeriesChart {
    pub fn view(
        &self,
        focus_x: Option<f64>,
        modifiers: keyboard::Modifiers,
    ) -> Element<'_, TimeSeriesMessage> {
        let focus_line = focus_x.map(|x| {
            let line: Element<'_, TimeSeriesMessage> = container(Space::new())
                .width(Length::Fixed(2.0))
                .height(Length::Fill)
                .style(|theme: &Theme| {
                    container::Style::default().background(Background::Color(
                        theme.extended_palette().primary.base.color,
                    ))
                })
                .into();

            if self.axis.live_mode {
                PlotOverlay::new(line, [self.x_axis_fraction(x), 0.5]).with_axes_transform()
            } else {
                PlotOverlay::new(line, [x, 0.5]).with_transform_y(Transform::axes())
            }
        });
        let tooltip_overlays = self.tooltip_overlays(focus_x);

        let plot = self.plot.view_with_shapes(
            focus_line.into_iter(),
            tooltip_overlays.into_iter(),
            TimeSeriesMessage::Plot,
        );

        let plot = mouse_area(plot)
            .on_press(TimeSeriesMessage::BeginCursorDrag)
            .on_release(TimeSeriesMessage::EndCursorDrag)
            .on_exit(TimeSeriesMessage::EndCursorDrag)
            .on_double_click(TimeSeriesMessage::ResetX);
        let plot = match scroll_action(modifiers) {
            Some(ScrollAction::PanX) => plot.on_scroll(TimeSeriesMessage::PanX),
            Some(ScrollAction::ZoomX) => plot.on_scroll(TimeSeriesMessage::ZoomX),
            None => plot,
        };

        let menu_label = if self.interaction.tooltips_visible {
            tr(Text::HideTooltips)
        } else {
            tr(Text::ShowTooltips)
        };
        let menu_item = container(text(menu_label).size(13))
            .width(Length::Fill)
            .align_x(Horizontal::Left);
        let menu = container(
            button(menu_item)
                .width(Length::Fill)
                .padding([6, 8])
                .style(style::context_menu_item)
                .on_press(TimeSeriesMessage::ToggleTooltips),
        )
        .width(Length::Fixed(CONTEXT_MENU_WIDTH))
        .padding(4)
        .style(style::context_menu);

        chart_context_menu(plot, menu)
            .open(self.interaction.context.is_some())
            .on_open(TimeSeriesMessage::OpenContextMenu)
            .on_close(TimeSeriesMessage::CloseContextMenu)
            .into()
    }

    pub(super) fn tooltip_focus_index(&self) -> Option<usize> {
        self.interaction
            .focus_index
            .filter(|_| self.interaction.tooltips_visible && self.interaction.context.is_none())
    }

    pub(super) fn tooltip_values(&self, point_index: usize) -> Vec<TooltipValue> {
        self.series
            .ids
            .iter()
            .zip(&self.series.labels)
            .zip(&self.series.colors)
            .filter_map(|((series_id, label), color)| {
                let position = self.plot.point_position(PointId {
                    series_id: *series_id,
                    point_index,
                })?;
                if !position[0].is_finite() || !position[1].is_finite() {
                    return None;
                }

                Some(TooltipValue {
                    position,
                    label,
                    value: (self.axis.value_formatter)(position[1]),
                    color: *color,
                })
            })
            .collect()
    }

    pub(super) fn tooltip_overlays(
        &self,
        focus_x: Option<f64>,
    ) -> Vec<PlotOverlay<'_, TimeSeriesMessage>> {
        let Some(point_index) = self.tooltip_focus_index() else {
            return Vec::new();
        };
        let values = self.tooltip_values(point_index);
        let Some(first) = values.first() else {
            return Vec::new();
        };
        let data_x = focus_x
            .filter(|x| x.is_finite())
            .unwrap_or(first.position[0]);
        let (anchor_x, horizontal) = if self.axis.live_mode {
            let anchor_x = self.x_axis_fraction(data_x);
            let horizontal = if anchor_x > 0.5 {
                Horizontal::Left
            } else {
                Horizontal::Right
            };
            (anchor_x, horizontal)
        } else {
            let (x_center, _, _) = self.axis.x_link.get();
            let horizontal = if data_x > x_center {
                Horizontal::Left
            } else {
                Horizontal::Right
            };
            (data_x, horizontal)
        };
        let offset = if horizontal == Horizontal::Left {
            [-TOOLTIP_OFFSET, 0.0]
        } else {
            [TOOLTIP_OFFSET, 0.0]
        };
        let tooltip = combined_tooltip(values);
        let overlay = if self.axis.live_mode {
            PlotOverlay::new(tooltip, [anchor_x, 0.5]).with_axes_transform()
        } else {
            PlotOverlay::new(tooltip, [anchor_x, 0.5]).with_transform_y(Transform::axes())
        };

        vec![
            overlay
                .with_anchor_offset(offset)
                .align_to_anchor(horizontal, Vertical::Center),
        ]
    }
}

fn combined_tooltip(values: Vec<TooltipValue>) -> Element<'static, TimeSeriesMessage> {
    let content: Column<'static, TimeSeriesMessage> = values.into_iter().fold(
        Column::new().spacing(TOOLTIP_ROW_SPACING),
        |content, value| {
            let marker_color = value.color;
            let marker = container(Space::new())
                .width(Length::Fixed(TOOLTIP_MARKER_WIDTH))
                .height(Length::Fixed(TOOLTIP_MARKER_HEIGHT))
                .style(move |_| {
                    container::Style::default().background(Background::Color(marker_color))
                });
            let label = text(value.label)
                .size(TOOLTIP_TEXT_SIZE)
                .width(Length::Fill)
                .wrapping(iced::widget::text::Wrapping::None);
            let value = text(value.value)
                .size(TOOLTIP_TEXT_SIZE)
                .wrapping(iced::widget::text::Wrapping::None);
            let row = Row::new()
                .push(marker)
                .push(label)
                .push(value)
                .spacing(TOOLTIP_ITEM_SPACING)
                .align_y(Vertical::Center);

            content.push(row)
        },
    );

    container(content)
        .width(Length::Fixed(TOOLTIP_WIDTH))
        .padding(TOOLTIP_PADDING)
        .style(style::tooltip)
        .into()
}
