use std::{fmt::Display, ops::Deref};

use contracts::{debug_ensures, ensures};

#[derive(Debug, Clone)]
pub struct Source(String);

impl Source {
    pub fn new(src: String) -> Self {
        Source(src)
    }

    pub fn chars(&self) -> std::str::Chars<'_> {
        self.0.chars()
    }

    pub fn len(&self) -> usize {
        self.0.len()
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

/// A reference to a contiguous range of characters in a source string.
/// Used to track the source spans.
#[derive(Debug, Clone, Copy)]
pub struct Span<'src> {
    src: &'src Source,
    span: Option<&'src str>,
}

impl<'src> Span<'src> {
    /// Create a new span that references the given source and covers the range from `start` to `start + len`.
    /// The `start` and `len` parameters are in bytes, not characters.
    /// Returns an error if the specified range is out of bounds of the source string or if it does not align with UTF-8 character boundaries.
    pub fn new(src: &'src Source, start: usize, len: usize) -> Result<Self, &'static str> {
        Ok(Self {
            src,
            span: Some(&src.get(start..start + len).ok_or("Invalid span range")?),
        })
    }

    /// Create an empty span that references the given source.
    /// The span will be an empty string, but it will still reference the source.
    pub fn empty(src: &'src Source) -> Self {
        Self { src, span: None }
    }

    /// Create an empty span that references the given source and starts at the given byte offset.
    /// The span will be an empty string, but it will still reference the source.
    pub fn empty_at(src: &'src Source, start: usize) -> Result<Self, &'static str> {
        Self::new(src, start, 0)
    }

    /// Return the length of the span in bytes.
    /// Note that this is the length in bytes, not characters. For example, a span that references Ferris "🦀" will have a length of 4 bytes, even though it is only one character.
    /// ```
    /// # use crustydrageon::src::{Source, Span};
    /// let src = Source::new("🦀".to_string());
    /// let span = Span::new(&src, 0, 4).unwrap();
    /// assert_eq!(span.len(), 4);
    /// ```
    ///
    /// If the span is empty, a length of 0 is returned:
    /// ```
    /// # use crustydrageon::src::{Source, Span};
    /// # let src = Source::new("12345".to_string());
    /// let span = Span::empty(&src);
    /// assert_eq!(span.len(), 0);
    /// ```
    ///
    /// # Note
    /// For most use cases, you should probably use `chars().count()` instead of `len()` to get the number of characters in the span, since UTF-8 characters can be more than one byte long.
    #[ensures(self.span.is_none() -> ret == 0)]
    pub fn len(&self) -> usize {
        self.span.map_or(0, |s| s.len())
    }

    pub fn is_empty(&self) -> bool {
        self.span.map_or(true, |s| s.is_empty())
    }

    pub fn chars(&self) -> std::str::Chars<'_> {
        self.as_str().chars()
    }

    /// Return the byte offset of the start of the span in the source string.
    pub fn start_index(&self) -> usize {
        self.span
            .map_or(0, |s| s.as_ptr() as usize - self.src.as_ptr() as usize)
    }

    /// Return the byte offset of the end of the span in the source string.
    pub fn end_index(&self) -> usize {
        self.start_index() + self.len()
    }

    /// Set the offset of the span to the given byte offset. The length is set to zero.
    /// ```
    /// # use crustydrageon::src::{Source, Span};
    /// let src = Source::new("let ferris = 5;".to_string());
    /// let mut span = Span::empty(&src);
    /// span.point_to(4).unwrap();
    /// assert_eq!(span.start_index(), 4);
    /// assert_eq!(span.end_index(), 4);
    /// assert_eq!(span.as_str(), "");
    /// assert_eq!(span.len(), 0);
    /// ```
    #[debug_ensures(ret.is_ok() -> self.src.get(index..index) == self.src.get(self.start_index()..self.start_index()))]
    #[debug_ensures(ret.is_err() -> old(self.start_index()) == self.start_index())]
    pub fn point_to(&mut self, index: usize) -> Result<(), &'static str> {
        self.span = Some(self.src.get(index..index).ok_or("Invalid offset")?);
        Ok(())
    }

    #[debug_ensures(ret.is_err() -> old(self.start_index()) == self.start_index())]
    pub fn start_at(&mut self, index: usize) -> Result<(), &'static str> {
        todo!()
    }

    /// Set the end index of the span to the index
    #[debug_ensures(ret.is_err() -> old(self.end_index()) == self.end_index())]
    pub fn end_at(&mut self, index: usize) -> Result<(), &'static str> {
        if index < self.start_index() {
            Err("Cannot set end index before start index")
        } else {
            self.span = Some(
                self.src
                    .get(self.start_index()..index)
                    .ok_or("Invalid offset")?,
            );
            Ok(())
        }
    }

    /// Clear the span, effectively resetting it to an empty string.
    /// The span will still reference the same source and start at the same offset.
    ///
    /// # Examples
    /// ```
    /// # use crustydrageon::src::{Source, Span};
    /// # let src = Source::new("Hello, world!".to_string());
    /// # let mut span = Span::empty(&src);
    /// # span.advance_by(5).unwrap();
    /// assert_eq!(span.as_str(), "Hello");
    /// span.clear();
    /// assert_eq!(span.as_str(), "");
    /// ```
    #[debug_ensures(old(self.start_index()) == self.start_index())]
    #[ensures(self.is_empty())]
    pub fn clear(&mut self) {
        self.span = Some(
            self.src
                .get(self.start_index()..self.start_index())
                .unwrap(),
        );
    }

    /// Completely reset the span. Unlike Span::clear(), the start index is not retained.
    #[ensures(self.is_empty())]
    pub fn reset(&mut self) {
        self.span = None;
    }

    /// Return the string slice that the span references.
    /// If the span is empty, return an empty string *which does not reference any part of the source*.
    #[debug_ensures(ret.is_empty() -> self.is_empty())]
    pub fn as_str(&self) -> &str {
        self.span.map_or("", |s| s)
    }

    /// Advance the span by `n` bytes.
    /// The span will be advanced from the end of the current span.
    /// If the span is currently empty, it will be set to the first `n` characters of the source.
    ///
    /// ```
    /// # use crustydrageon::src::{Source, Span};
    /// let src = Source::new("Hello, world!".to_string());
    /// let mut span = Span::empty(&src);
    /// span.advance_by(5).unwrap();
    /// assert_eq!(span.as_str(), "Hello");
    /// ```
    ///
    /// Returns an error if the advancement would go out of bounds of the source string.
    ///
    /// ```
    /// # use crustydrageon::src::{Source, Span};
    /// # let src = Source::new("Hello, world!".to_string());
    /// # let mut span = Span::empty(&src);
    /// # span.advance_by(1).unwrap();
    /// span.advance_by(100).expect_err("Advancing the span beyond the bounds of the source should return an error");
    /// ```
    #[debug_ensures(ret.is_ok() -> self.len() <= self.src.len())]
    #[debug_ensures(ret.is_ok() -> self.start_index() < self.src.len())]
    #[debug_ensures(ret.is_ok() -> self.end_index() <= self.src.len())]
    #[debug_ensures(ret.is_ok() -> self.start_index() <= self.end_index())]
    pub fn advance_by(&mut self, n: usize) -> Result<(), &'static str> {
        let span = match self.span {
            Some(span) => span,
            None => {
                self.span = Some(
                    self.src
                        .get(0..n)
                        .ok_or("Attempted to create a larger span than the source")?,
                );

                return Ok(());
            }
        };

        // get the index into self.src where the span starts
        let start = self.start_index();
        let new_len = span.len() + n;
        let end: usize = start + new_len;

        self.span = Some(
            self.src
                .get(start..end)
                .ok_or("Attempted to advance span out of bounds")?,
        );

        Ok(())
    }
}

impl Display for Span<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.span.map_or("", |s| s))
    }
}

pub struct Location {
    pub line: usize,
    pub column: usize,
    pub offset: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_span_empty() {
        let src = Source::new("Hello, world!".to_string());
        let span = Span::empty(&src);
        assert_eq!(span.as_str(), "");
    }

    #[test]
    fn test_span_new() {
        let src = Source::new("Hello, world!".to_string());
        let span = Span::new(&src, 0, 5).unwrap();
        assert_eq!(span.as_str(), "Hello");
    }

    #[test]
    fn test_span_new_unicode() {
        let src = Source::new("👋, world!".to_string());
        let span = Span::new(&src, 0, 4).unwrap();
        assert_eq!(span.as_str(), "👋");
        let span = Span::new(&src, 0, 5).unwrap();
        assert_eq!(span.as_str(), "👋,");
    }

    #[test]
    fn test_span_new_unicode_boundary() {
        let src = Source::new("👋, world!".to_string());
        Span::new(&src, 0, 1).unwrap_err();
        Span::new(&src, 0, 2).unwrap_err();
        Span::new(&src, 0, 3).unwrap_err();
    }

    #[test]
    fn test_span_advance() {
        let src = Source::new("Hello, world!".to_string());
        let mut span = Span::empty(&src);
        span.advance_by(1).unwrap();
        assert_eq!(span.as_str(), "H");
        span.advance_by(4).unwrap();
        assert_eq!(span.as_str(), "Hello");
        span.advance_by(2).unwrap();
        assert_eq!(span.as_str(), "Hello, ");
    }

    #[test]
    fn test_span_advance_utf8() {
        let src = Source::new("👋, world!".to_string());
        let mut span = Span::empty(&src);
        span.advance_by(4).unwrap();
        assert_eq!(span.as_str(), "👋");
        span.advance_by(2).unwrap();
        assert_eq!(span.as_str(), "👋, ");
    }

    #[test]
    fn test_span_advance_out_of_bounds() {
        let src = Source::new("Hello".to_string());
        let mut span = Span::empty(&src);
        span.advance_by(10).expect_err(
            "Advancing the span beyond the bounds of the source should return an error",
        );
    }
}
