use iced::{
    Element,
    Length::{Fill, Fixed},
    Padding, mouse,
    widget::{column, container, mouse_area, row, text},
};

use super::chart_card::{
    CARD_CONTENT_EDGE_INSET, CARD_CONTENT_SPACING, CARD_HEADER_HEIGHT,
    CARD_HEADER_HORIZONTAL_INSET, CARD_PADDING, CARD_TITLE_LEFT_PADDING, CARD_TITLE_SPACING,
    CardTitle, collapse_button,
};
use chiaro_widgets::{ButtonSize, ButtonVariant, card, icon_button, surface, typography};
use iced_fonts::lucide;

pub(crate) fn pane_card<'a, Message: Clone + 'a>(
    title: CardTitle<'a, Message>,
    content: impl Into<Element<'a, Message>>,
    content_padding: f32,
    header_action: impl Into<Element<'a, Message>>,
    collapsed: bool,
    on_toggle_collapsed: Message,
    highlighted: bool,
) -> Element<'a, Message> {
    pane_card_inner(
        title,
        content,
        content_padding,
        header_action,
        collapsed,
        on_toggle_collapsed,
        None,
        highlighted,
    )
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn pane_card_with_maximize<'a, Message: Clone + 'a>(
    title: CardTitle<'a, Message>,
    content: impl Into<Element<'a, Message>>,
    content_padding: f32,
    header_action: impl Into<Element<'a, Message>>,
    maximized: bool,
    collapsed: bool,
    on_toggle_maximized: Message,
    on_toggle_collapsed: Message,
    highlighted: bool,
) -> Element<'a, Message> {
    pane_card_inner(
        title,
        content,
        content_padding,
        header_action,
        collapsed,
        on_toggle_collapsed,
        Some((maximized, on_toggle_maximized)),
        highlighted,
    )
}

#[allow(clippy::too_many_arguments)]
fn pane_card_inner<'a, Message: Clone + 'a>(
    title: CardTitle<'a, Message>,
    content: impl Into<Element<'a, Message>>,
    content_padding: f32,
    header_action: impl Into<Element<'a, Message>>,
    collapsed: bool,
    on_toggle_collapsed: Message,
    maximize: Option<(bool, Message)>,
    highlighted: bool,
) -> Element<'a, Message> {
    let collapse = collapse_button(title.label(), collapsed, on_toggle_collapsed);
    let (label, icon) = title.into_parts();
    let title = container(
        row![icon, text(label).size(16).font(typography::SANS_SEMIBOLD)]
            .spacing(CARD_TITLE_SPACING)
            .align_y(iced::Alignment::Center),
    )
    .padding(iced::Padding::default().left(CARD_TITLE_LEFT_PADDING))
    .width(Fill)
    .height(Fill)
    .align_y(iced::alignment::Vertical::Center);
    let mut actions = row![collapse].spacing(4).align_y(iced::Alignment::Center);
    let title: Element<'_, Message> = if let Some((maximized, on_toggle_maximized)) = maximize {
        let (maximize_icon, maximize_label) = if maximized {
            (lucide::minimize_two().size(16), "Restore card")
        } else {
            (lucide::maximize_two().size(16), "Maximize card")
        };
        actions = actions.push(
            icon_button(maximize_icon, maximize_label)
                .variant(ButtonVariant::Ghost)
                .size(ButtonSize::IconSmall)
                .on_press(on_toggle_maximized.clone()),
        );
        mouse_area(title)
            .on_double_click(on_toggle_maximized)
            .interaction(mouse::Interaction::Pointer)
            .into()
    } else {
        title.into()
    };
    actions = actions.push(header_action.into());
    let header = container(
        row![title, actions,]
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
    let mut card_content = column![header].width(Fill);
    if !collapsed {
        card_content = card_content
            .push(
                container(content.into())
                    .padding(content_padding)
                    .width(Fill)
                    .clip(true)
                    .style(surface::card_content),
            )
            .spacing(CARD_CONTENT_SPACING);
    }

    card(card_content)
        .padding(Padding {
            top: CARD_PADDING,
            right: CARD_CONTENT_EDGE_INSET,
            bottom: CARD_CONTENT_EDGE_INSET,
            left: CARD_CONTENT_EDGE_INSET,
        })
        .width(Fill)
        .highlighted(highlighted)
        .into()
}

#[cfg(test)]
mod tests {
    use std::{cell::Cell, rc::Rc};

    use super::{CARD_PADDING, pane_card};
    use chiaro_widgets::surface;
    use iced::{Element, Theme, widget::Space};
    use iced_fonts::lucide;

    use crate::chart_card::CardTitle;

    struct TrackedContent(Rc<Cell<bool>>);

    impl<'a> From<TrackedContent> for Element<'a, ()> {
        fn from(content: TrackedContent) -> Self {
            content.0.set(true);
            Space::new().into()
        }
    }

    #[test]
    fn content_uses_the_chart_plot_surface_without_a_border() {
        let theme = Theme::Dark;
        let palette = theme.extended_palette();
        let style = surface::card_content(&theme);

        assert_eq!(style.background, Some(palette.background.base.color.into()));
        assert_eq!(style.text_color, Some(palette.background.base.text));
        assert_eq!(style.border.width, 0.0);
        assert_eq!(style.border.radius, 8.0.into());
    }

    #[test]
    fn collapsed_pane_omits_its_inner_content() {
        let converted = Rc::new(Cell::new(false));

        let _ = pane_card(
            CardTitle::new("Session", lucide::clipboard_list().size(16)),
            TrackedContent(converted.clone()),
            CARD_PADDING,
            Space::new(),
            true,
            (),
            false,
        );

        assert!(!converted.get());
    }

    #[test]
    fn expanded_pane_builds_its_inner_content() {
        let converted = Rc::new(Cell::new(false));

        let _ = pane_card(
            CardTitle::new("Session", lucide::clipboard_list().size(16)),
            TrackedContent(converted.clone()),
            CARD_PADDING,
            Space::new(),
            false,
            (),
            false,
        );

        assert!(converted.get());
    }
}
