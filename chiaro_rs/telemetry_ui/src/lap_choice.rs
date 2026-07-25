use chiaro_widgets::typography;
use iced::{
    Background, Border, Color, Element, Length, Theme,
    alignment::{Horizontal, Vertical},
    widget::{Space, button, column, container, row, rule, scrollable, text},
};
use iced_fonts::lucide;

const VISIBLE_ROWS: usize = 6;
const ROW_CONTENT_HEIGHT: f32 = 32.0;
const SEPARATOR_WIDTH: f32 = 1.0;
const ROW_HEIGHT: f32 = ROW_CONTENT_HEIGHT + SEPARATOR_WIDTH;
const LIST_BOTTOM_INSET: f32 = 8.0;
const CELL_HORIZONTAL_INSET: f32 = 8.0;
const NUMBER_CELL_WIDTH: f32 = 44.0;
const FUEL_CELL_WIDTH: f32 = 64.0;
const MARKER_CELL_WIDTH: f32 = 24.0;

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct LapChoice {
    index: usize,
    number: i32,
    duration_ms: i32,
    fuel_litres: Option<f32>,
    complete: bool,
}

impl LapChoice {
    pub(crate) const fn new(
        index: usize,
        number: i32,
        duration_ms: i32,
        fuel_litres: Option<f32>,
        complete: bool,
    ) -> Self {
        Self {
            index,
            number,
            duration_ms,
            fuel_litres,
            complete,
        }
    }
}

pub(crate) fn lap_choice_list<'a, Message, OnSelect>(
    choices: &'a [LapChoice],
    selected_index: Option<usize>,
    on_select: OnSelect,
    show_fuel: bool,
) -> Element<'a, Message>
where
    Message: Clone + 'a,
    OnSelect: Fn(usize) -> Message,
{
    if choices.is_empty() {
        return Space::new().height(Length::Fixed(0.0)).into();
    }

    let laps = choices.iter().fold(column![], |laps, choice| {
        let selected = selected_index == Some(choice.index);
        let duration = format_choice_duration(choice);
        let marker: Element<'_, Message> = if selected {
            lucide::check().size(15).into()
        } else {
            Space::new().width(Length::Fixed(15.0)).into()
        };

        let number = container(text(choice.number).size(14).font(typography::MONO))
            .padding([0.0, CELL_HORIZONTAL_INSET])
            .width(Length::Fixed(NUMBER_CELL_WIDTH))
            .height(Length::Fill)
            .align_y(Vertical::Center);
        let duration = container(
            text(duration)
                .size(14)
                .font(typography::MONO)
                .wrapping(iced::widget::text::Wrapping::None),
        )
        .padding([0.0, CELL_HORIZONTAL_INSET])
        .width(Length::Fill)
        .height(Length::Fill)
        .align_y(Vertical::Center)
        .clip(true);

        let mut content = row![
            number,
            rule::vertical(SEPARATOR_WIDTH).style(separator_style),
            duration,
        ]
        .width(Length::Fill)
        .height(Length::Fill)
        .align_y(iced::Alignment::Center);
        if show_fuel {
            let fuel = container(
                text(format_lap_fuel(choice.fuel_litres))
                    .size(13)
                    .font(typography::MONO)
                    .align_x(Horizontal::Right),
            )
            .padding([0.0, CELL_HORIZONTAL_INSET])
            .width(Length::Fixed(FUEL_CELL_WIDTH))
            .height(Length::Fill)
            .align_x(Horizontal::Right)
            .align_y(Vertical::Center);
            content = content
                .push(rule::vertical(SEPARATOR_WIDTH).style(separator_style))
                .push(fuel);
        }
        let marker = container(marker)
            .width(Length::Fixed(MARKER_CELL_WIDTH))
            .height(Length::Fill)
            .align_x(Horizontal::Center)
            .align_y(Vertical::Center);
        let content = content
            .push(rule::vertical(SEPARATOR_WIDTH).style(separator_style))
            .push(marker);

        laps.push(
            button(content)
                .padding(0)
                .height(Length::Fixed(ROW_CONTENT_HEIGHT))
                .width(Length::Fill)
                .style(move |theme, status| button_style(theme, status, selected, choice.complete))
                .on_press(on_select(choice.index)),
        )
        .push(rule::horizontal(SEPARATOR_WIDTH).style(separator_style))
    });
    let laps = laps.push(Space::new().height(Length::Fixed(LIST_BOTTOM_INSET)));

    scrollable(laps)
        .width(Length::Fill)
        .height(Length::Fixed(viewport_height(choices.len())))
        .spacing(4)
        .anchor_bottom()
        .into()
}

pub(crate) fn format_lap_time(milliseconds: i32) -> String {
    if milliseconds <= 0 {
        return "--:--.---".to_owned();
    }

    format_elapsed_milliseconds(milliseconds)
}

fn format_choice_duration(choice: &LapChoice) -> String {
    if choice.complete {
        format_lap_time(choice.duration_ms)
    } else {
        format_elapsed_milliseconds(choice.duration_ms)
    }
}

fn viewport_height(lap_count: usize) -> f32 {
    let visible_rows = lap_count.min(VISIBLE_ROWS);
    if visible_rows == 0 {
        return 0.0;
    }

    ROW_HEIGHT * visible_rows as f32 + LIST_BOTTOM_INSET
}

fn format_elapsed_milliseconds(milliseconds: i32) -> String {
    let milliseconds = milliseconds.max(0);

    let minutes = milliseconds / 60_000;
    let seconds = (milliseconds % 60_000) / 1_000;
    let millis = milliseconds % 1_000;
    format!("{minutes}:{seconds:02}.{millis:03}")
}

fn format_lap_fuel(fuel_litres: Option<f32>) -> String {
    fuel_litres.map_or_else(
        || "-- L".to_owned(),
        |fuel_litres| format!("{fuel_litres:.1} L"),
    )
}

fn button_style(
    theme: &Theme,
    status: iced::widget::button::Status,
    selected: bool,
    complete: bool,
) -> iced::widget::button::Style {
    let palette = theme.extended_palette();
    let background = if selected {
        Some(match status {
            iced::widget::button::Status::Active => palette.background.weak.color,
            iced::widget::button::Status::Hovered => palette.background.neutral.color,
            iced::widget::button::Status::Pressed => palette.background.strong.color,
            iced::widget::button::Status::Disabled => palette.background.weaker.color,
        })
    } else {
        match status {
            iced::widget::button::Status::Pressed => Some(palette.background.weak.color),
            iced::widget::button::Status::Hovered => Some(palette.background.weaker.color),
            iced::widget::button::Status::Disabled | iced::widget::button::Status::Active => None,
        }
    };
    let text_color = if complete || selected {
        if selected {
            match status {
                iced::widget::button::Status::Disabled => {
                    with_alpha(palette.background.base.text, 0.4)
                },
                _ => palette.background.base.text,
            }
        } else {
            palette.background.base.text
        }
    } else {
        with_alpha(palette.background.base.text, 0.62)
    };

    iced::widget::button::Style {
        background: background.map(Background::Color),
        text_color,
        border: Border {
            color: Color::TRANSPARENT,
            width: 0.0,
            radius: 0.0.into(),
        },
        ..iced::widget::button::Style::default()
    }
}

fn separator_style(theme: &Theme) -> rule::Style {
    rule::Style {
        color: theme.extended_palette().background.weaker.color,
        radius: 0.0.into(),
        fill_mode: rule::FillMode::Full,
        snap: true,
    }
}

fn with_alpha(color: Color, alpha: f32) -> Color {
    Color { a: alpha, ..color }
}

#[cfg(test)]
mod tests {
    use iced::{
        Background, Theme,
        widget::{button, rule},
    };

    use super::{
        LapChoice, button_style, format_choice_duration, format_lap_time, separator_style,
        viewport_height,
    };

    #[test]
    fn viewport_is_capped_at_six_rows() {
        assert_eq!(viewport_height(0), 0.0);
        assert_eq!(viewport_height(1), 41.0);
        assert_eq!(viewport_height(6), 206.0);
        assert_eq!(viewport_height(7), 206.0);
    }

    #[test]
    fn formats_lap_time() {
        assert_eq!(format_lap_time(91_234), "1:31.234");
        assert_eq!(format_lap_time(0), "--:--.---");
    }

    #[test]
    fn incomplete_lap_keeps_status_out_of_the_time_cell() {
        let choice = LapChoice::new(0, 1, 9_117, None, false);

        assert_eq!(format_choice_duration(&choice), "0:09.117");
    }

    #[test]
    fn selected_lap_uses_a_neutral_layer_with_the_canvas_foreground() {
        let theme = Theme::Dark;
        let palette = theme.extended_palette();
        let style = button_style(&theme, button::Status::Active, true, true);

        assert_eq!(
            style.background,
            Some(Background::Color(palette.background.weak.color))
        );
        assert_eq!(style.text_color, palette.background.base.text);
    }

    #[test]
    fn incomplete_lap_remains_secondary_but_readable() {
        let theme = Theme::Dark;
        let style = button_style(&theme, button::Status::Active, false, false);

        assert_eq!(style.text_color.a, 0.62);
    }

    #[test]
    fn lap_rows_keep_square_borderless_interaction_layers() {
        let style = button_style(&Theme::Dark, button::Status::Active, false, true);

        assert_eq!(style.background, None);
        assert_eq!(style.border.width, 0.0);
        assert_eq!(style.border.radius, 0.0.into());
    }

    #[test]
    fn lap_row_separators_use_the_weaker_background_tone() {
        let theme = Theme::Dark;
        let palette = theme.extended_palette();
        let separator = separator_style(&theme);

        assert_eq!(separator.color, palette.background.weaker.color);
        assert_eq!(separator.fill_mode, rule::FillMode::Full);
        assert_eq!(separator.radius, 0.0.into());
    }
}
