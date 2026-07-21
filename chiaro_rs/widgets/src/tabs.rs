//! Flat tabs for switching between peer views on the same screen.

use std::fmt;

use iced::{
    Background, Border, Color, Element, Event, Length, Padding, Rectangle, Shadow, Size, Theme,
    Vector,
    advanced::{
        Clipboard, Layout, Renderer as _, Shell, Widget, layout, mouse, overlay, renderer,
        widget::{Operation, Tree},
    },
    alignment::Vertical,
    widget::{Row, Text, button as iced_button, container, row, text},
};

/// Height of the title-bar row that owns application tabs.
pub const BAR_HEIGHT: f32 = 40.0;
/// Height of a tab inside the title bar.
pub const HEIGHT: f32 = 36.0;
const HORIZONTAL_PADDING: u16 = 16;
const CONTENT_SPACING: f32 = 8.0;
const ICON_TOP_PADDING: f32 = 2.0;
const LABEL_SIZE: u32 = 14;
const CORNER_RADIUS: f32 = 8.0;
const FLARE_RADIUS: f32 = 8.0;

/// A single destination in a [`Tabs`] bar.
#[must_use]
pub struct Tab<'a, Message> {
    label: Text<'a>,
    icon: Option<Element<'a, Message>>,
    selected: bool,
    on_press: Message,
    width: Length,
}

impl<Message> fmt::Debug for Tab<'_, Message> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Tab")
            .field("has_icon", &self.icon.is_some())
            .field("selected", &self.selected)
            .field("width", &self.width)
            .finish_non_exhaustive()
    }
}

impl<'a, Message: 'a> Tab<'a, Message> {
    /// Creates a tab that publishes `on_press` when selected.
    pub fn new(label: impl text::IntoFragment<'a>, selected: bool, on_press: Message) -> Self {
        Self {
            label: text(label),
            icon: None,
            selected,
            on_press,
            width: Length::Shrink,
        }
    }

    /// Adds a decorative icon before the tab label.
    pub fn icon(mut self, icon: impl Into<Element<'a, Message>>) -> Self {
        self.icon = Some(
            container(icon.into())
                .padding(Padding::ZERO.top(ICON_TOP_PADDING))
                .into(),
        );
        self
    }

    /// Overrides the tab width.
    ///
    /// Use [`Length::Fill`] on every tab when all destinations should share
    /// the available bar width equally.
    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }
}

impl<'a, Message: Clone + 'a> Tab<'a, Message> {
    /// Builds the underlying Iced button.
    pub fn build(self) -> Element<'a, Message> {
        let selected = self.selected;
        let content_width = tab_content_width(self.width);
        let mut content = row![].spacing(CONTENT_SPACING).align_y(Vertical::Center);
        if let Some(icon) = self.icon {
            content = content.push(icon);
        }
        let content = content.push(self.label.size(LABEL_SIZE));
        let label = container(content)
            .width(content_width)
            .height(Length::Fill)
            .padding([0, HORIZONTAL_PADDING])
            .align_y(Vertical::Center);

        let button = iced_button(label)
            .width(self.width)
            .height(HEIGHT)
            .padding(0)
            .on_press(self.on_press)
            .style(move |theme, status| style(theme, status, selected));

        Element::new(TabSurface {
            content: button.into(),
            selected,
        })
    }
}

fn tab_content_width(tab_width: Length) -> Length {
    match tab_width {
        Length::Shrink => Length::Shrink,
        _ => Length::Fill,
    }
}

impl<'a, Message: Clone + 'a> From<Tab<'a, Message>> for Element<'a, Message> {
    fn from(tab: Tab<'a, Message>) -> Self {
        tab.build()
    }
}

struct TabSurface<'a, Message> {
    content: Element<'a, Message>,
    selected: bool,
}

impl<Message> Widget<Message, Theme, iced::Renderer> for TabSurface<'_, Message> {
    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.content)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(std::slice::from_ref(&self.content));
    }

    fn size(&self) -> Size<Length> {
        self.content.as_widget().size()
    }

    fn size_hint(&self) -> Size<Length> {
        self.content.as_widget().size_hint()
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &iced::Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        self.content
            .as_widget_mut()
            .layout(&mut tree.children[0], renderer, limits)
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &iced::Renderer,
        operation: &mut dyn Operation,
    ) {
        self.content
            .as_widget_mut()
            .operate(&mut tree.children[0], layout, renderer, operation);
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &iced::Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        self.content.as_widget_mut().update(
            &mut tree.children[0],
            event,
            layout,
            cursor,
            renderer,
            clipboard,
            shell,
            viewport,
        );
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut iced::Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        if self.selected {
            draw_flares(renderer, theme, layout.bounds());
        }

        self.content.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            style,
            layout,
            cursor,
            viewport,
        );
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &iced::Renderer,
    ) -> mouse::Interaction {
        self.content.as_widget().mouse_interaction(
            &tree.children[0],
            layout,
            cursor,
            viewport,
            renderer,
        )
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &iced::Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, iced::Renderer>> {
        self.content.as_widget_mut().overlay(
            &mut tree.children[0],
            layout,
            renderer,
            viewport,
            translation,
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FlareSide {
    Left,
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct FlareGeometry {
    wing: Rectangle,
    cutout: Rectangle,
}

fn flare_geometry(bounds: Rectangle, side: FlareSide) -> FlareGeometry {
    let bottom = bounds.y + bounds.height;
    let wing_x = match side {
        FlareSide::Left => bounds.x - FLARE_RADIUS,
        FlareSide::Right => bounds.x + bounds.width,
    };
    let cutout_x = match side {
        FlareSide::Left => bounds.x - FLARE_RADIUS * 2.0,
        FlareSide::Right => bounds.x + bounds.width,
    };

    FlareGeometry {
        wing: Rectangle {
            x: wing_x,
            y: bottom - FLARE_RADIUS,
            width: FLARE_RADIUS,
            height: FLARE_RADIUS,
        },
        cutout: Rectangle {
            x: cutout_x,
            y: bottom - FLARE_RADIUS * 2.0,
            width: FLARE_RADIUS * 2.0,
            height: FLARE_RADIUS * 2.0,
        },
    }
}

fn draw_flares(renderer: &mut iced::Renderer, theme: &Theme, bounds: Rectangle) {
    let palette = theme.extended_palette();
    let tab_color = palette.background.base.color;
    let bar_color = palette.background.weaker.color;

    for side in [FlareSide::Left, FlareSide::Right] {
        let geometry = flare_geometry(bounds, side);
        renderer.fill_quad(
            renderer::Quad {
                bounds: geometry.wing,
                border: Border::default(),
                shadow: Shadow::default(),
                snap: true,
            },
            tab_color,
        );
        renderer.fill_quad(
            renderer::Quad {
                bounds: geometry.cutout,
                border: Border {
                    radius: FLARE_RADIUS.into(),
                    ..Border::default()
                },
                shadow: Shadow::default(),
                snap: true,
            },
            bar_color,
        );
    }
}

/// A horizontal collection of peer-view tabs.
#[must_use]
pub struct Tabs<'a, Message> {
    tabs: Vec<Tab<'a, Message>>,
    width: Length,
}

impl<Message> fmt::Debug for Tabs<'_, Message> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Tabs")
            .field("len", &self.tabs.len())
            .field("width", &self.width)
            .finish()
    }
}

impl<'a, Message> Tabs<'a, Message> {
    /// Creates a tab bar from any number of tabs.
    pub fn new(tabs: impl IntoIterator<Item = Tab<'a, Message>>) -> Self {
        Self {
            tabs: tabs.into_iter().collect(),
            width: Length::Fill,
        }
    }

    /// Overrides the width of the complete tab bar.
    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }
}

impl<'a, Message: Clone + 'a> Tabs<'a, Message> {
    /// Builds the underlying Iced row.
    pub fn build(self) -> Row<'a, Message> {
        row(self.tabs.into_iter().map(Element::from)).width(self.width)
    }
}

impl<'a, Message: Clone + 'a> From<Tabs<'a, Message>> for Element<'a, Message> {
    fn from(tabs: Tabs<'a, Message>) -> Self {
        tabs.build().into()
    }
}

/// Creates a single flat tab.
pub fn tab<'a, Message: 'a>(
    label: impl text::IntoFragment<'a>,
    selected: bool,
    on_press: Message,
) -> Tab<'a, Message> {
    Tab::new(label, selected, on_press)
}

/// Creates a horizontal tab bar.
pub fn tabs<'a, Message>(items: impl IntoIterator<Item = Tab<'a, Message>>) -> Tabs<'a, Message> {
    Tabs::new(items)
}

fn style(theme: &Theme, status: iced_button::Status, selected: bool) -> iced_button::Style {
    let palette = theme.extended_palette();
    let (background, text_color) = if selected {
        match status {
            iced_button::Status::Active => (
                Some(palette.background.base.color),
                palette.background.base.text,
            ),
            iced_button::Status::Hovered => (
                Some(palette.background.base.color),
                palette.background.base.text,
            ),
            iced_button::Status::Pressed => (
                Some(palette.background.base.color),
                palette.background.base.text,
            ),
            iced_button::Status::Disabled => (
                Some(with_alpha(palette.background.base.color, 0.55)),
                with_alpha(palette.background.base.text, 0.4),
            ),
        }
    } else {
        match status {
            iced_button::Status::Active => (None, palette.background.base.text),
            iced_button::Status::Hovered => (
                Some(palette.background.weakest.color),
                palette.background.weakest.text,
            ),
            iced_button::Status::Pressed => (
                Some(palette.background.weak.color),
                palette.background.weak.text,
            ),
            iced_button::Status::Disabled => (None, with_alpha(palette.background.base.text, 0.4)),
        }
    };

    iced_button::Style {
        background: background.map(Background::Color),
        text_color,
        border: Border {
            radius: tab_radius(),
            ..Border::default()
        },
        ..iced_button::Style::default()
    }
}

fn tab_radius() -> iced::border::Radius {
    iced::border::Radius {
        top_left: CORNER_RADIUS,
        top_right: CORNER_RADIUS,
        ..iced::border::Radius::default()
    }
}

const fn with_alpha(color: Color, alpha: f32) -> Color {
    Color { a: alpha, ..color }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tab_bar_accepts_more_than_two_destinations() {
        let tabs = Tabs::new([
            Tab::new("Telemetry", true, 0_u8),
            Tab::new("Car setup", false, 1),
            Tab::new("Notes", false, 2),
        ]);

        assert_eq!(tabs.tabs.len(), 3);
        assert_eq!(tabs.width, Length::Fill);
    }

    #[test]
    fn shrink_tabs_measure_the_label_instead_of_collapsing_fill_content() {
        assert_eq!(tab_content_width(Length::Shrink), Length::Shrink);
        assert_eq!(tab_content_width(Length::Fixed(120.0)), Length::Fill);
        assert_eq!(tab_content_width(Length::Fill), Length::Fill);
    }

    #[test]
    fn tabs_accept_a_leading_icon_and_leave_room_above_the_surface() {
        let tab = Tab::new("Telemetry", true, ()).icon(text("icon"));

        assert!(tab.icon.is_some());
        assert_eq!(BAR_HEIGHT - HEIGHT, 4.0);
        assert_eq!(ICON_TOP_PADDING, 2.0);
    }

    #[test]
    fn selected_tab_merges_with_the_content_surface() {
        let theme = Theme::Dark;
        let palette = theme.extended_palette();
        let tab = style(&theme, iced_button::Status::Active, true);
        let pressed = style(&theme, iced_button::Status::Pressed, true);

        assert_eq!(
            tab.background,
            Some(Background::Color(palette.background.base.color))
        );
        assert_eq!(tab.text_color, palette.background.base.text);
        assert_eq!(tab.border.width, 0.0);
        assert_eq!(tab.border.radius, tab_radius());
        assert_eq!(tab.border.radius.bottom_left, 0.0);
        assert_eq!(tab.border.radius.bottom_right, 0.0);
        assert_eq!(pressed.background, tab.background);
    }

    #[test]
    fn selected_tab_flares_extend_past_both_bottom_edges() {
        let bounds = Rectangle {
            x: 100.0,
            y: 4.0,
            width: 120.0,
            height: HEIGHT,
        };
        let left = flare_geometry(bounds, FlareSide::Left);
        let right = flare_geometry(bounds, FlareSide::Right);

        assert_eq!(FLARE_RADIUS, CORNER_RADIUS);
        assert_eq!(left.wing.x, bounds.x - FLARE_RADIUS);
        assert_eq!(left.wing.y + left.wing.height, bounds.y + bounds.height);
        assert_eq!(left.cutout.x + left.cutout.width, bounds.x);
        assert_eq!(right.wing.x, bounds.x + bounds.width);
        assert_eq!(right.cutout.x, bounds.x + bounds.width);
        assert_eq!(right.wing.y + right.wing.height, bounds.y + bounds.height);
    }

    #[test]
    fn unselected_tab_is_flat_and_only_adds_a_surface_on_interaction() {
        let theme = Theme::Dark;
        let palette = theme.extended_palette();
        let active = style(&theme, iced_button::Status::Active, false);
        let hovered = style(&theme, iced_button::Status::Hovered, false);

        assert!(active.background.is_none());
        assert_eq!(
            hovered.background,
            Some(Background::Color(palette.background.weakest.color))
        );
        assert_eq!(active.border.width, 0.0);
        assert_eq!(active.border.radius, tab_radius());
    }
}
