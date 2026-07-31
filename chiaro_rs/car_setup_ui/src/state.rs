//! State and update logic for the Car Setup screen.

use std::collections::BTreeMap;

use chiaro_actions::Action;
use chiaro_telemetry::Session;
use iced::{Point, Rectangle};

use crate::{
    interaction::{CardLayout, move_item_to, update_drop_target},
    layout::{CarSetupLayout, normalize_flags, normalize_order},
    setup_view,
};

#[derive(Debug, Default)]
pub struct CarSetupState {
    pub(super) cached_session_info_revision: Option<u64>,
    pub(super) cached_reference_session_info_revision: Option<u64>,
    pub(super) setup: setup_view::SetupViewData,
    pub(super) reference_setup: Option<setup_view::SetupViewData>,
    pub(super) layout_revision: u64,
    pub(super) card_order: Vec<String>,
    pub(super) card_collapsed: BTreeMap<String, bool>,
    pub(super) card_layouts: BTreeMap<String, CardLayout>,
    pub(super) maximized_card: Option<String>,
    pub(super) hovered_card: Option<String>,
    pub(super) dragging_card: Option<String>,
    pub(super) drop_target: Option<String>,
    pub(super) drag_origin: Option<Point>,
    pub(super) drag_cursor: Option<Point>,
    pub(super) drag_source_bounds: Option<Rectangle>,
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
                .map(|(key, value)| crate::CarSetupLayoutFlag {
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

    pub(super) fn clear_drag(&mut self) {
        self.dragging_card = None;
        self.drop_target = None;
        self.drag_origin = None;
        self.drag_cursor = None;
        self.drag_source_bounds = None;
    }

    fn reconcile_cards(&mut self) {
        let available = self.available_card_keys();
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

    pub(super) fn current_card_order(&self) -> Vec<String> {
        let available = self.available_card_keys();
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

    fn available_card_keys(&self) -> Vec<String> {
        self.setup.card_keys()
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
    OpenReferenceIbt,
    ClearReferenceIbt,
    ResetLayout,
}

impl CarSetupMessage {
    pub const fn resets_layout(&self) -> bool {
        matches!(self, Self::ResetLayout)
    }
}

pub fn activate(state: &mut CarSetupState, session: &Session, reference_session: Option<&Session>) {
    deactivate(state);
    refresh(state, session, reference_session);
}

pub fn deactivate(state: &mut CarSetupState) {
    state.clear_drag();
}

pub fn refresh(state: &mut CarSetupState, session: &Session, reference_session: Option<&Session>) {
    let revision = session.session_info_revision();
    if state.cached_session_info_revision != Some(revision) {
        state.cached_session_info_revision = Some(revision);
        state.setup = setup_data(session);
    }

    refresh_reference(state, reference_session);
    state.reconcile_cards();
}

fn refresh_reference(state: &mut CarSetupState, reference_session: Option<&Session>) {
    let reference_revision = reference_session.map(Session::session_info_revision);
    if state.cached_reference_session_info_revision != reference_revision {
        state.cached_reference_session_info_revision = reference_revision;
        state.reference_setup = reference_session.map(setup_data);
    }
}

pub fn reset_session(
    state: &mut CarSetupState,
    session: &Session,
    reference_session: Option<&Session>,
    active: bool,
) {
    state.cached_session_info_revision = None;
    state.setup = setup_view::SetupViewData::default();
    if active {
        refresh(state, session, reference_session);
    }
}

pub fn reset_reference(
    state: &mut CarSetupState,
    reference_session: Option<&Session>,
    active: bool,
) {
    state.cached_reference_session_info_revision = None;
    state.reference_setup = None;
    if active {
        refresh_reference(state, reference_session);
    }
    state.reconcile_cards();
}

fn setup_data(session: &Session) -> setup_view::SetupViewData {
    match session.session_info().map(chiaro_irsdk::SessionInfo::parse) {
        Some(Ok(document)) => setup_view::SetupViewData::from_document(&document),
        Some(Err(error)) => setup_view::SetupViewData::parse_error(error.to_string()),
        None => setup_view::SetupViewData::default(),
    }
}

pub fn update(state: &mut CarSetupState, message: CarSetupMessage) -> Option<Action> {
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
            None
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
            None
        },
        CarSetupMessage::SetHoveredCard(card) => {
            state.hovered_card = card;
            None
        },
        CarSetupMessage::BeginCardDrag(card) => {
            state.clear_drag();
            state.drag_source_bounds = state.card_layouts.get(&card).map(|layout| layout.bounds);
            state.dragging_card = Some(card);
            None
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
            None
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
            None
        },
        CarSetupMessage::DragCursor(position) => {
            if state.dragging_card.is_some() {
                state.drag_origin.get_or_insert(position);
                state.drag_cursor = Some(position);
                update_drop_target(state);
            }
            None
        },
        CarSetupMessage::CancelPointerInteractions => {
            state.clear_drag();
            None
        },
        CarSetupMessage::OpenReferenceIbt => Some(Action::OpenReferenceIbt),
        CarSetupMessage::ClearReferenceIbt => Some(Action::ClearReferenceIbt),
        CarSetupMessage::ResetLayout => {
            state.apply_layout(&CarSetupLayout::default());
            state.mark_layout_changed();
            None
        },
    }
}

#[cfg(test)]
mod tests {
    use chiaro_irsdk::SessionInfo;
    use chiaro_telemetry::Session;
    use iced::{Point, Rectangle};

    use super::{CarSetupMessage, CarSetupState, refresh, reset_reference, update};
    use crate::{CarSetupLayout, CarSetupLayoutFlag};

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

    fn session_with_setup(setup: &str) -> Session {
        let mut session = Session::default();
        record_setup(&mut session, setup, 1);
        session
    }

    fn record_setup(session: &mut Session, setup: &str, update_count: i32) {
        session.record_live_batch(
            std::iter::empty(),
            None,
            Some(SessionInfo {
                update_count,
                yaml: format!("DriverInfo:\n DriverCarPath: porsche9922cup\nCarSetup:\n{setup}"),
                raw: Vec::new(),
            }),
        );
    }

    #[test]
    fn loading_a_reference_never_changes_the_current_card_set() {
        let current = session_with_setup(
            "  TiresAero:\n   LeftFront:\n    ColdPressure: 155 kPa\n   RightFront:\n    ColdPressure: 156 kPa\n  Chassis:\n   LeftRear:\n    Camber: -3.5 deg\n   RightRear:\n    Camber: -3.4 deg\n   FrontARB: 3\n",
        );
        let reference = session_with_setup(
            "  TiresAero:\n   LF:\n    ColdPressure: 154 kPa\n   RF:\n    ColdPressure: 157 kPa\n  Chassis:\n   LR:\n    Camber: -3.6 deg\n   RR:\n    Camber: -3.3 deg\n",
        );
        let mut state = CarSetupState::default();

        refresh(&mut state, &current, None);
        let without_reference = state.current_card_order();
        refresh(&mut state, &current, Some(&reference));

        assert_eq!(state.current_card_order(), without_reference);
        assert!(
            without_reference
                .iter()
                .any(|key| key.ends_with("LeftFront"))
        );
        assert!(
            without_reference
                .iter()
                .any(|key| key.ends_with("RightFront"))
        );
        assert!(
            without_reference
                .iter()
                .any(|key| key.ends_with("LeftRear"))
        );
        assert!(
            without_reference
                .iter()
                .any(|key| key.ends_with("RightRear"))
        );
        assert!(
            without_reference
                .iter()
                .any(|key| key.ends_with("FrontARB"))
        );
    }

    #[test]
    fn resetting_a_reference_cannot_refresh_or_replace_current_cards() {
        let mut current = session_with_setup(
            "  Chassis:\n   LeftFront:\n    Camber: -3.5 deg\n   RightFront:\n    Camber: -3.4 deg\n   FrontARB: 3\n",
        );
        let reference = session_with_setup(
            "  Chassis:\n   LF:\n    Camber: -3.6 deg\n   RF:\n    Camber: -3.3 deg\n",
        );
        let mut state = CarSetupState::default();
        refresh(&mut state, &current, None);
        let before_reference = state.current_card_order();

        record_setup(&mut current, "  Chassis:\n   FrontARB: 4\n", 2);
        reset_reference(&mut state, Some(&reference), true);

        assert_eq!(state.current_card_order(), before_reference);
        assert!(state.reference_setup.is_some());
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
