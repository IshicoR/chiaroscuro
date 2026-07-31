//! Pointer and drag-and-drop interaction for the Car Setup screen.

use iced::{Rectangle, Subscription, mouse};

use crate::{CarSetupMessage, CarSetupState};

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct CardLayout {
    pub(super) bounds: Rectangle,
    pub(super) visible_bounds: Option<Rectangle>,
}

pub fn subscription(state: &CarSetupState, active: bool) -> Subscription<CarSetupMessage> {
    if !active && state.dragging_card.is_none() {
        return Subscription::none();
    }

    let card_input = if active || state.dragging_card.is_some() {
        iced::event::listen_with(card_input_event)
    } else {
        Subscription::none()
    };
    let drag_cursor = if state.dragging_card.is_some() {
        iced::event::listen_with(drag_cursor_event)
    } else {
        Subscription::none()
    };
    Subscription::batch([card_input, drag_cursor])
}

fn card_input_event(
    event: iced::Event,
    _status: iced::event::Status,
    _window: iced::window::Id,
) -> Option<CarSetupMessage> {
    match event {
        iced::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left))
        | iced::Event::Touch(iced::touch::Event::FingerLifted { .. }) => {
            Some(CarSetupMessage::FinishCardDrag)
        },
        iced::Event::Mouse(mouse::Event::CursorLeft)
        | iced::Event::Touch(iced::touch::Event::FingerLost { .. })
        | iced::Event::Window(iced::window::Event::Unfocused) => {
            Some(CarSetupMessage::CancelPointerInteractions)
        },
        _ => None,
    }
}

fn drag_cursor_event(
    event: iced::Event,
    _status: iced::event::Status,
    _window: iced::window::Id,
) -> Option<CarSetupMessage> {
    match event {
        iced::Event::Mouse(mouse::Event::CursorMoved { position }) => {
            Some(CarSetupMessage::DragCursor(position))
        },
        iced::Event::Touch(iced::touch::Event::FingerMoved { position, .. }) => {
            Some(CarSetupMessage::DragCursor(position))
        },
        _ => None,
    }
}

pub(super) fn update_drop_target(state: &mut CarSetupState) {
    let (Some(dragging), Some(origin), Some(cursor), Some(source_bounds)) = (
        state.dragging_card.as_ref(),
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
    let order = state.current_card_order();
    state.drop_target = select_drop_target(dragging, dragged_bounds, &order, |card| {
        state
            .card_layouts
            .get(card)
            .and_then(|layout| layout.visible_bounds)
    });
}

fn select_drop_target(
    dragging: &str,
    dragged_bounds: Rectangle,
    order: &[String],
    mut visible_bounds_for: impl FnMut(&str) -> Option<Rectangle>,
) -> Option<String> {
    let mut best = None;

    for candidate in order {
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

        if best.as_ref().is_none_or(|(_, best_area)| area > *best_area) {
            best = Some((candidate.clone(), area));
        }
    }

    best.map(|(item, _)| item)
}

pub(super) fn move_item_to(order: &mut Vec<String>, item: &str, target: &str) -> bool {
    let Some(from) = order.iter().position(|candidate| candidate == item) else {
        return false;
    };
    let Some(to) = order.iter().position(|candidate| candidate == target) else {
        return false;
    };
    if from == to {
        return false;
    }

    let item = order.remove(from);
    order.insert(to.min(order.len()), item);
    true
}
