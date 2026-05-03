pub mod commands;
mod lifecycle;
mod shared;
mod views;

pub use lifecycle::handle_close_requested;
pub use views::{show_main_window, show_preferences_window};
