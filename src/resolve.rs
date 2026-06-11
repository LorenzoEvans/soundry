//! Resolution and validation pass for parsed SFZ blocks.

use crate::ast::{
    Control, Diagnostic, Directive, HeaderBlock, HeaderKind, RawOpcode, ResolvedRegion, SfzDocument,
};
use crate::opcode_mapping::{is_known_opcode_key, parse_opcode, OpcodeParse};
use std::collections::HashMap;

pub(crate) fn resolve_document(
    blocks: Vec<HeaderBlock>,
    mut diagnostics: Vec<Diagnostic>,
) -> SfzDocument {
    let control = resolve_control(&blocks, &mut diagnostics);
    add_unresolved_include_warnings(&blocks, &mut diagnostics);
    let regions = resolve_regions(&blocks);
    for region in &regions {
        diagnostics.extend(region.diagnostics.clone());
    }
    SfzDocument {
        blocks,
        control,
        regions,
        diagnostics,
    }
}

fn add_unresolved_include_warnings(blocks: &[HeaderBlock], diagnostics: &mut Vec<Diagnostic>) {
    for directive in blocks.iter().flat_map(|block| &block.directives) {
        if let Directive::Include { path, span } = directive {
            diagnostics.push(Diagnostic {
                severity: crate::ast::Severity::Warning,
                message: format!("include `{path}` was not resolved"),
                opcode: None,
                value: Some(path.clone()),
                span: *span,
            });
        }
    }
}

fn resolve_control(
    blocks: &[HeaderBlock],
    document_diagnostics: &mut Vec<Diagnostic>,
) -> Option<Control> {
    let mut control = Control::default();
    let mut found = false;

    for block in blocks
        .iter()
        .filter(|block| block.kind == HeaderKind::Control)
    {
        found = true;

        for directive in &block.directives {
            match directive {
                Directive::Define { name, value, .. } => {
                    control.defines.push((name.clone(), value.clone()));
                }
                Directive::Include { path, .. } => control.includes.push(path.clone()),
            }
        }

        for opcode in &block.opcodes {
            control.raw_opcodes.push(opcode.clone());
            match parse_opcode(opcode) {
                OpcodeParse::Known(typed) => match typed {
                    crate::opcode_mapping::Opcode::DefaultPath(value) => {
                        control.default_path = Some(value);
                    }
                    crate::opcode_mapping::Opcode::NoteOffset(value) => {
                        control.note_offset = Some(value);
                    }
                    crate::opcode_mapping::Opcode::OctaveOffset(value) => {
                        control.octave_offset = Some(value);
                    }
                    crate::opcode_mapping::Opcode::LabelCc { .. } => {
                        control.label_cc.push(opcode.clone());
                    }
                    crate::opcode_mapping::Opcode::SetCc { .. } => {
                        control.set_cc.push(opcode.clone());
                    }
                    _ => {
                        control.unknown_opcodes.push(opcode.clone());
                    }
                },
                OpcodeParse::Unknown => control.unknown_opcodes.push(opcode.clone()),
                OpcodeParse::Invalid(diagnostic) => {
                    control.diagnostics.push(diagnostic.clone());
                    document_diagnostics.push(diagnostic);
                }
            }
        }
    }

    found.then_some(control)
}

fn resolve_regions(blocks: &[HeaderBlock]) -> Vec<ResolvedRegion> {
    let mut regions = Vec::new();
    let mut global = Vec::<RawOpcode>::new();
    let mut master = Vec::<RawOpcode>::new();
    let mut group = Vec::<RawOpcode>::new();

    for block in blocks {
        match &block.kind {
            HeaderKind::Global => {
                global = block.opcodes.clone();
                master.clear();
                group.clear();
            }
            HeaderKind::Master => {
                master = block.opcodes.clone();
                group.clear();
            }
            HeaderKind::Group => {
                group = block.opcodes.clone();
            }
            HeaderKind::Region => {
                let inherited = [&global[..], &master[..], &group[..], &block.opcodes[..]]
                    .into_iter()
                    .flatten()
                    .cloned()
                    .collect::<Vec<_>>();
                let raw_opcodes = override_by_key(inherited);
                regions.push(validate_region(raw_opcodes));
            }
            _ => {}
        }
    }

    regions
}

fn override_by_key(opcodes: Vec<RawOpcode>) -> Vec<RawOpcode> {
    let mut positions = HashMap::<String, usize>::new();
    let mut output = Vec::<RawOpcode>::new();

    for opcode in opcodes {
        if let Some(index) = positions.get(&opcode.key).copied() {
            output[index] = opcode;
        } else {
            positions.insert(opcode.key.clone(), output.len());
            output.push(opcode);
        }
    }

    output
}

fn validate_region(raw_opcodes: Vec<RawOpcode>) -> ResolvedRegion {
    let mut typed_opcodes = Vec::new();
    let mut unknown_opcodes = Vec::new();
    let mut diagnostics = Vec::new();

    for opcode in &raw_opcodes {
        match parse_opcode(opcode) {
            OpcodeParse::Known(typed) => typed_opcodes.push(typed),
            OpcodeParse::Unknown => unknown_opcodes.push(opcode.clone()),
            OpcodeParse::Invalid(diagnostic) => {
                diagnostics.push(diagnostic);
                if !is_known_opcode_key(&opcode.key) {
                    unknown_opcodes.push(opcode.clone());
                }
            }
        }
    }

    ResolvedRegion {
        raw_opcodes,
        typed_opcodes,
        unknown_opcodes,
        diagnostics,
    }
}
