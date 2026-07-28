use chiaro_i18n::{Text, tr};
use chiaro_widgets::context_menu as chart_context_menu;
use iced::{
    Background, Border, Color as IcedColor, Element, Length, Theme,
    alignment::{Horizontal, Vertical},
    keyboard,
    widget::{Column, Row, Space, button, container, mouse_area, rule, text},
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
const CONTEXT_MENU_WIDTH: f32 = 220.0;
const SECTOR_LINE_ALPHA: f32 = 0.42;
const SECTOR_LABEL_ALPHA: f32 = 0.16;
const RANGE_PORTIONS: u16 = 10_000;

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
        let mut bottom_overlays = self
            .range_background_overlay()
            .into_iter()
            .collect::<Vec<_>>();
        bottom_overlays.extend(self.marker_line_overlays());
        bottom_overlays.extend(focus_line);
        let top_overlays = self
            .marker_label_overlays()
            .into_iter()
            .chain(self.tooltip_overlays(focus_x));

        let plot = self.plot.view_with_shapes(
            bottom_overlays.into_iter(),
            top_overlays,
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
        let marker_menu_label = if self.interaction.markers_visible {
            tr(Text::HideSectors)
        } else {
            tr(Text::ShowSectors)
        };
        let marker_menu_item = container(text(marker_menu_label).size(13))
            .width(Length::Fill)
            .align_x(Horizontal::Left);
        let menu = self
            .series
            .labels
            .iter()
            .zip(&self.series.colors)
            .zip(&self.series.visible)
            .enumerate()
            .fold(
                Column::new()
                    .push(
                        button(menu_item)
                            .width(Length::Fill)
                            .padding([6, 8])
                            .style(style::context_menu_item)
                            .on_press(TimeSeriesMessage::ToggleTooltips),
                    )
                    .push(
                        button(marker_menu_item)
                            .width(Length::Fill)
                            .padding([6, 8])
                            .style(style::context_menu_item)
                            .on_press(TimeSeriesMessage::ToggleMarkers),
                    )
                    .push(rule::horizontal(1)),
                |menu, (index, ((label, color), visible))| {
                    let marker_color = *color;
                    let marker = container(Space::new())
                        .width(Length::Fixed(12.0))
                        .height(Length::Fixed(3.0))
                        .style(move |_| {
                            container::Style::default().background(Background::Color(marker_color))
                        });
                    let check: Element<'_, TimeSeriesMessage> = if *visible {
                        text("✓").size(14).into()
                    } else {
                        Space::new()
                            .width(Length::Fixed(14.0))
                            .height(Length::Fixed(14.0))
                            .into()
                    };
                    let item = Row::new()
                        .push(marker)
                        .push(
                            text(*label)
                                .size(13)
                                .width(Length::Fill)
                                .wrapping(iced::widget::text::Wrapping::None),
                        )
                        .push(check)
                        .spacing(7)
                        .align_y(Vertical::Center);

                    menu.push(
                        button(item)
                            .width(Length::Fill)
                            .padding([6, 8])
                            .style(style::context_menu_item)
                            .on_press(TimeSeriesMessage::ToggleSeriesVisibility(index)),
                    )
                },
            );
        let menu = container(menu)
            .width(Length::Fixed(CONTEXT_MENU_WIDTH))
            .padding(4)
            .style(style::context_menu);

        chart_context_menu(plot, menu)
            .open(self.interaction.context.is_some())
            .on_open(TimeSeriesMessage::OpenContextMenu)
            .on_close(TimeSeriesMessage::CloseContextMenu)
            .into()
    }

    pub(super) fn marker_line_overlays(&self) -> Vec<PlotOverlay<'_, TimeSeriesMessage>> {
        if !self.interaction.markers_visible {
            return Vec::new();
        }

        self.markers
            .iter()
            .filter(|marker| marker.x.is_finite())
            .map(|marker| {
                let color = with_alpha(marker.color, SECTOR_LINE_ALPHA);
                let line = container(Space::new())
                    .width(Length::Fixed(1.0))
                    .height(Length::Fill)
                    .style(move |_| {
                        container::Style::default().background(Background::Color(color))
                    });
                self.marker_overlay(line, marker.x, 0.5)
            })
            .collect()
    }

    pub(super) fn range_background_overlay(
        &self,
    ) -> Option<PlotOverlay<'static, TimeSeriesMessage>> {
        if !self.interaction.markers_visible || self.ranges.is_empty() {
            return None;
        }

        let mut ranges = self
            .ranges
            .iter()
            .filter_map(|range| {
                let start = range_portion(self.x_axis_fraction(range.start_x));
                let end = range_portion(self.x_axis_fraction(range.end_x));
                (end > start).then_some((start, end, range.color))
            })
            .collect::<Vec<_>>();
        ranges.sort_by_key(|range| range.0);
        if ranges.is_empty() {
            return None;
        }

        let mut background = Row::new().width(Length::Fill).height(Length::Fill);
        let mut cursor = 0;
        for (start, end, color) in ranges {
            let start = start.max(cursor);
            if start > cursor {
                background = background.push(range_block(start - cursor, None));
            }
            if end > start {
                background = background.push(range_block(end - start, Some(color)));
                cursor = end;
            }
        }
        if cursor < RANGE_PORTIONS {
            background = background.push(range_block(RANGE_PORTIONS - cursor, None));
        }

        Some(
            PlotOverlay::new(background, [0.0, 1.0])
                .with_axes_transform()
                .align_to_anchor(Horizontal::Right, Vertical::Bottom),
        )
    }

    pub(super) fn marker_label_overlays(&self) -> Vec<PlotOverlay<'_, TimeSeriesMessage>> {
        if !self.interaction.markers_visible {
            return Vec::new();
        }

        self.markers
            .iter()
            .filter(|marker| marker.x.is_finite())
            .map(|marker| {
                let color = marker.color;
                let background = with_alpha(color, SECTOR_LABEL_ALPHA);
                let label = container(text(marker.label.as_str()).size(11).color(color))
                    .padding([2, 5])
                    .style(move |_| container::Style {
                        background: Some(Background::Color(background)),
                        border: Border {
                            radius: 4.0.into(),
                            ..Border::default()
                        },
                        ..container::Style::default()
                    });
                self.marker_overlay(label, marker.x, 1.0)
                    .with_anchor_offset([0.0, -4.0])
                    .align_to_anchor(Horizontal::Center, Vertical::Bottom)
            })
            .collect()
    }

    fn marker_overlay<'a>(
        &self,
        element: impl Into<Element<'a, TimeSeriesMessage>>,
        x: f64,
        y: f64,
    ) -> PlotOverlay<'a, TimeSeriesMessage> {
        if self.axis.live_mode {
            PlotOverlay::new(element, [self.x_axis_fraction(x), y]).with_axes_transform()
        } else {
            PlotOverlay::new(element, [x, y]).with_transform_y(Transform::axes())
        }
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
            .zip(&self.series.visible)
            .filter_map(|(((series_id, label), color), visible)| {
                if !visible {
                    return None;
                }
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

const fn with_alpha(color: IcedColor, alpha: f32) -> IcedColor {
    IcedColor { a: alpha, ..color }
}

fn range_portion(fraction: f64) -> u16 {
    (fraction.clamp(0.0, 1.0) * f64::from(RANGE_PORTIONS)).round() as u16
}

fn range_block(portion: u16, color: Option<IcedColor>) -> Element<'static, TimeSeriesMessage> {
    let mut block = container(Space::new())
        .width(Length::FillPortion(portion))
        .height(Length::Fill);
    if let Some(color) = color {
        block =
            block.style(move |_| container::Style::default().background(Background::Color(color)));
    }
    block.into()
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
