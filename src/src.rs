use std::{fmt, ops::Deref, slice::SliceIndex};

use contracts::{debug_ensures, ensures};

#[derive(Debug, Clone)]
pub struct Source(String);

impl Source {
    pub fn new(src: String) -> Self {
        Source(src)
    }

    /// Returns the line number of the line containing index i
    pub fn get_lineno(&self, i: usize) -> Result<usize, String> {
        // count number of lines before index i
        let all_before_idx = self.get(..i).ok_or(format!(
            "Index {} out of bounds for Source of length {}",
            i,
            self.len()
        ))?;
        Ok(all_before_idx.matches("\n").count())
    }

    pub fn get_colno(&self, i: usize) -> Result<usize, String> {
        let line = self.get_lineno(i)?;

        if line > 0 {
            let line_start_idx = self[..i]
                .rfind("\n")
                .expect("Should always find a newline if lineno > 0");

            Ok(i - line_start_idx - 1)
        } else {
            Ok(i)
        }
    }

    #[ensures(index < self.len() -> ret.is_ok())]
    #[ensures(index >= self.len() -> ret.clone()
                .expect_err("Out of bounds index should result in error").to_lowercase().contains("out of bounds"))]
    pub fn location_at(&self, index: usize) -> Result<Location, String> {
        if index >= self.len() {
            return Err(format!(
                "Index {} out of bounds for Source of length {}",
                index,
                self.len()
            ));
        }

        return Ok(Location {
            line: self.get_lineno(index)?,
            column: self.get_colno(index)?,
            index,
        });
    }

    pub fn span_between<I: SliceIndex<str>>(&self, _i: I) -> Result<Span, &'static str> {
        unimplemented!()
    }
}

impl Deref for Source {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<String> for Source {
    fn from(value: String) -> Self {
        Source(value)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Location {
    line: usize,
    column: usize,
    index: usize,
}

impl Location {
    pub fn try_new(src: &Source, index: usize) -> Result<Self, String> {
        src.location_at(index)
    }

    pub fn line(&self) -> usize {
        self.line
    }

    pub fn column(&self) -> usize {
        self.column
    }
}

/// A reference to a contiguous range of characters in a source string.
/// Used to track the source spans.
#[derive(Debug, Clone)]
pub struct Span {
    loc: Location,
    span: String,
}

impl Span {
    pub fn new_between(_start: Location, _end: Location) -> Self {
        unimplemented!()
    }

    pub fn empty_at(src: &Source, index: usize) -> Result<Self, String> {
        Ok(Self {
            loc: src.location_at(index)?,
            span: String::new(),
        })
    }

    #[ensures(ret == self.span.len())]
    pub fn len(&self) -> usize {
        self.span.len()
    }

    #[ensures(ret == self.span.is_empty())]
    pub fn is_empty(&self) -> bool {
        self.span.is_empty()
    }

    #[ensures(self.span.chars().eq(ret))]
    pub fn chars(&self, _src: &Source) -> std::str::Chars<'_> {
        unimplemented!()
    }

    pub fn start(&self) -> Location {
        self.loc
    }

    /// Return the index into the Source string that this span starts at
    pub fn start_index(&self) -> usize {
        self.loc.index
    }

    /// Return the index into the Source string that this span ends at
    pub fn end_index(&self) -> usize {
        self.loc.index + self.len()
    }

    /// Set the start position of the span to the given index. The length is set to zero.
    pub fn point_to(&mut self, src: &Source, index: usize) -> Result<(), String> {
        self.clear();
        self.loc = src.location_at(index)?;
        Ok(())
    }

    /// Set the index of the span start
    pub fn start_at(&mut self, _src: &Source, _index: usize) -> Result<(), &'static str> {
        unimplemented!()
    }

    /// Set the index of the span end
    pub fn end_at(&mut self, _src: &Source, _index: usize) -> Result<(), &'static str> {
        unimplemented!()
    }

    #[debug_ensures(self.span.chars().last().unwrap() == c)]
    #[debug_ensures(self.len() == old(self.len()) + 1)]
    pub fn push_char(&mut self, c: char) {
        self.span.push(c);
    }

    #[debug_ensures(self.span.ends_with(s.as_str()))]
    pub fn push_str(&mut self, s: String) {
        s.chars().for_each(|c| self.push_char(c));
    }

    /// Clear the span, effectively resetting it to an empty string.
    /// The span will still start at the same Location.
    #[debug_ensures(old(self.start_index()) == self.start_index())]
    #[ensures(self.is_empty())]
    pub fn clear(&mut self) {
        self.span = String::new();
    }

    /// Return the string slice that the span references.
    /// If the span is empty, return an empty string *which does not reference any part of the source*.
    #[ensures(ret.is_ok() -> ret.unwrap() == self.span)]
    pub fn get<'src>(&self, src: &'src Source) -> Result<&'src str, &'static str> {
        src.get(self.start_index()..self.end_index())
            .ok_or("Failed to retrieve the Span text withinm a Source; was this Span created for this Source?")
    }
}

impl fmt::Display for Span {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.span)
    }
}

#[cfg(test)]
mod tests {
    use parameterized::ide;
    use parameterized::parameterized;
    use std::sync::LazyLock;

    use super::*;

    #[allow(non_upper_case_globals)]
    static hello_world: LazyLock<Source> =
        LazyLock::new(|| Source::new("Hello, world!".to_owned()));

    mod location {
        use super::*;

        macro_rules! test_location_at {
            (
                $($name:ident($src:literal) {
                    $($index_ok:literal : ($line:literal, $column:literal), $c:literal),+
                })+
            ) => {
                $(mod $name {
                    use super::*;

                    ide!();

                    #[parameterized(index = { $($index_ok),+ }, line = { $($line),+ }, column = { $($column),+ }, c = { $($c),+ })]
                    fn ok(index: usize, line: usize, column: usize, c: char) {
                        let src = $src;
                        let src = Source::new(src.to_owned());
                        let loc = src.location_at(index).unwrap();

                        assert_eq!(loc.index, index);
                        assert_eq!(loc.column, column);
                        assert_eq!(loc.line, line);
                        assert_eq!(src.chars().nth(loc.index).unwrap(), c);
                    }
                })+
            };
        }

        test_location_at! {
            hello_world("Hello, world!") {
                0 : (0, 0), 'H',
                4 : (0, 4), 'o',
                12 : (0, 12), '!'
            }

            int_foo_bar("int foo = 'a';\nint bar = 'b';") {
                13 : (0, 13), ';',
                14 : (0, 14), '\n',
                15 : (1, 0), 'i'
            }

            question("?") {
                0 : (0, 0), '?'
            }

            abc_multine("a\nb\nc") {
                0 : (0, 0), 'a',
                1 : (0, 1), '\n',
                2 : (1, 0), 'b',
                3 : (1, 1), '\n',
                4 : (2, 0), 'c'
            }
        }
    }

    mod span {
        use super::*;

        #[test]
        fn test_empty_at() {
            let src = Source::new("char semicolon = ';';".to_owned());
            let idx = 5;
            let span = Span::empty_at(&src, idx).unwrap();

            assert!(span.is_empty());
            assert_eq!(span.len(), 0);
            assert_eq!(span.span, "");
            assert_eq!(span.loc.index, idx);
        }

        #[test]
        fn test_span_push_char() {
            let mut span = Span::empty_at(&hello_world, 0).unwrap();

            span.push_char('H');
            span.push_char('e');
            span.push_char('l');

            assert_eq!(span.to_string(), "Hel");

            span.push_char('l');
            span.push_char('o');

            assert_eq!(span.to_string(), "Hello");
        }

        #[test]
        fn test_span_simple() {
            let mut span = Span::empty_at(&hello_world, 0).unwrap();

            span.push_str("Hello".to_owned());
            assert_eq!(span.to_string(), "Hello");
            assert_eq!(span.get(&hello_world).unwrap(), "Hello");
        }
    }
}
