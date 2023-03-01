use std::io::{self, Write};

#[cfg(debug_assertions)]
pub fn emit_error(message: &str) {
    io::stderr().write_all(message.as_bytes()).unwrap();
}

#[cfg(not(debug_assertions))]
pub fn emit_error(_: &str) {}
