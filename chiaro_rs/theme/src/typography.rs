use iced::{
    Font,
    font::{Family, Weight},
};

pub const SANS: Font = Font::with_name("Segoe UI");
pub const SANS_SEMIBOLD: Font = Font {
    weight: Weight::Semibold,
    ..SANS
};
pub const MONO_SEMIBOLD: Font = Font {
    family: Family::Name("Cascadia Mono"),
    weight: Weight::Semibold,
    ..Font::DEFAULT
};

pub const SANS_JP_REGULAR_BYTES: &[u8] =
    include_bytes!("../assets/fonts/IBMPlexSansJP-Regular.ttf");
