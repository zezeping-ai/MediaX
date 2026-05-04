mod lifecycle;
mod restart;
mod seek;
mod switches;
pub use lifecycle::{open, pause, play, stop};
pub use seek::seek;
pub use switches::{set_hw_decode_mode, set_quality_mode};
