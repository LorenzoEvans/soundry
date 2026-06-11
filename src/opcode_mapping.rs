//! Typed v0.1 opcode subset and validation.

use crate::ast::{Diagnostic, RawOpcode, Severity};

#[derive(Debug, Clone, PartialEq)]
pub enum Opcode {
    Sample(String),
    Key(u8),
    LoKey(u8),
    HiKey(u8),
    LoVel(u8),
    HiVel(u8),
    PitchKeycenter(u8),
    Offset(u32),
    End(u32),
    LoopMode(LoopMode),
    LoopStart(u32),
    LoopEnd(u32),
    Trigger(Trigger),
    Group(i32),
    OffBy(i32),
    OffMode(OffMode),
    SeqPosition(u32),
    SeqLength(u32),
    Volume(f32),
    Pan(f32),
    AmpVeltrack(f32),
    DefaultPath(String),
    NoteOffset(i16),
    OctaveOffset(i16),
    LabelCc { cc: ControlIndex, label: String },
    SetCc { cc: u8, value: f32 },
    CurveIndex(u32),
    CurveValue { index: u8, value: f32 },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoopMode {
    NoLoop,
    OneShot,
    LoopContinuous,
    LoopSustain,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Trigger {
    Attack,
    Release,
    First,
    Legato,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OffMode {
    Fast,
    Normal,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ControlIndex {
    Number(u8),
    Variable(String),
}

pub(crate) enum OpcodeParse {
    Known(Opcode),
    Unknown,
    Invalid(Diagnostic),
}

pub(crate) fn parse_opcode(opcode: &RawOpcode) -> OpcodeParse {
    match opcode.key.as_str() {
        "sample" => OpcodeParse::Known(Opcode::Sample(opcode.value.clone())),
        "key" => parse_midi_u8(opcode, Opcode::Key),
        "lokey" => parse_midi_u8(opcode, Opcode::LoKey),
        "hikey" => parse_midi_u8(opcode, Opcode::HiKey),
        "lovel" => parse_midi_u8(opcode, Opcode::LoVel),
        "hivel" => parse_midi_u8(opcode, Opcode::HiVel),
        "pitch_keycenter" => parse_midi_u8(opcode, Opcode::PitchKeycenter),
        "offset" => parse_u32(opcode, Opcode::Offset),
        "end" => parse_u32(opcode, Opcode::End),
        "loop_start" => parse_u32(opcode, Opcode::LoopStart),
        "loop_end" => parse_u32(opcode, Opcode::LoopEnd),
        "loop_mode" => parse_loop_mode(opcode),
        "trigger" => parse_trigger(opcode),
        "group" => parse_i32(opcode, Opcode::Group),
        "off_by" => parse_i32(opcode, Opcode::OffBy),
        "off_mode" => parse_off_mode(opcode),
        "seq_position" => parse_u32(opcode, Opcode::SeqPosition),
        "seq_length" => parse_u32(opcode, Opcode::SeqLength),
        "volume" => parse_f32(opcode, Opcode::Volume),
        "pan" => parse_f32(opcode, Opcode::Pan),
        "amp_veltrack" => parse_f32(opcode, Opcode::AmpVeltrack),
        "default_path" => OpcodeParse::Known(Opcode::DefaultPath(opcode.value.clone())),
        "note_offset" => parse_i16(opcode, Opcode::NoteOffset),
        "octave_offset" => parse_i16(opcode, Opcode::OctaveOffset),
        "curve_index" => parse_u32(opcode, Opcode::CurveIndex),
        key if key.starts_with("label_cc") => parse_label_cc(opcode),
        key if key.starts_with("set_cc") => parse_set_cc(opcode),
        key if is_curve_value_key(key) => parse_curve_value(opcode),
        _ => OpcodeParse::Unknown,
    }
}

pub(crate) fn is_known_opcode_key(key: &str) -> bool {
    matches!(
        key,
        "sample"
            | "key"
            | "lokey"
            | "hikey"
            | "lovel"
            | "hivel"
            | "pitch_keycenter"
            | "offset"
            | "end"
            | "loop_mode"
            | "loop_start"
            | "loop_end"
            | "trigger"
            | "group"
            | "off_by"
            | "off_mode"
            | "seq_position"
            | "seq_length"
            | "volume"
            | "pan"
            | "amp_veltrack"
            | "default_path"
            | "note_offset"
            | "octave_offset"
            | "curve_index"
    ) || key.starts_with("label_cc")
        || key.starts_with("set_cc")
        || is_curve_value_key(key)
}

fn parse_midi_u8(opcode: &RawOpcode, build: impl FnOnce(u8) -> Opcode) -> OpcodeParse {
    match opcode.value.parse::<u16>() {
        Ok(value) if value <= 127 => OpcodeParse::Known(build(value as u8)),
        Ok(_) => OpcodeParse::Invalid(Diagnostic::for_opcode(
            Severity::Error,
            "value must be in MIDI range 0..=127",
            opcode,
        )),
        Err(_) => invalid_number(opcode, "expected integer in MIDI range 0..=127"),
    }
}

fn parse_u32(opcode: &RawOpcode, build: impl FnOnce(u32) -> Opcode) -> OpcodeParse {
    match opcode.value.parse::<u32>() {
        Ok(value) => OpcodeParse::Known(build(value)),
        Err(_) => invalid_number(opcode, "expected unsigned integer"),
    }
}

fn parse_i32(opcode: &RawOpcode, build: impl FnOnce(i32) -> Opcode) -> OpcodeParse {
    match opcode.value.parse::<i32>() {
        Ok(value) => OpcodeParse::Known(build(value)),
        Err(_) => invalid_number(opcode, "expected integer"),
    }
}

fn parse_i16(opcode: &RawOpcode, build: impl FnOnce(i16) -> Opcode) -> OpcodeParse {
    match opcode.value.parse::<i16>() {
        Ok(value) => OpcodeParse::Known(build(value)),
        Err(_) => invalid_number(opcode, "expected integer"),
    }
}

fn parse_f32(opcode: &RawOpcode, build: impl FnOnce(f32) -> Opcode) -> OpcodeParse {
    match opcode.value.parse::<f32>() {
        Ok(value) => OpcodeParse::Known(build(value)),
        Err(_) => invalid_number(opcode, "expected number"),
    }
}

fn parse_loop_mode(opcode: &RawOpcode) -> OpcodeParse {
    let mode = match opcode.value.as_str() {
        "no_loop" => LoopMode::NoLoop,
        "one_shot" => LoopMode::OneShot,
        "loop_continuous" => LoopMode::LoopContinuous,
        "loop_sustain" => LoopMode::LoopSustain,
        _ => {
            return OpcodeParse::Invalid(Diagnostic::for_opcode(
                Severity::Error,
                "unknown loop_mode",
                opcode,
            ))
        }
    };
    OpcodeParse::Known(Opcode::LoopMode(mode))
}

fn parse_trigger(opcode: &RawOpcode) -> OpcodeParse {
    let trigger = match opcode.value.as_str() {
        "attack" => Trigger::Attack,
        "release" => Trigger::Release,
        "first" => Trigger::First,
        "legato" => Trigger::Legato,
        _ => {
            return OpcodeParse::Invalid(Diagnostic::for_opcode(
                Severity::Error,
                "unknown trigger",
                opcode,
            ))
        }
    };
    OpcodeParse::Known(Opcode::Trigger(trigger))
}

fn parse_off_mode(opcode: &RawOpcode) -> OpcodeParse {
    let mode = match opcode.value.as_str() {
        "fast" => OffMode::Fast,
        "normal" => OffMode::Normal,
        _ => {
            return OpcodeParse::Invalid(Diagnostic::for_opcode(
                Severity::Error,
                "unknown off_mode",
                opcode,
            ))
        }
    };
    OpcodeParse::Known(Opcode::OffMode(mode))
}

fn parse_label_cc(opcode: &RawOpcode) -> OpcodeParse {
    let suffix = &opcode.key["label_cc".len()..];
    match parse_cc_suffix(opcode, suffix) {
        Ok(cc) => OpcodeParse::Known(Opcode::LabelCc {
            cc,
            label: opcode.value.clone(),
        }),
        Err(diagnostic) => OpcodeParse::Invalid(diagnostic),
    }
}

fn parse_set_cc(opcode: &RawOpcode) -> OpcodeParse {
    let suffix = &opcode.key["set_cc".len()..];
    let cc = match suffix.parse::<u16>() {
        Ok(value) if value <= 127 => value as u8,
        Ok(_) => {
            return OpcodeParse::Invalid(Diagnostic::for_opcode(
                Severity::Error,
                "CC number must be in range 0..=127",
                opcode,
            ))
        }
        Err(_) => return invalid_number(opcode, "expected set_ccN with numeric N"),
    };

    match opcode.value.parse::<f32>() {
        Ok(value) => OpcodeParse::Known(Opcode::SetCc { cc, value }),
        Err(_) => invalid_number(opcode, "expected numeric CC value"),
    }
}

fn parse_cc_suffix(opcode: &RawOpcode, suffix: &str) -> OpcodeParseResult<ControlIndex> {
    if let Some(variable) = suffix.strip_prefix('$') {
        if variable.is_empty() {
            return Err(Diagnostic::for_opcode(
                Severity::Error,
                "empty CC variable name",
                opcode,
            ));
        }
        return Ok(ControlIndex::Variable(variable.to_string()));
    }

    match suffix.parse::<u16>() {
        Ok(value) if value <= 127 => Ok(ControlIndex::Number(value as u8)),
        Ok(_) => Err(Diagnostic::for_opcode(
            Severity::Error,
            "CC number must be in range 0..=127",
            opcode,
        )),
        Err(_) => Err(Diagnostic::for_opcode(
            Severity::Error,
            "expected label_ccN or label_cc$VAR",
            opcode,
        )),
    }
}

fn parse_curve_value(opcode: &RawOpcode) -> OpcodeParse {
    let index = opcode.key[1..].parse::<u8>().unwrap_or_default();
    match opcode.value.parse::<f32>() {
        Ok(value) => OpcodeParse::Known(Opcode::CurveValue { index, value }),
        Err(_) => invalid_number(opcode, "expected curve value number"),
    }
}

fn is_curve_value_key(key: &str) -> bool {
    key.len() == 4
        && key.starts_with('v')
        && key[1..].chars().all(|ch| ch.is_ascii_digit())
        && key[1..].parse::<u8>().is_ok_and(|value| value <= 127)
}

fn invalid_number(opcode: &RawOpcode, message: &str) -> OpcodeParse {
    OpcodeParse::Invalid(Diagnostic::for_opcode(Severity::Error, message, opcode))
}

type OpcodeParseResult<T> = Result<T, Diagnostic>;
