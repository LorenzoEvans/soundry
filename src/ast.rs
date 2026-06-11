//! Public data model for raw and resolved SFZ documents.

use crate::opcode_mapping::Opcode;
use std::fmt;

/// A parsed SFZ document.
///
/// `blocks` preserves the source structure and raw opcode data. `regions`
/// contains inheritance-resolved region views with typed known opcodes and raw
/// fallback data for everything else.
#[derive(Debug, Clone, PartialEq)]
pub struct SfzDocument {
    pub blocks: Vec<HeaderBlock>,
    pub control: Option<Control>,
    pub regions: Vec<ResolvedRegion>,
    pub diagnostics: Vec<Diagnostic>,
}

impl SfzDocument {
    pub fn new(blocks: Vec<HeaderBlock>) -> Self {
        Self {
            blocks,
            control: None,
            regions: Vec::new(),
            diagnostics: Vec::new(),
        }
    }
}

/// A contiguous SFZ header block with its raw content.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeaderBlock {
    pub kind: HeaderKind,
    pub opcodes: Vec<RawOpcode>,
    pub directives: Vec<Directive>,
    pub span: Option<SourceSpan>,
}

impl HeaderBlock {
    pub fn new(kind: HeaderKind, line: usize) -> Self {
        Self {
            kind,
            opcodes: Vec::new(),
            directives: Vec::new(),
            span: Some(SourceSpan { line, column: 1 }),
        }
    }
}

/// Known SFZ header names plus a raw fallback for extensions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HeaderKind {
    Control,
    Global,
    Master,
    Group,
    Region,
    Curve,
    Effect,
    Midi,
    Unknown(String),
}

impl HeaderKind {
    pub fn from_name(name: &str) -> Self {
        match name.trim().to_ascii_lowercase().as_str() {
            "control" => Self::Control,
            "global" => Self::Global,
            "master" => Self::Master,
            "group" => Self::Group,
            "region" => Self::Region,
            "curve" => Self::Curve,
            "effect" => Self::Effect,
            "midi" => Self::Midi,
            other => Self::Unknown(other.to_string()),
        }
    }
}

/// A raw SFZ opcode assignment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawOpcode {
    pub key: String,
    pub value: String,
    pub span: Option<SourceSpan>,
}

impl RawOpcode {
    pub fn new(
        key: impl Into<String>,
        value: impl Into<String>,
        line: usize,
        column: usize,
    ) -> Self {
        Self {
            key: key.into(),
            value: value.into(),
            span: Some(SourceSpan { line, column }),
        }
    }
}

/// A preprocessor-style SFZ directive.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Directive {
    Define {
        name: String,
        value: String,
        span: Option<SourceSpan>,
    },
    Include {
        path: String,
        span: Option<SourceSpan>,
    },
}

impl Directive {
    pub fn span(&self) -> Option<SourceSpan> {
        match self {
            Self::Define { span, .. } | Self::Include { span, .. } => *span,
        }
    }
}

/// Parsed control block details for the v0.1 core subset.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Control {
    pub default_path: Option<String>,
    pub note_offset: Option<i16>,
    pub octave_offset: Option<i16>,
    pub label_cc: Vec<RawOpcode>,
    pub set_cc: Vec<RawOpcode>,
    pub defines: Vec<(String, String)>,
    pub includes: Vec<String>,
    pub raw_opcodes: Vec<RawOpcode>,
    pub unknown_opcodes: Vec<RawOpcode>,
    pub diagnostics: Vec<Diagnostic>,
}

/// A region after SFZ inheritance and typed validation.
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedRegion {
    pub raw_opcodes: Vec<RawOpcode>,
    pub typed_opcodes: Vec<Opcode>,
    pub unknown_opcodes: Vec<RawOpcode>,
    pub diagnostics: Vec<Diagnostic>,
}

/// A non-fatal parse, resolution, or validation problem.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub severity: Severity,
    pub message: String,
    pub opcode: Option<String>,
    pub value: Option<String>,
    pub span: Option<SourceSpan>,
}

impl Diagnostic {
    pub fn warning(message: impl Into<String>) -> Self {
        Self {
            severity: Severity::Warning,
            message: message.into(),
            opcode: None,
            value: None,
            span: None,
        }
    }

    pub fn error(message: impl Into<String>) -> Self {
        Self {
            severity: Severity::Error,
            message: message.into(),
            opcode: None,
            value: None,
            span: None,
        }
    }

    pub fn for_opcode(severity: Severity, message: impl Into<String>, opcode: &RawOpcode) -> Self {
        Self {
            severity,
            message: message.into(),
            opcode: Some(opcode.key.clone()),
            value: Some(opcode.value.clone()),
            span: opcode.span,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Warning,
    Error,
}

/// 1-based source location.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceSpan {
    pub line: usize,
    pub column: usize,
}

/// Top-level parse failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    pub message: String,
}

impl ParseError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for ParseError {}

/// Include loading failure returned by a caller-provided resolver.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IncludeError {
    pub message: String,
}

impl IncludeError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for IncludeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for IncludeError {}
