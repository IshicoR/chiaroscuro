mod sample;
mod wire;

pub use sample::TelemetrySample;
pub use wire::{DecodeError, decode, encode};
