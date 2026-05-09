use std::env;

#[must_use] 
pub fn positional_arg(pos: usize) -> Option<String> {
    env::args().nth(pos + 1)
}

#[must_use] 
pub fn flag(name: &'static str) -> bool {
    env::args().find(move |arg| {
        arg.len() == name.len() + 2 // + 2 for leading '--'
        && arg.starts_with("--")
        && arg.ends_with(name)
    }).is_some()
}
