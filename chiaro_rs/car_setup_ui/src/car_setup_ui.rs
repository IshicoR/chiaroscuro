//! Car setup screen state, update logic, and view.

mod interaction;
mod layout;
mod setup_view;
mod state;
mod view;

pub use interaction::subscription;
pub use layout::{CarSetupLayout, CarSetupLayoutFlag};
pub use state::{
    CarSetupMessage, CarSetupState, activate, deactivate, refresh, reset_reference, reset_session,
    update,
};
pub use view::view;
