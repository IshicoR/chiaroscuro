//! Reusable, themed Iced widgets for Chiaro.

pub mod badge;
pub mod button;
pub mod card;
pub mod context_menu;
pub mod dialog;
pub mod form_control;
pub mod icon_button;
pub mod icon_toggle_button;
pub mod navigation_item;
pub mod selection;
pub mod surface;
pub mod tabs;
pub mod toggle_button;
pub mod typography;
pub mod window_control;

pub use badge::{Badge, Variant as BadgeVariant, badge};
pub use button::{Button, Size as ButtonSize, Variant as ButtonVariant, button};
pub use card::{Card, Variant as CardVariant, callout, card, panel};
pub use context_menu::{ContextMenu, context_menu};
pub use dialog::{Dialog, dialog};
pub use form_control::{checkbox_style, toggler_style};
pub use icon_button::{IconButton, icon_button, tooltip_style as icon_tooltip_style};
pub use icon_toggle_button::{IconToggleButton, icon_toggle_button};
pub use navigation_item::{NavigationItem, navigation_item};
pub use selection::{QuietSelection, quiet_selection};
pub use tabs::{Tab, Tabs, tab, tabs};
pub use toggle_button::{ToggleButton, toggle_button};
pub use window_control::{Kind as WindowControlKind, WindowControlButton, window_control};
