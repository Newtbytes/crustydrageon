use std::env;

#[must_use]
pub fn positional_arg(pos: usize) -> Option<String> {
    env::args().nth(pos + 1)
}

#[must_use]
pub fn flag(name: &'static str) -> bool {
    env::args().find(move |arg| arg == name).is_some()
}
