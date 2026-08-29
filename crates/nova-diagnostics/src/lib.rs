//! Structured compiler diagnostics with human and JSON Lines renderers.

use nova_source::{SourceFile, Span};
use std::fmt;

/// Diagnostic severity.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Severity {
    /// Compilation cannot successfully continue.
    Error,
    /// A valid program deserves user attention.
    Warning,
}

impl Severity {
    /// Returns the stable lowercase representation used by JSON output.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Error => "error",
            Self::Warning => "warning",
        }
    }
}

impl fmt::Display for Severity {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// Whether a source label identifies the main failure or supporting context.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LabelStyle {
    /// Main source range for the diagnostic.
    Primary,
    /// Related source range.
    Secondary,
}

impl LabelStyle {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Primary => "primary",
            Self::Secondary => "secondary",
        }
    }
}

/// A message attached to an exact source range.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Label {
    /// Relationship of this label to the diagnostic.
    pub style: LabelStyle,
    /// Exact half-open source range.
    pub span: Span,
    /// Explanation for the selected range.
    pub message: String,
}

/// A compiler diagnostic independent of presentation format.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Diagnostic {
    /// Error or warning severity.
    pub severity: Severity,
    /// Stable identifier within this compiler version.
    pub code: String,
    /// Summary of the problem.
    pub message: String,
    /// Ordered source annotations. The first primary label is rendered first.
    pub labels: Vec<Label>,
    /// Ordered supporting notes.
    pub notes: Vec<String>,
}

impl Diagnostic {
    /// Creates an error diagnostic.
    #[must_use]
    pub fn error(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            severity: Severity::Error,
            code: code.into(),
            message: message.into(),
            labels: Vec::new(),
            notes: Vec::new(),
        }
    }

    /// Creates a warning diagnostic.
    #[must_use]
    pub fn warning(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            severity: Severity::Warning,
            code: code.into(),
            message: message.into(),
            labels: Vec::new(),
            notes: Vec::new(),
        }
    }

    /// Adds the primary source annotation.
    #[must_use]
    pub fn with_primary(mut self, span: Span, message: impl Into<String>) -> Self {
        self.labels.push(Label {
            style: LabelStyle::Primary,
            span,
            message: message.into(),
        });
        self
    }

    /// Adds a supporting source annotation.
    #[must_use]
    pub fn with_secondary(mut self, span: Span, message: impl Into<String>) -> Self {
        self.labels.push(Label {
            style: LabelStyle::Secondary,
            span,
            message: message.into(),
        });
        self
    }

    /// Adds a supporting note.
    #[must_use]
    pub fn with_note(mut self, note: impl Into<String>) -> Self {
        self.notes.push(note.into());
        self
    }

    fn primary_label_index(&self) -> Option<usize> {
        self.labels
            .iter()
            .position(|label| label.style == LabelStyle::Primary)
            .or_else(|| (!self.labels.is_empty()).then_some(0))
    }
}

/// Renders one diagnostic for a terminal without ANSI color.
#[must_use]
pub fn render_human(diagnostic: &Diagnostic, source: &SourceFile) -> String {
    let mut output = format!(
        "{}[{}]: {}",
        diagnostic.severity, diagnostic.code, diagnostic.message
    );

    let primary_index = diagnostic.primary_label_index();
    if let Some(label) = primary_index.and_then(|index| diagnostic.labels.get(index)) {
        render_primary_human(&mut output, label, source);
    }

    for (index, label) in diagnostic.labels.iter().enumerate() {
        if Some(index) == primary_index {
            continue;
        }
        output.push_str("\n = ");
        output.push_str(label.style.as_str());
        output.push_str(": ");
        if label.span.source() == source.id() {
            if let Some(location) = source.location(label.span.start()) {
                output.push_str(source.name());
                output.push(':');
                output.push_str(&location.line.to_string());
                output.push(':');
                output.push_str(&location.column.to_string());
                output.push_str(": ");
            }
        }
        output.push_str(&label.message);
    }

    for note in &diagnostic.notes {
        output.push_str("\n = note: ");
        output.push_str(note);
    }

    output
}

fn render_primary_human(output: &mut String, label: &Label, source: &SourceFile) {
    if label.span.source() != source.id() {
        output.push_str("\n --> <different source>:");
        output.push_str(&label.span.start().to_string());
        output.push_str("..");
        output.push_str(&label.span.end().to_string());
        return;
    }

    let Some(location) = source.location(label.span.start()) else {
        output.push_str("\n --> <invalid span>");
        return;
    };
    let line_text = source.line_text(location.line).unwrap_or("");
    let gutter_width = location.line.to_string().len();

    output.push_str("\n --> ");
    output.push_str(source.name());
    output.push(':');
    output.push_str(&location.line.to_string());
    output.push(':');
    output.push_str(&location.column.to_string());
    output.push('\n');
    output.push_str(&" ".repeat(gutter_width));
    output.push_str(" |\n");
    output.push_str(&location.line.to_string());
    output.push_str(" | ");
    output.push_str(line_text);
    output.push('\n');
    output.push_str(&" ".repeat(gutter_width));
    output.push_str(" | ");
    if let Some(prefix) = source.text().get(location.line_start..label.span.start()) {
        for character in prefix.chars() {
            output.push(if character == '\t' { '\t' } else { ' ' });
        }
    }

    let visible_end = label.span.end().min(location.line_end);
    let selected = source
        .text()
        .get(label.span.start()..visible_end)
        .map_or(0, |text| text.chars().count());
    output.push_str(&"^".repeat(selected.max(1)));
    if !label.message.is_empty() {
        output.push(' ');
        output.push_str(&label.message);
    }
}

/// Renders diagnostics separated by a blank line.
#[must_use]
pub fn render_human_all(diagnostics: &[Diagnostic], source: &SourceFile) -> String {
    diagnostics
        .iter()
        .map(|diagnostic| render_human(diagnostic, source))
        .collect::<Vec<_>>()
        .join("\n\n")
}

/// Renders one diagnostic as one self-contained JSON object.
#[must_use]
pub fn render_json(diagnostic: &Diagnostic, source: &SourceFile) -> String {
    let mut output = String::from("{\"severity\":");
    push_json_string(&mut output, diagnostic.severity.as_str());
    output.push_str(",\"code\":");
    push_json_string(&mut output, &diagnostic.code);
    output.push_str(",\"message\":");
    push_json_string(&mut output, &diagnostic.message);
    output.push_str(",\"labels\":[");

    for (index, label) in diagnostic.labels.iter().enumerate() {
        if index != 0 {
            output.push(',');
        }
        output.push_str("{\"style\":");
        push_json_string(&mut output, label.style.as_str());
        output.push_str(",\"source\":");
        push_json_string(&mut output, source.name());
        output.push_str(",\"span\":{\"start\":");
        output.push_str(&label.span.start().to_string());
        output.push_str(",\"end\":");
        output.push_str(&label.span.end().to_string());
        output.push('}');

        if label.span.source() == source.id() {
            if let Some(location) = source.location(label.span.start()) {
                output.push_str(",\"location\":{\"line\":");
                output.push_str(&location.line.to_string());
                output.push_str(",\"column\":");
                output.push_str(&location.column.to_string());
                output.push('}');
            }
        }

        output.push_str(",\"message\":");
        push_json_string(&mut output, &label.message);
        output.push('}');
    }

    output.push_str("],\"notes\":[");
    for (index, note) in diagnostic.notes.iter().enumerate() {
        if index != 0 {
            output.push(',');
        }
        push_json_string(&mut output, note);
    }
    output.push_str("]}");
    output
}

/// Renders one JSON object per line for streaming consumers.
#[must_use]
pub fn render_json_lines(diagnostics: &[Diagnostic], source: &SourceFile) -> String {
    diagnostics
        .iter()
        .map(|diagnostic| render_json(diagnostic, source))
        .collect::<Vec<_>>()
        .join("\n")
}

fn push_json_string(output: &mut String, value: &str) {
    output.push('"');
    for character in value.chars() {
        match character {
            '"' => output.push_str("\\\""),
            '\\' => output.push_str("\\\\"),
            '\u{08}' => output.push_str("\\b"),
            '\u{0c}' => output.push_str("\\f"),
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            '\t' => output.push_str("\\t"),
            '\u{00}'..='\u{1f}' => {
                output.push_str("\\u");
                output.push_str(&format!("{:04x}", u32::from(character)));
            }
            _ => output.push(character),
        }
    }
    output.push('"');
}

#[cfg(test)]
mod tests {
    use super::{Diagnostic, render_human, render_json};
    use nova_source::{SourceFile, SourceId};

    #[test]
    fn renders_exact_human_location_and_caret() {
        let source = SourceFile::new(SourceId::new(0), "sample.nv", "let β = 1;\n");
        let span = source.span(4, 6).expect("beta is two complete UTF-8 bytes");
        let diagnostic = Diagnostic::error("N0001", "example")
            .with_primary(span, "selected")
            .with_note("a note");
        let rendered = render_human(&diagnostic, &source);

        assert!(rendered.contains("error[N0001]: example"));
        assert!(rendered.contains("sample.nv:1:5"));
        assert!(rendered.contains("|     ^ selected"));
        assert!(rendered.contains("= note: a note"));
    }

    #[test]
    fn json_output_escapes_user_control_characters() {
        let source = SourceFile::new(SourceId::new(0), "sample.nv", "x");
        let span = source.span(0, 1).expect("valid span");
        let diagnostic = Diagnostic::error("N\"1", "line\nnext").with_primary(span, "\\quoted\"");
        let rendered = render_json(&diagnostic, &source);

        assert!(rendered.starts_with("{\"severity\":\"error\""));
        assert!(rendered.contains("\"code\":\"N\\\"1\""));
        assert!(rendered.contains("\"message\":\"line\\nnext\""));
        assert!(rendered.contains("\"message\":\"\\\\quoted\\\"\""));
        assert!(rendered.ends_with("}"));
    }

    #[test]
    fn warning_severity_is_preserved_by_both_renderers() {
        let source = SourceFile::new(SourceId::new(0), "sample.nv", "x");
        let span = source.span(0, 1).expect("valid span");
        let diagnostic =
            Diagnostic::warning("N3033", "example warning").with_primary(span, "selected");

        assert!(render_human(&diagnostic, &source).starts_with("warning[N3033]"));
        assert!(render_json(&diagnostic, &source).starts_with("{\"severity\":\"warning\""));
    }
}
