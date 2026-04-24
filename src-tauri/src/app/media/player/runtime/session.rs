use std::sync::Arc;
use std::sync::atomic::AtomicBool;

#[allow(dead_code)]
pub struct DecodeSession<'a> {
    pub source: &'a str,
    pub stop_flag: &'a Arc<AtomicBool>,
}
