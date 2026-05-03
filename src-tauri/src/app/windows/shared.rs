use std::sync::{Mutex, OnceLock};
use tauri::{PhysicalPosition, PhysicalSize};

pub const MAIN_WINDOW_LABEL: &str = "main";
pub const PREFERENCES_WINDOW_LABEL: &str = "preferences";

#[derive(Clone, Copy)]
pub struct WindowRestoreBounds {
    pub position: PhysicalPosition<i32>,
    pub size: PhysicalSize<u32>,
    pub maximized: bool,
}

pub fn main_window_restore_bounds() -> &'static Mutex<Option<WindowRestoreBounds>> {
    static RESTORE_BOUNDS: OnceLock<Mutex<Option<WindowRestoreBounds>>> = OnceLock::new();
    RESTORE_BOUNDS.get_or_init(|| Mutex::new(None))
}
