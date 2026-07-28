use iced::{Point, keyboard, mouse};
use iced_plot::{PlotUiMessage, PointId};

use super::TimeSeriesChart;

#[derive(Debug, Clone)]
pub enum TimeSeriesMessage {
    Plot(PlotUiMessage),
    BeginCursorDrag,
    EndCursorDrag,
    PanX(mouse::ScrollDelta),
    ZoomX(mouse::ScrollDelta),
    ResetX,
    OpenContextMenu(Point),
    CloseContextMenu,
    ToggleTooltips,
    ToggleMarkers,
    ToggleSeriesVisibility(usize),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ScrollAction {
    PanX,
    ZoomX,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct ChartContext {
    pub(super) local_position: Point,
    pub(super) data_position: Option<[f64; 2]>,
}

pub(super) struct InteractionState {
    pub(super) focus_index: Option<usize>,
    pub(super) cursor_position: Option<[f64; 2]>,
    pub(super) cursor_dragging: bool,
    pub(super) tooltips_visible: bool,
    pub(super) markers_visible: bool,
    pub(super) context: Option<ChartContext>,
}

impl TimeSeriesChart {
    pub fn update(&mut self, message: TimeSeriesMessage) -> Option<f64> {
        match message {
            TimeSeriesMessage::Plot(mut message) => {
                let cursor_position = match &mut message {
                    PlotUiMessage::RenderUpdate(update) => update
                        .cursor_position_ui
                        .take()
                        .map(|cursor| [cursor.x, cursor.y]),
                    _ => None,
                };
                self.plot.update(message);

                cursor_position.and_then(|cursor| self.track_cursor(cursor))
            },
            TimeSeriesMessage::BeginCursorDrag => {
                self.interaction.cursor_dragging = true;
                self.interaction.cursor_position.map(|cursor| cursor[0])
            },
            TimeSeriesMessage::EndCursorDrag => {
                self.interaction.cursor_dragging = false;
                None
            },
            TimeSeriesMessage::PanX(delta) => {
                self.pan_x(delta);
                None
            },
            TimeSeriesMessage::ZoomX(delta) => {
                self.zoom_x(delta);
                None
            },
            TimeSeriesMessage::ResetX => {
                self.reset_x_view();
                None
            },
            TimeSeriesMessage::OpenContextMenu(local_position) => {
                self.interaction.context = Some(ChartContext {
                    local_position,
                    data_position: self.interaction.cursor_position,
                });
                None
            },
            TimeSeriesMessage::CloseContextMenu => {
                self.interaction.context = None;
                None
            },
            TimeSeriesMessage::ToggleTooltips => {
                self.interaction.tooltips_visible = !self.interaction.tooltips_visible;
                self.interaction.context = None;
                None
            },
            TimeSeriesMessage::ToggleMarkers => {
                self.interaction.markers_visible = !self.interaction.markers_visible;
                None
            },
            TimeSeriesMessage::ToggleSeriesVisibility(index) => {
                if let Some((id, visible)) = self
                    .series
                    .ids
                    .get(index)
                    .zip(self.series.visible.get_mut(index))
                {
                    self.plot.update(PlotUiMessage::ToggleSeriesVisibility(*id));
                    *visible = !*visible;
                }
                None
            },
        }
    }

    pub fn cancel_interaction(&mut self) {
        self.interaction.cursor_dragging = false;
        self.interaction.context = None;
    }

    pub fn set_focus_index(&mut self, point_index: Option<usize>) {
        if self.interaction.focus_index == point_index {
            return;
        }

        if !self.axis.live_mode {
            self.plot.clear_pick();
            if let Some(point_index) = point_index {
                for ((series_id, series_length), visible) in self
                    .series
                    .ids
                    .iter()
                    .zip(&self.series.lengths)
                    .zip(&self.series.visible)
                {
                    if !visible {
                        continue;
                    }
                    if point_index >= *series_length {
                        continue;
                    }
                    self.plot.add_pick_point(PointId {
                        series_id: *series_id,
                        point_index,
                    });
                }
            }
        }
        self.interaction.focus_index = point_index;
    }

    #[doc(hidden)]
    pub const fn focus_index(&self) -> Option<usize> {
        self.interaction.focus_index
    }

    #[doc(hidden)]
    pub const fn is_cursor_dragging(&self) -> bool {
        self.interaction.cursor_dragging
    }

    #[doc(hidden)]
    pub const fn tooltips_visible(&self) -> bool {
        self.interaction.tooltips_visible
    }

    #[doc(hidden)]
    pub const fn markers_visible(&self) -> bool {
        self.interaction.markers_visible
    }

    #[doc(hidden)]
    pub const fn context_menu_target(&self) -> Option<[f64; 2]> {
        match self.interaction.context {
            Some(context) => context.data_position,
            None => None,
        }
    }

    #[doc(hidden)]
    pub const fn is_context_menu_open(&self) -> bool {
        self.interaction.context.is_some()
    }

    pub(super) fn track_cursor(&mut self, cursor: [f64; 2]) -> Option<f64> {
        self.interaction.cursor_position = Some(cursor);
        self.interaction.cursor_dragging.then_some(cursor[0])
    }
}

pub(super) fn scroll_action(modifiers: keyboard::Modifiers) -> Option<ScrollAction> {
    if modifiers.control() {
        Some(ScrollAction::ZoomX)
    } else if modifiers.shift() {
        Some(ScrollAction::PanX)
    } else {
        None
    }
}
