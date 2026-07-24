use iced::{Font, font::Weight};

/// IBM Plex Sans JP Regular, bundled with the application.
pub const SANS: Font = Font::with_name("IBM Plex Sans JP");
pub const SANS_SEMIBOLD: Font = Font {
    weight: Weight::Semibold,
    ..SANS
};
/// IBM Plex Mono Regular, bundled with the application.
pub const MONO: Font = Font::with_name("IBM Plex Mono");

pub const SANS_REGULAR_BYTES: &[u8] = include_bytes!("../assets/fonts/IBMPlexSansJP-Regular.ttf");
pub const SANS_SEMIBOLD_BYTES: &[u8] = include_bytes!("../assets/fonts/IBMPlexSansJP-SemiBold.ttf");
pub const MONO_REGULAR_BYTES: &[u8] = include_bytes!("../assets/fonts/IBMPlexMono-Regular.ttf");
