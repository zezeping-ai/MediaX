use std::sync::atomic::AtomicBool;
use std::sync::Arc;

#[allow(dead_code)]
pub struct DecodeSession<'a> {
    pub source: &'a str,
    pub stop_flag: &'a Arc<AtomicBool>,
}
