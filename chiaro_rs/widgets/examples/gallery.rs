use chiaro_widgets::{
    BadgeVariant, ButtonSize, ButtonVariant, WindowControlKind, badge, button, callout, card,
    dialog, icon_button, icon_toggle_button, navigation_item, panel, surface, toggle_button,
    typography, window_control,
};
use iced::{
    Color, Element, Length, Theme,
    theme::Palette,
    widget::{column, container, row, scrollable, text, text_input},
};
use iced_fonts::{LUCIDE_FONT_BYTES, lucide};

const CARBON_DARK: Palette = Palette {
    background: Color::from_rgb8(0x16, 0x16, 0x16),
    text: Color::from_rgb8(0xF4, 0xF4, 0xF4),
    primary: Color::from_rgb8(0x45, 0x89, 0xFF),
    success: Color::from_rgb8(0x42, 0xBE, 0x65),
    warning: Color::from_rgb8(0xF1, 0xC2, 0x1B),
    danger: Color::from_rgb8(0xFA, 0x4D, 0x56),
};

fn main() -> iced::Result {
    iced::application(Gallery::new, Gallery::update, Gallery::view)
        .title("Chiaro Widget Gallery")
        .theme(Gallery::theme)
        .style(|_, theme| surface::application(theme))
        .font(LUCIDE_FONT_BYTES)
        .font(typography::SANS_REGULAR_BYTES)
        .font(typography::SANS_SEMIBOLD_BYTES)
        .font(typography::MONO_REGULAR_BYTES)
        .default_font(typography::SANS)
        .window(iced::window::Settings {
            size: iced::Size::new(1_000.0, 800.0),
            ..iced::window::Settings::default()
        })
        .antialiasing(true)
        .centered()
        .run()
}

#[derive(Debug)]
struct Gallery {
    toggle_selected: bool,
    navigation: Destination,
    last_action: &'static str,
    dialog_open: bool,
    profile_name: String,
}

impl Gallery {
    fn new() -> Self {
        Self {
            toggle_selected: true,
            navigation: Destination::Dashboard,
            last_action: "Nothing yet",
            dialog_open: false,
            profile_name: "Maverick".to_owned(),
        }
    }

    fn update(&mut self, message: Message) {
        match message {
            Message::Action(action) => self.last_action = action,
            Message::Toggle => {
                self.toggle_selected = !self.toggle_selected;
                self.last_action = "Toggle changed";
            },
            Message::Navigate(destination) => {
                self.navigation = destination;
                self.last_action = destination.label();
            },
            Message::WindowControl(action) => self.last_action = action,
            Message::OpenDialog => {
                self.dialog_open = true;
                self.last_action = "Dialog opened";
            },
            Message::CloseDialog => {
                self.dialog_open = false;
                self.last_action = "Dialog closed";
            },
            Message::SaveDialog => {
                self.dialog_open = false;
                self.last_action = "Profile saved";
            },
            Message::ProfileNameChanged(name) => self.profile_name = name,
        }
    }

    fn view(&self) -> Element<'_, Message> {
        let content = column![
            column![
                text("Chiaro Widget Gallery")
                    .size(32)
                    .font(typography::SANS_SEMIBOLD),
                text("共有Widgetの状態・寸法・装飾を単独で確認できます。").size(15),
                text(format!("Last action: {}", self.last_action)).size(13),
            ]
            .spacing(6),
            section("Button variants", button_variants()),
            section("Button sizes", button_sizes()),
            section("IconButton", icon_buttons()),
            section("IconToggleButton", icon_toggle_buttons()),
            section("ToggleButton", self.toggle_buttons()),
            section("NavigationItem", self.navigation_items()),
            section("Badge", badges()),
            section("Card / Panel / Callout", surfaces()),
            section("Dialog", self.dialog_trigger()),
            section("WindowControlButton", window_controls()),
        ]
        .spacing(28)
        .width(Length::Fill);

        let page = container(scrollable(
            container(content).padding(32).width(Length::Fill),
        ))
        .width(Length::Fill)
        .height(Length::Fill)
        .style(|theme| surface::content(theme, false));

        let body = column![
            text("Display name").size(14),
            text_input("Display name", &self.profile_name).on_input(Message::ProfileNameChanged),
        ]
        .spacing(8);
        let footer = row![
            button(text("Cancel"))
                .variant(ButtonVariant::Outline)
                .on_press(Message::CloseDialog),
            button(text("Save changes")).on_press(Message::SaveDialog),
        ]
        .spacing(8);

        dialog(page, body)
            .open(self.dialog_open)
            .title("Edit profile")
            .description("Update the name shown throughout the Chiaro application.")
            .footer(footer)
            .close_label("Close dialog")
            .on_close(Message::CloseDialog)
            .into()
    }

    fn theme(&self) -> Theme {
        Theme::custom("Carbon Dark", CARBON_DARK)
    }

    fn toggle_buttons(&self) -> Element<'_, Message> {
        row![
            labeled_widget(
                "Selected",
                toggle_button(lucide::check().size(16), self.toggle_selected)
                    .on_press(Message::Toggle),
            ),
            labeled_widget(
                "Unselected",
                toggle_button(lucide::x().size(16), !self.toggle_selected)
                    .on_press(Message::Toggle),
            ),
        ]
        .spacing(20)
        .into()
    }

    fn navigation_items(&self) -> Element<'_, Message> {
        row![
            container(
                navigation_item(lucide::layout_dashboard().size(18), "Dashboard")
                    .selected(self.navigation == Destination::Dashboard)
                    .on_press(Message::Navigate(Destination::Dashboard)),
            )
            .width(120),
            container(
                navigation_item(lucide::settings().size(18), "Settings")
                    .selected(self.navigation == Destination::Settings)
                    .on_press(Message::Navigate(Destination::Settings)),
            )
            .width(120),
        ]
        .spacing(12)
        .into()
    }

    fn dialog_trigger(&self) -> Element<'_, Message> {
        row![
            button(text("Open dialog"))
                .variant(ButtonVariant::Outline)
                .on_press(Message::OpenDialog),
            text(if self.dialog_open { "Open" } else { "Closed" }).size(13),
        ]
        .spacing(12)
        .align_y(iced::Alignment::Center)
        .into()
    }
}

#[derive(Debug, Clone)]
enum Message {
    Action(&'static str),
    Toggle,
    Navigate(Destination),
    WindowControl(&'static str),
    OpenDialog,
    CloseDialog,
    SaveDialog,
    ProfileNameChanged(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Destination {
    Dashboard,
    Settings,
}

impl Destination {
    const fn label(self) -> &'static str {
        match self {
            Self::Dashboard => "Dashboard selected",
            Self::Settings => "Settings selected",
        }
    }
}

fn section<'a>(
    title: &'static str,
    content: impl Into<Element<'a, Message>>,
) -> Element<'a, Message> {
    column![
        text(title).size(20).font(typography::SANS_SEMIBOLD),
        content.into(),
    ]
    .spacing(12)
    .into()
}

fn button_variants() -> Element<'static, Message> {
    row![
        variant_sample("Primary", ButtonVariant::Primary),
        variant_sample("Secondary", ButtonVariant::Secondary),
        variant_sample("Outline", ButtonVariant::Outline),
        variant_sample("Ghost", ButtonVariant::Ghost),
        variant_sample("Destructive", ButtonVariant::Destructive),
    ]
    .spacing(16)
    .into()
}

fn variant_sample(label: &'static str, variant: ButtonVariant) -> Element<'static, Message> {
    column![
        text(label).size(13),
        button(text("Enabled"))
            .variant(variant)
            .on_press(Message::Action(label)),
        button(text("Disabled")).variant(variant),
    ]
    .spacing(8)
    .into()
}

fn button_sizes() -> Element<'static, Message> {
    row![
        size_sample("Extra small", ButtonSize::ExtraSmall),
        size_sample("Small", ButtonSize::Small),
        size_sample("Medium", ButtonSize::Medium),
        size_sample("Large", ButtonSize::Large),
    ]
    .spacing(16)
    .align_y(iced::Alignment::End)
    .into()
}

fn size_sample(label: &'static str, size: ButtonSize) -> Element<'static, Message> {
    column![
        text(label).size(13),
        button(text(label))
            .size(size)
            .on_press(Message::Action(label)),
    ]
    .spacing(8)
    .into()
}

fn icon_buttons() -> Element<'static, Message> {
    row![
        labeled_widget(
            "Outline",
            icon_button(lucide::folder_open().size(17), "Open file")
                .variant(ButtonVariant::Outline)
                .on_press(Message::Action("Open file")),
        ),
        labeled_widget(
            "Ghost",
            icon_button(lucide::trash_two().size(17), "Delete")
                .variant(ButtonVariant::Ghost)
                .on_press(Message::Action("Delete")),
        ),
        labeled_widget(
            "Disabled",
            icon_button(lucide::save().size(17), "Disabled save").variant(ButtonVariant::Secondary),
        ),
    ]
    .spacing(20)
    .into()
}

fn icon_toggle_buttons() -> Element<'static, Message> {
    row![
        labeled_widget(
            "Selected",
            icon_toggle_button(lucide::layout_list().size(16), "Single column", true)
                .on_press(Message::Action("Single column")),
        ),
        labeled_widget(
            "Unselected",
            icon_toggle_button(lucide::layout_grid().size(16), "Two columns", false)
                .on_press(Message::Action("Two columns")),
        ),
        labeled_widget(
            "Disabled",
            icon_toggle_button(lucide::layout_grid().size(16), "Disabled layout", false),
        ),
    ]
    .spacing(20)
    .into()
}

fn labeled_widget<'a>(
    label: &'static str,
    widget: impl Into<Element<'a, Message>>,
) -> Element<'a, Message> {
    column![text(label).size(13), widget.into()]
        .spacing(8)
        .into()
}

fn badges() -> Element<'static, Message> {
    column![
        row![
            badge("Neutral").variant(BadgeVariant::Neutral),
            badge("Primary").variant(BadgeVariant::Primary),
            badge("Success").variant(BadgeVariant::Success),
            badge("Danger").variant(BadgeVariant::Danger),
        ]
        .spacing(12)
        .align_y(iced::Alignment::Center),
        row![
            badge("42.5%")
                .variant(BadgeVariant::Success)
                .progress(0.425)
                .meter_color(Color::from_rgb(0.12, 0.72, 0.38))
                .width(Length::Fixed(72.0)),
            badge("75.0%")
                .variant(BadgeVariant::Danger)
                .progress(0.75)
                .meter_color(Color::from_rgb(0.90, 0.24, 0.24))
                .width(Length::Fixed(72.0)),
            badge("-90.0°")
                .variant(BadgeVariant::Primary)
                .centered_progress(-0.5)
                .meter_color(Color::from_rgb(0.20, 0.72, 0.68))
                .width(Length::Fixed(72.0)),
            badge("90.0°")
                .variant(BadgeVariant::Primary)
                .centered_progress(0.5)
                .meter_color(Color::from_rgb(0.20, 0.72, 0.68))
                .width(Length::Fixed(72.0)),
        ]
        .spacing(12)
        .align_y(iced::Alignment::Center),
    ]
    .spacing(12)
    .into()
}

fn surfaces() -> Element<'static, Message> {
    row![
        card(surface_content(
            "Card",
            "Standalone content with border and elevation.",
        ))
        .padding(20)
        .width(Length::Fill),
        panel(surface_content(
            "Panel",
            "A compact section nested within a larger view.",
        ))
        .padding(20)
        .width(Length::Fill),
        callout(surface_content(
            "Callout",
            "Explanatory information or status belongs here.",
        ))
        .padding(20)
        .width(Length::Fill),
    ]
    .spacing(16)
    .into()
}

fn surface_content(title: &'static str, description: &'static str) -> Element<'static, Message> {
    column![
        text(title).size(17).font(typography::SANS_SEMIBOLD),
        text(description).size(13),
    ]
    .spacing(6)
    .into()
}

fn window_controls() -> Element<'static, Message> {
    row![
        window_control(
            WindowControlKind::Minimize,
            Message::WindowControl("Minimize clicked"),
        ),
        window_control(
            WindowControlKind::Maximize,
            Message::WindowControl("Maximize clicked"),
        ),
        window_control(
            WindowControlKind::Close,
            Message::WindowControl("Close clicked"),
        ),
    ]
    .spacing(8)
    .into()
}
