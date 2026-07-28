//! Car setup screen state, update logic, and view.

mod setup_view;

use std::collections::BTreeMap;

use chiaro_telemetry::{LiveTelemetrySourceInfo, Session};
use chiaro_widgets::{CardTitle, bounds_reporter, pane_card_with_maximize};
use iced::{
    Element, Length, Point, Rectangle, Subscription, Vector,
    alignment::Vertical,
    mouse,
    widget::{column, container, float, mouse_area, row, scrollable},
};

const CONTENT_PADDING: f32 = 24.0;
const CARD_SPACING: f32 = 12.0;
const CARD_TITLE_ICON_SIZE: f32 = 16.0;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CarSetupLayoutFlag {
    pub key: String,
    pub value: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CarSetupLayout {
    pub card_order: Vec<String>,
    pub card_collapsed: Vec<CarSetupLayoutFlag>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct CardLayout {
    bounds: Rectangle,
    visible_bounds: Option<Rectangle>,
}

#[derive(Debug, Default)]
pub struct CarSetupState {
    cached_session_info_revision: Option<u64>,
    setup: setup_view::SetupViewData,
    layout_revision: u64,
    card_order: Vec<String>,
    card_collapsed: BTreeMap<String, bool>,
    card_layouts: BTreeMap<String, CardLayout>,
    maximized_card: Option<String>,
    hovered_card: Option<String>,
    dragging_card: Option<String>,
    drop_target: Option<String>,
    drag_origin: Option<Point>,
    drag_cursor: Option<Point>,
    drag_source_bounds: Option<Rectangle>,
}

impl CarSetupState {
    pub const fn layout_revision(&self) -> u64 {
        self.layout_revision
    }

    pub fn layout_snapshot(&self) -> CarSetupLayout {
        CarSetupLayout {
            card_order: self.card_order.clone(),
            card_collapsed: self
                .card_collapsed
                .iter()
                .map(|(key, value)| CarSetupLayoutFlag {
                    key: key.clone(),
                    value: *value,
                })
                .collect(),
        }
    }

    /// Restores a persisted layout without marking it as a user edit.
    pub fn apply_layout(&mut self, layout: &CarSetupLayout) {
        self.card_order = normalize_order(&layout.card_order);
        self.card_collapsed = normalize_flags(&layout.card_collapsed);
        self.maximized_card = None;
        self.clear_drag();
        self.card_layouts.clear();
    }

    fn mark_layout_changed(&mut self) {
        self.layout_revision = self.layout_revision.wrapping_add(1);
    }

    fn clear_drag(&mut self) {
        self.dragging_card = None;
        self.drop_target = None;
        self.drag_origin = None;
        self.drag_cursor = None;
        self.drag_source_bounds = None;
    }

    fn reconcile_cards(&mut self) {
        let available = self.setup.card_keys();
        for key in &available {
            if !self.card_order.contains(key) {
                self.card_order.push(key.clone());
            }
        }
        if self
            .maximized_card
            .as_ref()
            .is_some_and(|key| !available.contains(key))
        {
            self.maximized_card = None;
        }
        if self
            .dragging_card
            .as_ref()
            .is_some_and(|key| !available.contains(key))
        {
            self.clear_drag();
        }
        self.card_layouts.clear();
    }

    fn current_card_order(&self) -> Vec<String> {
        let available = self.setup.card_keys();
        let mut order = self
            .card_order
            .iter()
            .filter(|key| available.contains(key))
            .cloned()
            .collect::<Vec<_>>();
        for key in available {
            if !order.contains(&key) {
                order.push(key);
            }
        }
        order
    }

    fn merge_current_order(&mut self, order: Vec<String>) {
        self.card_order.retain(|key| !order.contains(key));
        self.card_order.extend(order);
    }
}

#[derive(Debug, Clone)]
pub enum CarSetupMessage {
    ToggleCardCollapsed(String),
    ToggleCardMaximized(String),
    SetHoveredCard(Option<String>),
    BeginCardDrag(String),
    CardLayoutChanged {
        card: String,
        bounds: Rectangle,
        visible_bounds: Option<Rectangle>,
    },
    FinishCardDrag,
    DragCursor(Point),
    CancelPointerInteractions,
    ResetLayout,
}

impl CarSetupMessage {
    pub const fn resets_layout(&self) -> bool {
        matches!(self, Self::ResetLayout)
    }
}

pub fn activate(state: &mut CarSetupState, session: &Session) {
    deactivate(state);
    refresh(state, session);
}

pub fn deactivate(state: &mut CarSetupState) {
    state.clear_drag();
}

pub fn refresh(state: &mut CarSetupState, session: &Session) {
    let revision = session.session_info_revision();
    if state.cached_session_info_revision == Some(revision) {
        return;
    }

    state.cached_session_info_revision = Some(revision);
    state.setup = match session.session_info().map(chiaro_irsdk::SessionInfo::parse) {
        Some(Ok(document)) => setup_view::SetupViewData::from_document(&document),
        Some(Err(error)) => setup_view::SetupViewData::parse_error(error.to_string()),
        None => setup_view::SetupViewData::default(),
    };
    state.reconcile_cards();
}

pub fn reset_session(state: &mut CarSetupState, session: &Session, active: bool) {
    state.cached_session_info_revision = None;
    state.setup = setup_view::SetupViewData::default();
    if active {
        refresh(state, session);
    }
}

pub fn update(state: &mut CarSetupState, message: CarSetupMessage) {
    match message {
        CarSetupMessage::ToggleCardCollapsed(card) => {
            let collapsed = !state.card_collapsed.get(&card).copied().unwrap_or(false);
            state.card_collapsed.insert(card.clone(), collapsed);
            if collapsed && state.maximized_card.as_ref() == Some(&card) {
                state.maximized_card = None;
            }
            state.card_layouts.remove(&card);
            state.clear_drag();
            state.mark_layout_changed();
        },
        CarSetupMessage::ToggleCardMaximized(card) => {
            let maximizing = state.maximized_card.as_ref() != Some(&card);
            state.maximized_card = maximizing.then(|| card.clone());
            let expanded_collapsed =
                maximizing && state.card_collapsed.get(&card).copied().unwrap_or(false);
            if expanded_collapsed {
                state.card_collapsed.insert(card, false);
                state.mark_layout_changed();
            }
            state.card_layouts.clear();
            state.clear_drag();
        },
        CarSetupMessage::SetHoveredCard(card) => state.hovered_card = card,
        CarSetupMessage::BeginCardDrag(card) => {
            state.clear_drag();
            state.drag_source_bounds = state.card_layouts.get(&card).map(|layout| layout.bounds);
            state.dragging_card = Some(card);
        },
        CarSetupMessage::CardLayoutChanged {
            card,
            bounds,
            visible_bounds,
        } => {
            state.card_layouts.insert(
                card.clone(),
                CardLayout {
                    bounds,
                    visible_bounds,
                },
            );
            if state.dragging_card.as_ref() == Some(&card) && state.drag_source_bounds.is_none() {
                state.drag_source_bounds = Some(bounds);
            }
            update_drop_target(state);
        },
        CarSetupMessage::FinishCardDrag => {
            let changed = if let (Some(dragging), Some(target)) =
                (state.dragging_card.clone(), state.drop_target.clone())
            {
                let mut order = state.current_card_order();
                if move_item_to(&mut order, &dragging, &target) {
                    state.merge_current_order(order);
                    state.card_layouts.clear();
                    true
                } else {
                    false
                }
            } else {
                false
            };
            state.clear_drag();
            if changed {
                state.mark_layout_changed();
            }
        },
        CarSetupMessage::DragCursor(position) => {
            if state.dragging_card.is_some() {
                state.drag_origin.get_or_insert(position);
                state.drag_cursor = Some(position);
                update_drop_target(state);
            }
        },
        CarSetupMessage::CancelPointerInteractions => state.clear_drag(),
        CarSetupMessage::ResetLayout => {
            state.apply_layout(&CarSetupLayout::default());
            state.mark_layout_changed();
        },
    }
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

fn update_drop_target(state: &mut CarSetupState) {
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

fn move_item_to(order: &mut Vec<String>, item: &str, target: &str) -> bool {
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

fn normalize_order(keys: &[String]) -> Vec<String> {
    let mut normalized = Vec::new();
    for key in keys {
        if !key.trim().is_empty() && !normalized.contains(key) {
            normalized.push(key.clone());
        }
    }
    normalized
}

fn normalize_flags(flags: &[CarSetupLayoutFlag]) -> BTreeMap<String, bool> {
    let mut normalized = BTreeMap::new();
    for flag in flags {
        if !flag.key.trim().is_empty() {
            normalized.entry(flag.key.clone()).or_insert(flag.value);
        }
    }
    normalized
}

pub fn view<'a>(
    state: &'a CarSetupState,
    session: &'a Session,
    live_source: LiveTelemetrySourceInfo,
) -> Element<'a, CarSetupMessage> {
    container(content(state, session, live_source))
        .padding(CONTENT_PADDING)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

fn content<'a>(
    state: &'a CarSetupState,
    session: &'a Session,
    live_source: LiveTelemetrySourceInfo,
) -> Element<'a, CarSetupMessage> {
    let order = state
        .maximized_card
        .as_ref()
        .map_or_else(|| state.current_card_order(), |card| vec![card.clone()]);
    let mut content = column![].spacing(CARD_SPACING).width(Length::Fill);
    let mut section_run = Vec::new();

    for card in order {
        if setup_view::SetupViewData::card_is_full_width(&card) {
            if !section_run.is_empty() {
                content = content.push(card_columns(
                    state,
                    session,
                    live_source,
                    std::mem::take(&mut section_run),
                ));
            }
            content = content.push(draggable_card(state, session, live_source, card));
        } else {
            section_run.push(card);
        }
    }
    if !section_run.is_empty() {
        content = content.push(card_columns(state, session, live_source, section_run));
    }

    scrollable(content)
        .spacing(10)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

fn card_columns<'a>(
    state: &'a CarSetupState,
    session: &'a Session,
    live_source: LiveTelemetrySourceInfo,
    cards: Vec<String>,
) -> Element<'a, CarSetupMessage> {
    if cards.len() == 1 {
        return draggable_card(state, session, live_source, cards[0].clone());
    }

    let mut columns: [Vec<String>; 2] = std::array::from_fn(|_| Vec::new());
    let mut weights = [0_usize; 2];
    for card in cards {
        let column = usize::from(weights[1] < weights[0]);
        weights[column] = weights[column].saturating_add(state.setup.card_weight(&card));
        columns[column].push(card);
    }
    let [left_cards, right_cards] = columns;
    let left = left_cards.into_iter().fold(
        column![].spacing(CARD_SPACING).width(Length::Fill),
        |column, card| column.push(draggable_card(state, session, live_source, card)),
    );
    let right = right_cards.into_iter().fold(
        column![].spacing(CARD_SPACING).width(Length::Fill),
        |column, card| column.push(draggable_card(state, session, live_source, card)),
    );

    row![left, right]
        .spacing(CARD_SPACING)
        .align_y(Vertical::Top)
        .width(Length::Fill)
        .into()
}

fn draggable_card<'a>(
    state: &'a CarSetupState,
    session: &'a Session,
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
        state.setup.card_content(session, live_source, &card),
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

#[cfg(test)]
mod tests {
    use iced::{Point, Rectangle};

    use super::{
        CarSetupLayout, CarSetupLayoutFlag, CarSetupMessage, CarSetupState, normalize_flags,
        normalize_order, update,
    };

    fn card_bounds(y: f32) -> Rectangle {
        Rectangle {
            x: 0.0,
            y,
            width: 100.0,
            height: 100.0,
        }
    }

    fn report_card_layout(state: &mut CarSetupState, card: &str, bounds: Rectangle) {
        update(
            state,
            CarSetupMessage::CardLayoutChanged {
                card: card.to_owned(),
                bounds,
                visible_bounds: Some(bounds),
            },
        );
    }

    fn begin_card_drag(state: &mut CarSetupState, card: &str) {
        update(state, CarSetupMessage::BeginCardDrag(card.to_owned()));
        update(state, CarSetupMessage::DragCursor(Point::new(50.0, 20.0)));
    }

    #[test]
    fn persisted_layout_is_normalized_without_marking_an_edit() {
        let mut state = CarSetupState::default();
        state.apply_layout(&CarSetupLayout {
            card_order: vec!["summary".into(), String::new(), "summary".into()],
            card_collapsed: vec![
                CarSetupLayoutFlag {
                    key: "summary".into(),
                    value: true,
                },
                CarSetupLayoutFlag {
                    key: "summary".into(),
                    value: false,
                },
            ],
        });

        assert_eq!(state.layout_revision(), 0);
        assert_eq!(state.card_order, ["summary"]);
        assert_eq!(state.card_collapsed.get("summary"), Some(&true));
    }

    #[test]
    fn layout_reset_is_a_persistable_change() {
        let mut state = CarSetupState::default();

        update(&mut state, CarSetupMessage::ResetLayout);

        assert_eq!(state.layout_revision(), 1);
        assert!(CarSetupMessage::ResetLayout.resets_layout());
    }

    #[test]
    fn dynamic_layout_helpers_ignore_empty_and_duplicate_keys() {
        assert_eq!(
            normalize_order(&["summary".into(), String::new(), "summary".into()]),
            ["summary"]
        );
        assert_eq!(
            normalize_flags(&[
                CarSetupLayoutFlag {
                    key: "status".into(),
                    value: true,
                },
                CarSetupLayoutFlag {
                    key: "status".into(),
                    value: false,
                },
            ])
            .get("status"),
            Some(&true)
        );
    }

    #[test]
    fn cards_reorder_only_while_their_bounds_overlap() {
        let mut state = CarSetupState::default();
        let source = card_bounds(0.0);
        let target = card_bounds(116.0);

        report_card_layout(&mut state, "summary", source);
        report_card_layout(&mut state, "status", target);
        begin_card_drag(&mut state, "summary");
        update(
            &mut state,
            CarSetupMessage::DragCursor(Point::new(50.0, 40.0)),
        );
        assert_eq!(state.drop_target.as_deref(), Some("status"));

        update(
            &mut state,
            CarSetupMessage::DragCursor(Point::new(50.0, 20.0)),
        );
        assert_eq!(state.drop_target, None);
        update(&mut state, CarSetupMessage::FinishCardDrag);
        assert_eq!(state.current_card_order(), ["summary", "status"]);

        report_card_layout(&mut state, "summary", source);
        report_card_layout(&mut state, "status", target);
        begin_card_drag(&mut state, "summary");
        update(
            &mut state,
            CarSetupMessage::DragCursor(Point::new(50.0, 40.0)),
        );
        update(&mut state, CarSetupMessage::FinishCardDrag);

        assert_eq!(state.current_card_order(), ["status", "summary"]);
        assert_eq!(state.layout_revision(), 1);
        assert_eq!(state.dragging_card, None);
        assert_eq!(state.drop_target, None);
    }

    #[test]
    fn collapse_and_maximize_keep_a_single_visible_mode() {
        let mut state = CarSetupState::default();

        update(
            &mut state,
            CarSetupMessage::ToggleCardMaximized("summary".to_owned()),
        );
        assert_eq!(state.maximized_card.as_deref(), Some("summary"));
        assert_eq!(state.layout_revision(), 0);

        update(
            &mut state,
            CarSetupMessage::ToggleCardCollapsed("summary".to_owned()),
        );
        assert_eq!(state.maximized_card, None);
        assert_eq!(state.card_collapsed.get("summary"), Some(&true));
        assert_eq!(state.layout_revision(), 1);

        update(
            &mut state,
            CarSetupMessage::ToggleCardMaximized("summary".to_owned()),
        );
        assert_eq!(state.maximized_card.as_deref(), Some("summary"));
        assert_eq!(state.card_collapsed.get("summary"), Some(&false));
        assert_eq!(state.layout_revision(), 2);
    }

    #[test]
    fn cancelling_pointer_interactions_aborts_dragging() {
        let mut state = CarSetupState::default();
        report_card_layout(&mut state, "summary", card_bounds(0.0));
        begin_card_drag(&mut state, "summary");

        update(&mut state, CarSetupMessage::CancelPointerInteractions);

        assert_eq!(state.dragging_card, None);
        assert_eq!(state.drop_target, None);
        assert_eq!(state.drag_origin, None);
        assert_eq!(state.drag_cursor, None);
    }
}
