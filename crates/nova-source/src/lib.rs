//! Source identity, UTF-8 text, byte spans, and rendered locations.

use std::fmt;

/// Stable identity for one source within a compiler session.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct SourceId(u32);

impl SourceId {
    /// Creates an identity from a session-local integer.
    #[must_use]
    pub const fn new(raw: u32) -> Self {
        Self(raw)
    }

    /// Returns the session-local integer representation.
    #[must_use]
    pub const fn raw(self) -> u32 {
        self.0
    }
}

/// A half-open UTF-8 byte range associated with one source.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct Span {
    source: SourceId,
    start: usize,
    end: usize,
}

impl Span {
    /// Creates a span when the bounds are ordered.
    #[must_use]
    pub const fn new(source: SourceId, start: usize, end: usize) -> Option<Self> {
        if start <= end {
            Some(Self { source, start, end })
        } else {
            None
        }
    }

    /// Creates an empty span at an offset.
    #[must_use]
    pub const fn empty(source: SourceId, offset: usize) -> Self {
        Self {
            source,
            start: offset,
            end: offset,
        }
    }

    /// Returns the source identity.
    #[must_use]
    pub const fn source(self) -> SourceId {
        self.source
    }

    /// Returns the inclusive byte start.
    #[must_use]
    pub const fn start(self) -> usize {
        self.start
    }

    /// Returns the exclusive byte end.
    #[must_use]
    pub const fn end(self) -> usize {
        self.end
    }

    /// Returns the byte length.
    #[must_use]
    pub const fn len(self) -> usize {
        self.end - self.start
    }

    /// Reports whether this span is empty.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.start == self.end
    }

    /// Returns the smallest span covering both spans when their sources match.
    #[must_use]
    pub fn covering(self, other: Self) -> Option<Self> {
        (self.source == other.source).then_some(Self {
            source: self.source,
            start: self.start.min(other.start),
            end: self.end.max(other.end),
        })
    }
}

impl fmt::Display for Span {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{}:{}..{}",
            self.source.raw(),
            self.start,
            self.end
        )
    }
}

/// A one-based human source location.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Location {
    /// One-based line number.
    pub line: usize,
    /// One-based Unicode-scalar column.
    pub column: usize,
    /// Byte offset at the start of the line.
    pub line_start: usize,
    /// Byte offset after the visible contents of the line.
    pub line_end: usize,
}

/// One validated UTF-8 source file.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceFile {
    id: SourceId,
    name: String,
    text: String,
    line_starts: Vec<usize>,
}

impl SourceFile {
    /// Creates a source and indexes its line starts.
    #[must_use]
    pub fn new(id: SourceId, name: impl Into<String>, text: impl Into<String>) -> Self {
        let text = text.into();
        let mut line_starts = vec![0];
        for (offset, byte) in text.bytes().enumerate() {
            if byte == b'\n' {
                line_starts.push(offset + 1);
            }
        }
        Self {
            id,
            name: name.into(),
            text,
            line_starts,
        }
    }

    /// Returns this source's identity.
    #[must_use]
    pub const fn id(&self) -> SourceId {
        self.id
    }

    /// Returns the display name, normally a path.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the validated UTF-8 contents.
    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Returns the source length in UTF-8 bytes.
    #[must_use]
    pub fn len(&self) -> usize {
        self.text.len()
    }

    /// Reports whether this source is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.text.is_empty()
    }

    /// Returns an empty span at end of file.
    #[must_use]
    pub fn eof_span(&self) -> Span {
        Span::empty(self.id, self.text.len())
    }

    /// Creates a source-qualified span after validating bounds and UTF-8 edges.
    #[must_use]
    pub fn span(&self, start: usize, end: usize) -> Option<Span> {
        if start <= end
            && end <= self.text.len()
            && self.text.is_char_boundary(start)
            && self.text.is_char_boundary(end)
        {
            Span::new(self.id, start, end)
        } else {
            None
        }
    }

    /// Returns text selected by a valid span for this source.
    #[must_use]
    pub fn slice(&self, span: Span) -> Option<&str> {
        if span.source() != self.id {
            return None;
        }
        self.text.get(span.start()..span.end())
    }

    /// Resolves a character-boundary byte offset to a human location.
    #[must_use]
    pub fn location(&self, offset: usize) -> Option<Location> {
        if offset > self.text.len() || !self.text.is_char_boundary(offset) {
            return None;
        }

        let line_index = self
            .line_starts
            .partition_point(|line_start| *line_start <= offset)
            .saturating_sub(1);
        let line_start = self.line_starts[line_index];
        let mut line_end = self
            .line_starts
            .get(line_index + 1)
            .map_or(self.text.len(), |next_start| next_start.saturating_sub(1));
        if self.text.as_bytes().get(line_end.wrapping_sub(1)) == Some(&b'\r') {
            line_end -= 1;
        }
        let column = self.text[line_start..offset].chars().count() + 1;

        Some(Location {
            line: line_index + 1,
            column,
            line_start,
            line_end,
        })
    }

    /// Returns a visible line without its line ending.
    #[must_use]
    pub fn line_text(&self, one_based_line: usize) -> Option<&str> {
        let line_start = *self.line_starts.get(one_based_line.checked_sub(1)?)?;
        let location = self.location(line_start)?;
        self.text.get(location.line_start..location.line_end)
    }
}

#[cfg(test)]
mod tests {
    use super::{SourceFile, SourceId, Span};

    #[test]
    fn resolves_unicode_columns_and_crlf_lines() {
        let source = SourceFile::new(SourceId::new(7), "test.nv", "a\nβx\r\n");

        let beta = source.location(2).expect("beta starts on a character edge");
        let x = source.location(4).expect("x starts on a character edge");
        let eof = source
            .location(source.len())
            .expect("EOF is a character edge");

        assert_eq!((beta.line, beta.column), (2, 1));
        assert_eq!((x.line, x.column), (2, 2));
        assert_eq!((eof.line, eof.column), (3, 1));
        assert_eq!(source.line_text(2), Some("βx"));
    }

    #[test]
    fn rejects_invalid_or_foreign_spans() {
        let source = SourceFile::new(SourceId::new(1), "test.nv", "β");
        let foreign = Span::new(SourceId::new(2), 0, 1).expect("ordered span");

        assert_eq!(source.span(0, 1), None);
        assert_eq!(source.span(3, 3), None);
        assert_eq!(source.slice(foreign), None);
    }

    #[test]
    fn combines_spans_only_within_one_source() {
        let left = Span::new(SourceId::new(1), 3, 5).expect("ordered span");
        let right = Span::new(SourceId::new(1), 1, 4).expect("ordered span");
        let foreign = Span::empty(SourceId::new(2), 0);

        assert_eq!(
            left.covering(right).map(|span| (span.start(), span.end())),
            Some((1, 5))
        );
        assert_eq!(left.covering(foreign), None);
    }
}
