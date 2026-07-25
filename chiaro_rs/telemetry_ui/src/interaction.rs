//! Pointer, keyboard, and drag-and-drop interaction for the Telemetry screen.

use iced::{Rectangle, Subscription, keyboard, mouse};

use super::{TelemetryMessage, TelemetryState};

pub(super) fn update_chart_drop_target(state: &mut TelemetryState) {
    let (Some(dragging), Some(origin), Some(cursor), Some(source_bounds)) = (
        state.dragging_chart,
        state.drag_origin,
        state.drag_cursor,
        state.drag_source_bounds,
    ) else {
        state.drop_target = None;
        return;
    };

    let dragged_bounds = Rectangle {
        x: source_bounds.x + cursor.x - origin.x,
        y: source_bounds.y + cursor.y - origin.y,
        ..source_bounds
    };
    state.drop_target = select_drop_target(dragging, dragged_bounds, &state.chart_order, |chart| {
        state.chart_visibility[chart.index()]
            .then(|| state.chart_layouts[chart.index()].and_then(|layout| layout.visible_bounds))
            .flatten()
    });
}

pub(super) fn update_chart_list_drop_target(state: &mut TelemetryState) {
    let (Some(dragging), Some(origin), Some(cursor), Some(source_bounds)) = (
        state.dragging_chart_list_item,
        state.chart_list_drag_origin,
        state.chart_list_drag_cursor,
        state.chart_list_drag_source_bounds,
    ) else {
        state.chart_list_drop_target = None;
        return;
    };

    let dragged_bounds = Rectangle {
        x: source_bounds.x + cursor.x - origin.x,
        y: source_bounds.y + cursor.y - origin.y,
        ..source_bounds
    };
    state.chart_list_drop_target =
        select_drop_target(dragging, dragged_bounds, &state.chart_order, |chart| {
            state.chart_list_layouts[chart.index()].and_then(|layout| layout.visible_bounds)
        });
}

pub(super) fn update_lap_analysis_drop_target(state: &mut TelemetryState) {
    let (Some(dragging), Some(origin), Some(cursor), Some(source_bounds)) = (
        state.dragging_lap_analysis_card,
        state.lap_analysis_drag_origin,
        state.lap_analysis_drag_cursor,
        state.lap_analysis_drag_source_bounds,
    ) else {
        state.lap_analysis_drop_target = None;
        return;
    };

    let dragged_bounds = Rectangle {
        x: source_bounds.x + cursor.x - origin.x,
        y: source_bounds.y + cursor.y - origin.y,
        ..source_bounds
    };
    state.lap_analysis_drop_target = select_drop_target(
        dragging,
        dragged_bounds,
        &state.lap_analysis_order,
        |card| state.lap_analysis_layouts[card.index()].and_then(|layout| layout.visible_bounds),
    );
}

pub(super) fn update_setup_card_drop_target(state: &mut TelemetryState) {
    let (Some(dragging), Some(origin), Some(cursor), Some(source_bounds)) = (
        state.dragging_setup_card,
        state.setup_card_drag_origin,
        state.setup_card_drag_cursor,
        state.setup_card_drag_source_bounds,
    ) else {
        state.setup_card_drop_target = None;
        return;
    };

    let dragged_bounds = Rectangle {
        x: source_bounds.x + cursor.x - origin.x,
        y: source_bounds.y + cursor.y - origin.y,
        ..source_bounds
    };
    state.setup_card_drop_target =
        select_drop_target(dragging, dragged_bounds, &state.setup_card_order, |card| {
            state.setup_card_layouts[card.index()].and_then(|layout| layout.visible_bounds)
        });
}

pub(super) fn select_drop_target<Id: Copy + Eq>(
    dragging: Id,
    dragged_bounds: Rectangle,
    order: &[Id],
    mut visible_bounds_for: impl FnMut(Id) -> Option<Rectangle>,
) -> Option<Id> {
    let mut best = None;

    for &candidate in order {
        if candidate == dragging {
            continue;
        }
        let Some(visible_bounds) = visible_bounds_for(candidate) else {
            continue;
        };
        let Some(overlap) = dragged_bounds.intersection(&visible_bounds) else {
            continue;
        };
        let area = overlap.width * overlap.height;

        if best.is_none_or(|(_, best_area)| area > best_area) {
            best = Some((candidate, area));
        }
    }

    best.map(|(item, _)| item)
}

pub(super) fn move_item_to<Id: Copy + Eq>(order: &mut Vec<Id>, item: Id, target: Id) -> bool {
    let Some(from) = order.iter().position(|candidate| *candidate == item) else {
        return false;
    };
    let Some(to) = order.iter().position(|candidate| *candidate == target) else {
        return false;
    };
    if from == to {
        return false;
    }

    let item = order.remove(from);
    order.insert(to.min(order.len()), item);
    true
}

pub fn subscription(state: &TelemetryState, active: bool) -> Subscription<TelemetryMessage> {
    if !active && !state.is_dragging_card() {
        return Subscription::none();
    }

    let modifier_input = iced::event::listen_with(modifier_event);
    let card_input = if active || state.is_dragging_card() {
        iced::event::listen_with(card_input_event)
    } else {
        Subscription::none()
    };
    let drag_cursor = if state.is_dragging_card() {
        iced::event::listen_with(drag_cursor_event)
    } else {
        Subscription::none()
    };
    Subscription::batch([modifier_input, card_input, drag_cursor])
}

fn modifier_event(
    event: iced::Event,
    _status: iced::event::Status,
    _window: iced::window::Id,
) -> Option<TelemetryMessage> {
    match event {
        iced::Event::Keyboard(keyboard::Event::ModifiersChanged(modifiers)) => {
            Some(TelemetryMessage::KeyboardModifiersChanged(modifiers))
        },
        iced::Event::Window(iced::window::Event::Unfocused) => {
            Some(TelemetryMessage::CancelPointerInteractions {
                reset_modifiers: true,
            })
        },
        _ => None,
    }
}

fn card_input_event(
    event: iced::Event,
    _status: iced::event::Status,
    _window: iced::window::Id,
) -> Option<TelemetryMessage> {
    match event {
        iced::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left))
        | iced::Event::Touch(iced::touch::Event::FingerLifted { .. }) => {
            Some(TelemetryMessage::FinishCardDrag)
        },
        iced::Event::Mouse(mouse::Event::CursorLeft)
        | iced::Event::Touch(iced::touch::Event::FingerLost { .. }) => {
            Some(TelemetryMessage::CancelPointerInteractions {
                reset_modifiers: false,
            })
        },
        _ => None,
    }
}

fn drag_cursor_event(
    event: iced::Event,
    _status: iced::event::Status,
    _window: iced::window::Id,
) -> Option<TelemetryMessage> {
    match event {
        iced::Event::Mouse(mouse::Event::CursorMoved { position }) => {
            Some(TelemetryMessage::DragCursor(position))
        },
        iced::Event::Touch(iced::touch::Event::FingerMoved { position, .. }) => {
            Some(TelemetryMessage::DragCursor(position))
        },
        _ => None,
    }
}
