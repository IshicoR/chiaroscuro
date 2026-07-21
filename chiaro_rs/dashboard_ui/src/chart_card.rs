use std::borrow::Cow;

use iced::{
    Element,
    Length::{Fill, Fixed},
    Padding, mouse,
    widget::{column, container, mouse_area, row, text},
};

use chiaro_widgets::{ButtonSize, ButtonVariant, card, icon_button, typography};
use iced_fonts::lucide;

pub(crate) const CARD_PADDING: f32 = 10.0;
pub(crate) const CARD_TITLE_LEFT_PADDING: f32 = 4.0;
pub(crate) const CARD_CONTENT_SPACING: f32 = 8.0;
pub(crate) const CARD_HEADER_HEIGHT: f32 = 28.0;
pub(crate) const CARD_CONTENT_EDGE_INSET: f32 = 4.0;
pub(crate) const CARD_HEADER_HORIZONTAL_INSET: f32 = CARD_PADDING - CARD_CONTENT_EDGE_INSET;
pub(super) const CARD_TITLE_SPACING: f32 = 7.0;
const CARD_TITLE_ICON_TOP_PADDING: f32 = 2.0;

/// A dashboard card title with a decorative leading icon.
///
/// `label` stays separate from the visual icon so it can also be used for
/// accessible collapse and expand labels.
pub(crate) struct CardTitle<'a, Message> {
    label: Cow<'a, str>,
    icon: Element<'a, Message>,
}

impl<'a, Message: 'a> CardTitle<'a, Message> {
    pub(crate) fn new(
        label: impl Into<Cow<'a, str>>,
        icon: impl Into<Element<'a, Message>>,
    ) -> Self {
        Self {
            label: label.into(),
            // Lucide's glyph box sits slightly above the title font's visible
            // center. Asymmetric padding lowers it by one logical pixel once
            // the wrapper itself is vertically centered in the title row.
            icon: container(icon.into())
                .padding(Padding::ZERO.top(CARD_TITLE_ICON_TOP_PADDING))
                .into(),
        }
    }

    pub(crate) fn label(&self) -> &str {
        self.label.as_ref()
    }

    pub(super) fn into_parts(self) -> (Cow<'a, str>, Element<'a, Message>) {
        (self.label, self.icon)
    }
}

// This declarative view constructor keeps the independently typed header,
// body, state flags, and interaction messages explicit at its call sites.
#[allow(clippy::too_many_arguments)]
pub fn chart_card<'a, Message: Clone + 'a>(
    title: CardTitle<'a, Message>,
    chart: impl Into<Element<'a, Message>>,
    header_action: impl Into<Element<'a, Message>>,
    maximized: bool,
    collapsed: bool,
    on_toggle_maximize: Message,
    on_toggle_collapsed: Message,
    highlighted: bool,
    lift: f32,
) -> Element<'a, Message> {
    let (maximize_icon, maximize_label) = if maximized {
        (lucide::minimize_two().size(16), "Restore chart")
    } else {
        (lucide::maximize_two().size(16), "Maximize chart")
    };
    let maximize = icon_button(maximize_icon, maximize_label)
        .variant(ButtonVariant::Ghost)
        .size(ButtonSize::IconSmall)
        .on_press(on_toggle_maximize.clone());
    let collapse = collapse_button(title.label(), collapsed, on_toggle_collapsed);
    let (label, icon) = title.into_parts();

    let header = container(
        row![
            mouse_area(
                container(
                    row![icon, text(label).size(16).font(typography::SANS_SEMIBOLD)]
                        .spacing(CARD_TITLE_SPACING)
                        .align_y(iced::Alignment::Center),
                )
                .padding(iced::Padding::default().left(CARD_TITLE_LEFT_PADDING))
                .width(Fill)
                .height(Fill)
                .align_y(iced::alignment::Vertical::Center),
            )
            .on_double_click(on_toggle_maximize)
            .interaction(mouse::Interaction::Pointer),
            collapse,
            maximize,
            header_action.into(),
        ]
        .spacing(4)
        .width(Fill)
        .height(Fixed(CARD_HEADER_HEIGHT))
        .align_y(iced::Alignment::Center),
    )
    .padding(Padding {
        left: CARD_HEADER_HORIZONTAL_INSET,
        right: CARD_HEADER_HORIZONTAL_INSET,
        ..Padding::ZERO
    })
    .width(Fill);
    let mut content = column![header].width(Fill);
    if !collapsed {
        content = content
            .push(container(chart.into()).width(Fill).height(Fill))
            .spacing(CARD_CONTENT_SPACING)
            .height(Fill);
    }

    let mut card = card(content)
        .padding(Padding {
            top: CARD_PADDING,
            right: CARD_CONTENT_EDGE_INSET,
            bottom: CARD_CONTENT_EDGE_INSET,
            left: CARD_CONTENT_EDGE_INSET,
        })
        .width(Fill)
        .highlighted(highlighted)
        .lift(lift);
    if !collapsed {
        card = card.height(Fill);
    }

    card.into()
}

pub(crate) fn collapse_button<'a, Message: Clone + 'a>(
    title: &str,
    collapsed: bool,
    on_toggle: Message,
) -> Element<'a, Message> {
    let icon = if collapsed {
        lucide::chevron_down().size(16)
    } else {
        lucide::chevron_up().size(16)
    };

    icon_button(icon, collapse_accessibility_label(title, collapsed))
        .variant(ButtonVariant::Ghost)
        .size(ButtonSize::IconSmall)
        .on_press(on_toggle)
        .into()
}

fn collapse_accessibility_label(title: &str, collapsed: bool) -> String {
    if collapsed {
        format!("Expand {title}")
    } else {
        format!("Collapse {title}")
    }
}

#[cfg(test)]
mod tests {
    use std::{cell::Cell, rc::Rc};

    use iced::{Element, Length, widget::Space};
    use iced_fonts::lucide;

    use super::{CardTitle, chart_card, collapse_accessibility_label};

    struct TrackedContent(Rc<Cell<bool>>);

    impl<'a> From<TrackedContent> for Element<'a, ()> {
        fn from(content: TrackedContent) -> Self {
            content.0.set(true);
            Space::new().into()
        }
    }

    #[test]
    fn collapsed_chart_is_header_sized_and_does_not_build_its_body() {
        let converted = Rc::new(Cell::new(false));
        let card = chart_card(
            CardTitle::new("Speed", lucide::gauge().size(16)),
            TrackedContent(converted.clone()),
            Space::new(),
            false,
            true,
            (),
            (),
            false,
            0.0,
        );

        assert!(!converted.get());
        assert_eq!(card.as_widget().size().height, Length::Shrink);
    }

    #[test]
    fn expanded_chart_builds_its_body_and_fills_the_assigned_height() {
        let converted = Rc::new(Cell::new(false));
        let card = chart_card(
            CardTitle::new("Speed", lucide::gauge().size(16)),
            TrackedContent(converted.clone()),
            Space::new(),
            false,
            false,
            (),
            (),
            false,
            0.0,
        );

        assert!(converted.get());
        assert_eq!(card.as_widget().size().height, Length::Fill);
    }

    #[test]
    fn card_title_keeps_a_plain_label_for_accessibility() {
        let title: CardTitle<'_, ()> = CardTitle::new("Speed", lucide::gauge().size(16));

        assert_eq!(title.label(), "Speed");
    }

    #[test]
    fn collapse_accessibility_labels_remain_based_on_the_plain_title() {
        assert_eq!(
            collapse_accessibility_label("Speed", false),
            "Collapse Speed"
        );
        assert_eq!(collapse_accessibility_label("Speed", true), "Expand Speed");
    }
}
