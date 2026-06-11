//! Raw SFZ parser plus define/include expansion.

use crate::ast::{
    Diagnostic, Directive, HeaderBlock, HeaderKind, IncludeError, ParseError, RawOpcode, SourceSpan,
};
use std::collections::{HashMap, HashSet};

/// Caller-provided include loading.
pub trait IncludeResolver {
    fn resolve(&self, path: &str) -> Result<String, IncludeError>;
}

/// Options for parsing SFZ text.
#[derive(Default)]
pub struct ParseOptions<'a> {
    pub include_resolver: Option<&'a dyn IncludeResolver>,
    pub max_include_depth: usize,
}

impl<'a> ParseOptions<'a> {
    pub fn with_include_resolver(resolver: &'a dyn IncludeResolver) -> Self {
        Self {
            include_resolver: Some(resolver),
            max_include_depth: 16,
        }
    }
}

pub(crate) fn parse_raw_document(
    input: &str,
) -> Result<(Vec<HeaderBlock>, Vec<Diagnostic>), ParseError> {
    parse_raw_document_named(input, "<input>")
}

pub(crate) fn expand_and_parse(
    input: &str,
    options: &ParseOptions<'_>,
) -> Result<(Vec<HeaderBlock>, Vec<Diagnostic>), ParseError> {
    let mut diagnostics = Vec::new();
    let mut visited = HashSet::new();
    let expanded = expand_includes(input, "<input>", options, 0, &mut visited, &mut diagnostics)?;
    let (blocks, mut parse_diagnostics) = parse_raw_document_named(&expanded, "<expanded>")?;
    diagnostics.append(&mut parse_diagnostics);
    Ok((blocks, diagnostics))
}

fn expand_includes(
    input: &str,
    source_name: &str,
    options: &ParseOptions<'_>,
    depth: usize,
    visited: &mut HashSet<String>,
    diagnostics: &mut Vec<Diagnostic>,
) -> Result<String, ParseError> {
    if depth > options.max_include_depth {
        diagnostics.push(Diagnostic::warning(format!(
            "include depth limit reached while parsing {source_name}"
        )));
        return Ok(String::new());
    }

    let mut output = String::new();
    for (line_index, raw_line) in strip_block_comments(input).lines().enumerate() {
        let line_number = line_index + 1;
        let line = strip_line_comment(raw_line).trim();
        if let Some(directive) = parse_directive(line, line_number, 1) {
            match directive {
                Directive::Include { path, span } => {
                    if let Some(resolver) = options.include_resolver {
                        if !visited.insert(path.clone()) {
                            diagnostics.push(Diagnostic {
                                severity: crate::ast::Severity::Warning,
                                message: format!("skipping recursive include `{path}`"),
                                opcode: None,
                                value: Some(path),
                                span,
                            });
                            continue;
                        }
                        match resolver.resolve(&path) {
                            Ok(content) => {
                                let nested = expand_includes(
                                    &content,
                                    &path,
                                    options,
                                    depth + 1,
                                    visited,
                                    diagnostics,
                                )?;
                                output.push_str(&nested);
                                if !nested.ends_with('\n') {
                                    output.push('\n');
                                }
                            }
                            Err(error) => diagnostics.push(Diagnostic {
                                severity: crate::ast::Severity::Error,
                                message: format!("failed to include `{path}`: {error}"),
                                opcode: None,
                                value: Some(path.clone()),
                                span,
                            }),
                        }
                        visited.remove(&path);
                    } else {
                        diagnostics.push(Diagnostic {
                            severity: crate::ast::Severity::Warning,
                            message: format!("include `{path}` was not resolved"),
                            opcode: None,
                            value: Some(path),
                            span,
                        });
                        output.push_str(raw_line);
                        output.push('\n');
                    }
                }
                Directive::Define { .. } => {
                    output.push_str(raw_line);
                    output.push('\n');
                }
            }
        } else {
            output.push_str(raw_line);
            output.push('\n');
        }
    }

    Ok(output)
}

fn parse_raw_document_named(
    input: &str,
    _source_name: &str,
) -> Result<(Vec<HeaderBlock>, Vec<Diagnostic>), ParseError> {
    let cleaned = strip_block_comments(input);
    let mut blocks = Vec::new();
    let mut current: Option<HeaderBlock> = None;
    let mut defines: HashMap<String, String> = HashMap::new();
    let mut diagnostics = Vec::new();

    for (line_index, raw_line) in cleaned.lines().enumerate() {
        let line_number = line_index + 1;
        let line = strip_line_comment(raw_line);
        let mut cursor = 0;

        while cursor < line.len() {
            cursor = skip_spaces(line, cursor);
            if cursor >= line.len() {
                break;
            }

            let rest = &line[cursor..];
            if rest.starts_with('<') {
                if let Some(end) = rest.find('>') {
                    if let Some(block) = current.take() {
                        blocks.push(block);
                    }
                    let name = &rest[1..end];
                    current = Some(HeaderBlock::new(HeaderKind::from_name(name), line_number));
                    cursor += end + 1;
                    continue;
                }
                diagnostics.push(Diagnostic::error(format!(
                    "unterminated header on line {line_number}"
                )));
                break;
            }

            if rest.starts_with('#') {
                if let Some(directive) = parse_directive(rest.trim(), line_number, cursor + 1) {
                    if let Directive::Define { name, value, .. } = &directive {
                        defines.insert(name.clone(), value.clone());
                    }
                    ensure_block(&mut current, line_number)
                        .directives
                        .push(substitute_directive(directive, &defines));
                } else {
                    diagnostics.push(Diagnostic::warning(format!(
                        "unsupported directive on line {line_number}"
                    )));
                }
                break;
            }

            if let Some((opcode, next_cursor)) = parse_opcode_at(line, cursor, line_number) {
                let opcode = substitute_opcode(opcode, &defines);
                ensure_block(&mut current, line_number).opcodes.push(opcode);
                cursor = next_cursor;
            } else {
                diagnostics.push(Diagnostic::warning(format!(
                    "could not parse content on line {line_number}: {}",
                    rest.trim()
                )));
                break;
            }
        }
    }

    if let Some(block) = current {
        blocks.push(block);
    }

    Ok((blocks, diagnostics))
}

fn ensure_block(current: &mut Option<HeaderBlock>, line: usize) -> &mut HeaderBlock {
    current.get_or_insert_with(|| HeaderBlock::new(HeaderKind::Unknown("preamble".into()), line))
}

fn parse_opcode_at(line: &str, cursor: usize, line_number: usize) -> Option<(RawOpcode, usize)> {
    let key_start = cursor;
    let mut key_end = key_start;
    for (offset, ch) in line[key_start..].char_indices() {
        if is_key_char(ch) {
            key_end = key_start + offset + ch.len_utf8();
        } else {
            break;
        }
    }

    if key_end == key_start {
        return None;
    }

    let mut pos = skip_spaces(line, key_end);
    if !line[pos..].starts_with('=') {
        return None;
    }
    pos += 1;
    pos = skip_spaces(line, pos);
    let value_start = pos;
    let mut value_end = line.len();
    let mut scan = pos;

    while scan < line.len() {
        let Some(ch) = line[scan..].chars().next() else {
            break;
        };
        if ch.is_whitespace() {
            let next = skip_spaces(line, scan + ch.len_utf8());
            if looks_like_opcode_start(line, next) || line[next..].starts_with('<') {
                value_end = scan;
                break;
            }
        }
        scan += ch.len_utf8();
    }

    let key = line[key_start..key_end].trim();
    let value = line[value_start..value_end].trim();
    let opcode = RawOpcode::new(key, value, line_number, key_start + 1);
    Some((opcode, value_end))
}

fn looks_like_opcode_start(line: &str, cursor: usize) -> bool {
    let mut pos = cursor;
    let mut saw_key = false;
    while pos < line.len() {
        let Some(ch) = line[pos..].chars().next() else {
            break;
        };
        if is_key_char(ch) {
            saw_key = true;
            pos += ch.len_utf8();
        } else {
            break;
        }
    }
    saw_key && line[skip_spaces(line, pos)..].starts_with('=')
}

fn parse_directive(line: &str, line_number: usize, column: usize) -> Option<Directive> {
    let trimmed = line.trim();
    let span = Some(SourceSpan {
        line: line_number,
        column,
    });

    if let Some(rest) = trimmed.strip_prefix("#define") {
        let rest = rest.trim();
        let mut parts = rest.splitn(2, char::is_whitespace);
        let name = parts.next()?.trim().to_string();
        let value = parts.next().unwrap_or("").trim().to_string();
        if name.is_empty() {
            return None;
        }
        return Some(Directive::Define { name, value, span });
    }

    if let Some(rest) = trimmed.strip_prefix("#include") {
        let rest = rest.trim().strip_prefix('=').unwrap_or(rest.trim()).trim();
        let path = rest.trim_matches('"').trim_matches('\'').to_string();
        if path.is_empty() {
            return None;
        }
        return Some(Directive::Include { path, span });
    }

    None
}

fn substitute_opcode(mut opcode: RawOpcode, defines: &HashMap<String, String>) -> RawOpcode {
    opcode.key = substitute_defines(&opcode.key, defines);
    opcode.value = substitute_defines(&opcode.value, defines);
    opcode
}

fn substitute_directive(directive: Directive, defines: &HashMap<String, String>) -> Directive {
    match directive {
        Directive::Define { name, value, span } => Directive::Define {
            name,
            value: substitute_defines(&value, defines),
            span,
        },
        Directive::Include { path, span } => Directive::Include {
            path: substitute_defines(&path, defines),
            span,
        },
    }
}

fn substitute_defines(input: &str, defines: &HashMap<String, String>) -> String {
    let mut output = input.to_string();
    for (name, value) in defines {
        output = output.replace(name, value);
    }
    output
}

fn strip_line_comment(line: &str) -> &str {
    line.split_once("//").map_or(line, |(prefix, _)| prefix)
}

fn strip_block_comments(input: &str) -> String {
    let mut output = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();
    let mut in_comment = false;

    while let Some(ch) = chars.next() {
        if in_comment {
            if ch == '*' && chars.peek() == Some(&'/') {
                chars.next();
                in_comment = false;
            } else if ch == '\n' {
                output.push('\n');
            }
            continue;
        }

        if ch == '/' && chars.peek() == Some(&'*') {
            chars.next();
            in_comment = true;
            continue;
        }

        output.push(ch);
    }

    output
}

fn skip_spaces(line: &str, mut cursor: usize) -> usize {
    while cursor < line.len() {
        let Some(ch) = line[cursor..].chars().next() else {
            break;
        };
        if !ch.is_whitespace() {
            break;
        }
        cursor += ch.len_utf8();
    }
    cursor
}

fn is_key_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || matches!(ch, '_' | '$' | '.')
}
