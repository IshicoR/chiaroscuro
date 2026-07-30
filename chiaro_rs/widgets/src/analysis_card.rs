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
use crate::{ButtonSize, ButtonVariant, card, icon_button, surface, typography};
use chiaro_i18n::{Text, tr};
use iced_fonts::lucide;

#[allow(clippy::too_many_arguments)]
pub fn pane_card<'a, Message: Clone + 'a>(
    title: CardTitle<'a, Message>,
    content: impl Into<Element<'a, Message>>,
    content_padding: f32,
    drag: Option<(Message, mouse::Interaction)>,
    actions_visible: bool,
    collapsed: bool,
    on_toggle_collapsed: Message,
    highlighted: bool,
) -> Element<'a, Message> {
    pane_card_inner(
        title,
        content,
        content_padding,
        drag,
        actions_visible,
        collapsed,
        on_toggle_collapsed,
        None,
        highlighted,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn pane_card_with_maximize<'a, Message: Clone + 'a>(
    title: CardTitle<'a, Message>,
    content: impl Into<Element<'a, Message>>,
    content_padding: f32,
    drag: Option<(Message, mouse::Interaction)>,
    actions_visible: bool,
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
        drag,
        actions_visible,
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
    drag: Option<(Message, mouse::Interaction)>,
    actions_visible: bool,
    collapsed: bool,
    on_toggle_collapsed: Message,
    maximize: Option<(bool, Message)>,
    highlighted: bool,
) -> Element<'a, Message> {
    let title_label = title.label().to_owned();
    let (label, icon, trailing) = title.into_parts();
    let mut title_content = row![icon, text(label).size(16).font(typography::SANS_SEMIBOLD)]
        .spacing(CARD_TITLE_SPACING)
        .align_y(iced::Alignment::Center);
    if let Some(trailing) = trailing {
        title_content = title_content.push(trailing);
    }
    let title = container(title_content)
        .padding(iced::Padding::default().left(CARD_TITLE_LEFT_PADDING))
        .width(Fill)
        .height(Fill)
        .align_y(iced::alignment::Vertical::Center);
    let mut actions = row![].spacing(4).align_y(iced::Alignment::Center);
    let maximize_message = maximize.as_ref().map(|(_, message)| message.clone());
    if actions_visible {
        actions = actions.push(collapse_button(
            &title_label,
            collapsed,
            on_toggle_collapsed,
            ButtonSize::IconSmall,
        ));
    }
    if actions_visible && let Some((maximized, on_toggle_maximized)) = maximize.as_ref() {
        let (maximize_icon, maximize_label) = if *maximized {
            (lucide::minimize_two().size(16), tr(Text::RestoreCard))
        } else {
            (lucide::maximize_two().size(16), tr(Text::MaximizeCard))
        };
        actions = actions.push(
            icon_button(maximize_icon, maximize_label)
                .variant(ButtonVariant::Ghost)
                .size(ButtonSize::IconSmall)
                .on_press(on_toggle_maximized.clone()),
        );
    }
    let header_content = row![title, actions]
        .spacing(4)
        .width(Fill)
        .height(Fixed(CARD_HEADER_HEIGHT))
        .align_y(iced::Alignment::Center);
    let mut header = mouse_area(header_content);
    if let Some(on_double_click) = maximize_message {
        header = header.on_double_click(on_double_click);
    }
    if let Some((on_press, interaction)) = drag {
        header = header.on_press(on_press).interaction(interaction);
    }
    let header = container(header)
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
    use crate::surface;
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
            None,
            true,
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
            None,
            true,
            false,
            (),
            false,
        );

        assert!(converted.get());
    }
}
