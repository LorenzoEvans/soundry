//! Soundry is a raw-preserving SFZ parser library.
//!
//! The crate parses SFZ text into a document AST, resolves region inheritance,
//! validates a practical v0.1 core opcode subset, and keeps unsupported headers
//! and opcodes available as raw data.

pub mod ast;
pub mod opcode_mapping;
pub mod parser;
mod resolve;

pub use ast::{
    Control, Diagnostic, Directive, HeaderBlock, HeaderKind, IncludeError, ParseError, RawOpcode,
    ResolvedRegion, Severity, SfzDocument, SourceSpan,
};
pub use opcode_mapping::{ControlIndex, LoopMode, OffMode, Opcode, Trigger};
pub use parser::{IncludeResolver, ParseOptions};

/// Parse SFZ text without resolving includes.
///
/// `#include` directives are preserved in the raw document and reported as
/// warnings because no include resolver was supplied. `#define` directives are
/// applied to later raw opcode keys and values.
pub fn parse_sfz(input: &str) -> Result<SfzDocument, ParseError> {
    let options = ParseOptions {
        include_resolver: None,
        max_include_depth: 16,
    };
    parse_sfz_with_options(input, options)
}

/// Parse SFZ text with caller-provided options.
///
/// Provide an [`IncludeResolver`] through [`ParseOptions`] to expand includes
/// before raw parsing and resolution. The returned document exposes raw blocks,
/// resolved regions, typed opcodes, unknown opcodes, and diagnostics.
pub fn parse_sfz_with_options(
    input: &str,
    options: ParseOptions<'_>,
) -> Result<SfzDocument, ParseError> {
    let (blocks, diagnostics) = if options.include_resolver.is_some() {
        parser::expand_and_parse(input, &options)?
    } else {
        parser::parse_raw_document(input)?
    };
    Ok(resolve::resolve_document(blocks, diagnostics))
}
