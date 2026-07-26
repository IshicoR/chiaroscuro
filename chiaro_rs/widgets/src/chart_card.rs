use std::borrow::Cow;

use iced::{
    Element,
    Length::{Fill, Fixed},
    Padding, mouse,
    widget::{column, container, mouse_area, row, text, tooltip},
};

use crate::{ButtonSize, ButtonVariant, card, icon_button, icon_tooltip_style, typography};
use chiaro_i18n::{Text, collapse_label, expand_label, tr};
use iced_fonts::lucide;

pub(crate) const CARD_PADDING: f32 = 10.0;
pub(crate) const CARD_TITLE_LEFT_PADDING: f32 = 4.0;
pub(crate) const CARD_CONTENT_SPACING: f32 = 8.0;
pub const CARD_HEADER_HEIGHT: f32 = 28.0;
pub(crate) const CARD_CONTENT_EDGE_INSET: f32 = 4.0;
pub(crate) const CARD_HEADER_HORIZONTAL_INSET: f32 = CARD_PADDING - CARD_CONTENT_EDGE_INSET;
pub(crate) const CARD_TITLE_SPACING: f32 = 7.0;

/// A screen card title with a decorative leading icon.
///
/// `label` stays separate from the visual icon so it can also be used for
/// accessible collapse and expand labels.
pub struct CardTitle<'a, Message> {
    label: Cow<'a, str>,
    icon: Element<'a, Message>,
    trailing: Option<Element<'a, Message>>,
}

impl<Message> std::fmt::Debug for CardTitle<'_, Message> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("CardTitle")
            .field("label", &self.label)
            .finish_non_exhaustive()
    }
}

impl<'a, Message: 'a> CardTitle<'a, Message> {
    pub fn new(label: impl Into<Cow<'a, str>>, icon: impl Into<Element<'a, Message>>) -> Self {
        Self {
            label: label.into(),
            // Lucide's optical center sits slightly below IBM Plex Sans JP's
            // title glyphs. Bottom padding raises the rendered glyph by one
            // logical pixel while keeping its layout box centered.
            icon: container(icon.into())
                .padding(Padding::ZERO.bottom(2))
                .into(),
            trailing: None,
        }
    }

    /// Adds non-interactive content immediately after the title label.
    pub fn with_trailing(mut self, content: impl Into<Element<'a, Message>>) -> Self {
        self.trailing = Some(content.into());
        self
    }

    pub(crate) fn label(&self) -> &str {
        self.label.as_ref()
    }

    pub(crate) fn into_parts(
        self,
    ) -> (
        Cow<'a, str>,
        Element<'a, Message>,
        Option<Element<'a, Message>>,
    ) {
        (self.label, self.icon, self.trailing)
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
        (lucide::minimize_two().size(16), tr(Text::RestoreChart))
    } else {
        (lucide::maximize_two().size(16), tr(Text::MaximizeChart))
    };
    let maximize = icon_button(maximize_icon, maximize_label)
        .variant(ButtonVariant::Ghost)
        .size(ButtonSize::IconSmall)
        .on_press(on_toggle_maximize.clone());
    let collapse = collapse_button(title.label(), collapsed, on_toggle_collapsed);
    let (label, icon, trailing) = title.into_parts();
    let mut title_content = row![icon, text(label).size(16).font(typography::SANS_SEMIBOLD)]
        .spacing(CARD_TITLE_SPACING)
        .align_y(iced::Alignment::Center);
    if let Some(trailing) = trailing {
        title_content = title_content.push(trailing);
    }

    let header = container(
        row![
            mouse_area(
                container(title_content,)
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

/// Builds the standard grab handle used by reorderable cards.
pub fn card_drag_handle<Message: Clone + 'static>(
    on_press: Message,
    interaction: mouse::Interaction,
) -> Element<'static, Message> {
    tooltip(
        mouse_area(
            container(lucide::grip_vertical().size(16)).padding(Padding {
                top: 5.0,
                right: 6.0,
                bottom: 7.0,
                left: 6.0,
            }),
        )
        .on_press(on_press)
        .interaction(interaction),
        container(text(tr(Text::DragToReorder)).size(12)).padding([4, 8]),
        tooltip::Position::Top,
    )
    .gap(4)
    .padding(0)
    .style(icon_tooltip_style)
    .into()
}

fn collapse_accessibility_label(title: &str, collapsed: bool) -> String {
    if collapsed {
        expand_label(title)
    } else {
        collapse_label(title)
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
