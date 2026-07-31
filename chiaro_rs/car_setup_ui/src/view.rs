//! Iced view for the Car Setup screen.

use chiaro_actions::ReferenceIbtState;
use chiaro_telemetry::{LiveTelemetrySourceInfo, Session};
use chiaro_widgets::{CardTitle, bounds_reporter, pane_card_with_maximize};
use iced::{
    Element, Length, Vector, mouse,
    widget::{column, container, float, mouse_area, row, scrollable},
};

use crate::{
    CarSetupMessage, CarSetupState,
    setup_view::{self, CardSide},
};

const CONTENT_PADDING: f32 = 16.0;
const CARD_SPACING: f32 = 10.0;
const CARD_TITLE_ICON_SIZE: f32 = 16.0;

pub fn view<'a>(
    state: &'a CarSetupState,
    session: &'a Session,
    reference_session: Option<&'a Session>,
    reference_ibt: &'a ReferenceIbtState,
    live_source: LiveTelemetrySourceInfo,
) -> Element<'a, CarSetupMessage> {
    container(content(
        state,
        session,
        reference_session,
        reference_ibt,
        live_source,
    ))
    .padding(CONTENT_PADDING)
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}

fn content<'a>(
    state: &'a CarSetupState,
    session: &'a Session,
    reference_session: Option<&'a Session>,
    reference_ibt: &'a ReferenceIbtState,
    live_source: LiveTelemetrySourceInfo,
) -> Element<'a, CarSetupMessage> {
    if let Some(card) = state.maximized_card.as_ref() {
        return draggable_card(
            state,
            session,
            reference_session,
            reference_ibt,
            live_source,
            card.clone(),
        );
    }

    let order = state.current_card_order();
    let mut content = column![].spacing(CARD_SPACING).width(Length::Fill);
    let mut placed = vec![false; order.len()];

    for (index, card) in order.iter().enumerate() {
        if placed[index] {
            continue;
        }
        placed[index] = true;

        let Some(pair) = setup_view::SetupViewData::card_pair(card) else {
            content = content.push(draggable_card(
                state,
                session,
                reference_session,
                reference_ibt,
                live_source,
                card.clone(),
            ));
            continue;
        };
        let counterpart = order
            .iter()
            .enumerate()
            .find_map(|(candidate_index, candidate)| {
                if placed[candidate_index] {
                    return None;
                }
                let candidate_pair = setup_view::SetupViewData::card_pair(candidate)?;
                (candidate_pair.group == pair.group && candidate_pair.side != pair.side)
                    .then_some((candidate_index, candidate.clone(), candidate_pair.side))
            });

        let Some((counterpart_index, counterpart, counterpart_side)) = counterpart else {
            content = content.push(draggable_card(
                state,
                session,
                reference_session,
                reference_ibt,
                live_source,
                card.clone(),
            ));
            continue;
        };
        placed[counterpart_index] = true;
        let (left, right) = if pair.side == CardSide::Left {
            (card.clone(), counterpart)
        } else if counterpart_side == CardSide::Left {
            (counterpart, card.clone())
        } else {
            unreachable!("paired cards must have opposite sides")
        };
        content = content.push(
            row![
                draggable_card(
                    state,
                    session,
                    reference_session,
                    reference_ibt,
                    live_source,
                    left,
                ),
                draggable_card(
                    state,
                    session,
                    reference_session,
                    reference_ibt,
                    live_source,
                    right,
                ),
            ]
            .spacing(CARD_SPACING)
            .width(Length::Fill),
        );
    }

    scrollable(content)
        .spacing(10)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

fn draggable_card<'a>(
    state: &'a CarSetupState,
    session: &'a Session,
    reference_session: Option<&'a Session>,
    reference_ibt: &'a ReferenceIbtState,
    live_source: LiveTelemetrySourceInfo,
    card: String,
) -> Element<'a, CarSetupMessage> {
    let maximized = state.maximized_card.as_ref() == Some(&card);
    let collapsed = state.card_collapsed.get(&card).copied().unwrap_or(false);
    let interaction = if state.dragging_card.as_ref() == Some(&card) {
        mouse::Interaction::Grabbing
    } else {
        mouse::Interaction::Grab
    };
    let drag = if maximized {
        None
    } else {
        Some((CarSetupMessage::BeginCardDrag(card.clone()), interaction))
    };
    let actions_visible =
        state.hovered_card.as_ref() == Some(&card) || state.dragging_card.as_ref() == Some(&card);
    let highlighted = state.dragging_card.is_some()
        && (state.dragging_card.as_ref() == Some(&card)
            || state.drop_target.as_ref() == Some(&card));
    let mut title = CardTitle::new(
        state.setup.card_title(session, &card),
        setup_view::SetupViewData::card_icon(&card).size(CARD_TITLE_ICON_SIZE),
    );
    if let Some(trailing) = state.setup.card_title_trailing(&card) {
        title = title.with_trailing(trailing);
    }
    let card_content = pane_card_with_maximize(
        title,
        state.setup.card_content(
            setup_view::SetupViewContext {
                session,
                reference_session,
                reference_ibt,
                reference: state.reference_setup.as_ref(),
                live_source,
            },
            &card,
        ),
        0.0,
        drag,
        actions_visible,
        maximized,
        collapsed,
        CarSetupMessage::ToggleCardMaximized(card.clone()),
        CarSetupMessage::ToggleCardCollapsed(card.clone()),
        highlighted,
    );
    let card_content: Element<'_, CarSetupMessage> = mouse_area(card_content)
        .on_enter(CarSetupMessage::SetHoveredCard(Some(card.clone())))
        .on_exit(CarSetupMessage::SetHoveredCard(None))
        .into();
    let card_content: Element<'_, CarSetupMessage> = if state.dragging_card.as_ref() == Some(&card)
        && let (Some(origin), Some(cursor)) = (state.drag_origin, state.drag_cursor)
    {
        float(card_content)
            .translate(move |_, _| Vector::new(cursor.x - origin.x, cursor.y - origin.y))
            .into()
    } else {
        card_content
    };

    bounds_reporter(card, card_content, |card, bounds, visible_bounds| {
        CarSetupMessage::CardLayoutChanged {
            card,
            bounds,
            visible_bounds,
        }
    })
}
