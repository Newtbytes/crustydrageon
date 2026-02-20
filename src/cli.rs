use std::env;

pub fn positional_arg(pos: usize) -> Option<String> {
    env::args().nth(pos + 1)
}

pub fn flag(name: &'static str) -> bool {
    match env::args().find(move |arg| {
        arg.len() == name.len() + 2 // + 2 for leading '--'
        && arg.starts_with("--")
        && arg.ends_with(name)
    }) {
        Some(_) => true,
        None => false,
    }
}
