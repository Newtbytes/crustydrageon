pub struct Source(String);

impl Source {
    pub fn get_string(&self) -> &String {
        &self.0
    }
}

pub struct Location {
    line: usize,
    col: usize,
    idx: usize,
}

pub struct Span {
    start: Location,
    end: Location,
}
