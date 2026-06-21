//! Source code location and span tracking.
//!
//! This module provides abstractions for tracking positions and ranges within source code.
//! It defines the following main types:
//!
//! - [`Source`]: A wrapper around the entire source file as a top-level span
//! - [`Location`]: A byte index with line and column information
//! - [`Span`]: A contiguous range of characters in source code
//!
//! These types are used throughout the compiler to associate syntax elements with their
//! original source locations.

use std::fmt;

use contracts::ensures;
use derive_more::Deref;
#[cfg(test)]
use proptest_derive::Arbitrary;

/// A top-level [`Span`] referring to the entire source file.
#[derive(Debug, Clone, Deref)]
#[cfg_attr(test, derive(Arbitrary))]
pub struct Source(Span);

impl Source {
    #[must_use]
    pub fn new(src: String) -> Self {
        Self(Span {
            loc: Location::default(),
            span: src,
        })
    }
}

impl From<Source> for Span {
    fn from(src: Source) -> Self {
        Span {
            loc: Location::default(),
            span: src.to_string(),
        }
    }
}

impl From<String> for Source {
    fn from(value: String) -> Self {
        Source::new(value)
    }
}

/// A character index into source code with line and column information.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(test, derive(Arbitrary))]
pub struct Location {
    line: usize,
    column: usize,
    index: usize,
}

impl Location {
    /// Get the line number.
    #[must_use]
    pub fn line(&self) -> usize {
        self.line
    }

    /// Get the column number.
    #[must_use]
    pub fn column(&self) -> usize {
        self.column
    }
}

/// A reference to a contiguous range of characters in a source string.
#[derive(Debug, Clone, PartialEq, Eq, Default, Deref)]
#[cfg_attr(test, derive(Arbitrary))]
pub struct Span {
    loc: Location,
    #[deref]
    span: String,
}

impl Span {
    /// Create an empty [`subspan`](Span::subspan()) at an index.
    ///
    /// # Examples
    ///
    /// ```
    /// # use crustydrageon::src::*;
    /// let span = Source::new("Hello, world!".to_owned());
    /// let subspan = span.empty_at(4).unwrap();
    ///
    /// assert!(subspan.is_empty());
    /// ```
    #[must_use]
    pub fn empty_at(&self, index: usize) -> Option<Self> {
        self.subspan(index, index)
    }

    #[ensures(ret == self.span.len())]
    pub fn len(&self) -> usize {
        self.span.len()
    }

    #[ensures(ret == self.span.is_empty())]
    #[ensures(ret == (self.start_index() == self.end_index()))]
    pub fn is_empty(&self) -> bool {
        self.span.is_empty()
    }

    /// Return an iterator over the [`char`]s in this span.
    ///
    /// See [`str::chars`].
    ///
    /// # Examples
    ///
    /// ```
    /// # use crustydrageon::src::*;
    /// let src = Source::new("Hello, world!".to_owned());
    /// let mut chars = src.chars();
    ///
    /// assert_eq!(chars.next(), Some('H'));
    /// assert_eq!(chars.next(), Some('e'));
    /// assert_eq!(chars.next(), Some('l'));
    /// assert_eq!(chars.next(), Some('l'));
    /// assert_eq!(chars.next(), Some('o'));
    ///
    /// assert!(src.chars().eq("Hello, world!".chars()));
    /// ```
    pub fn chars(&self) -> std::str::Chars<'_> {
        self.span.chars()
    }

    /// Return the [`Location`] within the parent span pointing to the first character in this span.
    ///
    /// # Examples
    ///
    /// ```
    /// # use crustydrageon::src::*;
    /// let src = Source::new("Hello, world!".to_owned());
    ///
    /// assert_eq!(src.empty_at(0).unwrap().start(), src.location_at(0).unwrap());
    /// assert_eq!(src.empty_at(5).unwrap().start(), src.location_at(5).unwrap());
    ///
    /// let hello_span = src.subspan(0, 5).unwrap();
    /// assert_eq!(hello_span.start(), src.location_at(0).unwrap());
    ///
    /// let world_span = src.subspan(7, 12).unwrap();
    /// assert_eq!(world_span.start(), src.location_at(7).unwrap());
    /// ```
    #[must_use]
    pub fn start(&self) -> Location {
        self.loc
    }

    /// Return the index into the Source string that this span starts at.
    #[must_use]
    pub fn start_index(&self) -> usize {
        self.loc.index
    }

    /// Return the index into the Source string that this span ends at.
    #[must_use]
    pub fn end_index(&self) -> usize {
        self.loc.index + self.len()
    }

    /// Returns the line number of the line containing the byte at index `i`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use crustydrageon::src::*;
    /// let two_decls = Source::new("int a;\nint b;".to_owned());
    ///
    /// assert_eq!(two_decls.find_line(4).unwrap(), 0);
    /// assert_eq!(two_decls.find_line(11).unwrap(), 1);
    ///
    /// // using an out of bounds index is an error
    /// assert!(two_decls.find_line(13).is_err());
    /// ```
    pub fn find_line(&self, i: usize) -> Result<usize, String> {
        let err = format!(
            "Index {} out of bounds for Source of length {}",
            i,
            self.len()
        );

        if i >= self.len() {
            return Err(err);
        }

        // count number of lines before index i
        let all_before_idx = self.get(..i).ok_or(err)?;
        Ok(all_before_idx.matches('\n').count())
    }

    /// Returns the line number of the line containing the byte at index `i`.
    pub fn find_column(&self, i: usize) -> Result<usize, String> {
        let line = self.find_line(i)?;

        if line > 0 {
            let line_start_idx = self[..i]
                .rfind('\n')
                .expect("Should always find a newline if lineno > 0");

            Ok(i - line_start_idx - 1)
        } else {
            Ok(i)
        }
    }

    /// Returns a Location pointing to the character at index `i`.
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
            line: self.find_line(index)?,
            column: self.find_column(index)?,
            index,
        });
    }

    /// Create a subspan of this [`Span`].
    ///
    /// Creates a subspan that contains the contiguous sequence of characters from index `start` (inclusive) to index
    /// `end` (exclusive).
    ///
    /// Instead of panicking for out of bound indices, [`None`] is returned.
    ///
    /// # Examples
    ///
    /// ```
    /// # use crustydrageon::src::Source;
    /// let src = Source::new("Hello, world!".to_owned());
    ///
    /// assert_eq!(src.subspan(0, 5).unwrap().as_str(), "Hello");
    /// assert_eq!(src.subspan(7, 12).unwrap().as_str(), "world");
    ///
    /// // Out of bounds
    /// assert!(src.subspan(src.len() + 1, src.len() + 2).is_none());
    /// assert!(src.subspan(src.len() - 1, src.len() + 1).is_none());
    /// ```
    #[ensures(ret.is_some() -> start < self.len() && end <= self.len(),
        "if [`None`] isn't returned, the indices must be in-bounds"
    )]
    #[ensures(
        start >= self.len() || end > self.len() -> ret.is_none(),
        "if either end of the input range is out of bounds, [`None`] is returned"
    )]
    pub fn subspan(&self, start: usize, end: usize) -> Option<Self> {
        Some(Self {
            loc: self.location_at(start).ok()?,
            span: self.get(start..end)?.to_owned(),
        })
    }
}

impl fmt::Display for Span {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.span)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use proptest::prelude::*;
    use rstest::{fixture, rstest};

    #[fixture]
    fn hello_world() -> Source {
        Source::new("Hello, world!".to_owned())
    }

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

                    #[rstest]
                    $(#[case($index_ok, $line, $column, $c)])+
                    fn ok(#[case] index: usize, #[case] line: usize, #[case] column: usize, #[case] c: char) {
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

        mod init {
            use super::*;

            #[rstest]
            #[case("Hello, world!", 4)]
            #[case("char semicolon = ';';", 5)]
            fn test_empty_at(#[case] src: &str, #[case] idx: usize) {
                let src = Source::new(src.to_owned());
                let span = Span::empty_at(&src, idx).unwrap();

                assert!(span.is_empty());
                assert_eq!(span.len(), 0);
                assert_eq!(span.span, "");
                assert_eq!(span.loc.index, idx);
            }

            proptest! {
                #[test]
                fn test_equal_empty_at(src: String, idx: usize) {
                    let src = Source::new(src);

                    prop_assert_eq!(Span::empty_at(&src, idx), src.empty_at(idx));
                }
            }
        }
    }
}
