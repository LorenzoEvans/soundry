use soundry::{
    parse_sfz, parse_sfz_with_options, HeaderKind, IncludeError, IncludeResolver, Opcode,
    ParseOptions, Severity,
};
use std::collections::HashMap;

#[test]
fn parses_basic_region() {
    let document = parse_sfz("<region> sample=kick.wav key=36").unwrap();

    assert_eq!(document.regions.len(), 1);
    assert!(document.regions[0]
        .typed_opcodes
        .contains(&Opcode::Sample("kick.wav".into())));
    assert!(document.regions[0].typed_opcodes.contains(&Opcode::Key(36)));
}

#[test]
fn parses_control_header() {
    let document = parse_sfz(
        r#"
<control>
default_path=Samples/
note_offset=1
octave_offset=-1
label_cc7=Volume
set_cc1=64
"#,
    )
    .unwrap();

    let control = document.control.unwrap();
    assert_eq!(control.default_path.as_deref(), Some("Samples/"));
    assert_eq!(control.note_offset, Some(1));
    assert_eq!(control.octave_offset, Some(-1));
    assert_eq!(control.label_cc.len(), 1);
    assert_eq!(control.set_cc.len(), 1);
}

#[test]
fn preserves_unknown_opcode() {
    let document = parse_sfz("<region>\nsample=kick.wav\nx_custom=abc").unwrap();
    let region = &document.regions[0];

    assert_eq!(region.unknown_opcodes[0].key, "x_custom");
    assert_eq!(region.unknown_opcodes[0].value, "abc");
    assert!(region
        .raw_opcodes
        .iter()
        .any(|opcode| opcode.key == "x_custom"));
}

#[test]
fn preserves_unknown_header() {
    let document = parse_sfz("<foo>\nbar=baz\n<region>\nsample=a.wav").unwrap();

    assert_eq!(document.blocks[0].kind, HeaderKind::Unknown("foo".into()));
    assert_eq!(document.blocks[0].opcodes[0].key, "bar");
}

#[test]
fn resolves_global_group_region_inheritance() {
    let document = parse_sfz(
        r#"
<global>
volume=-3
<master>
pan=5
<group>
lovel=10
hivel=100
<region>
sample=snare.wav
key=38
"#,
    )
    .unwrap();

    let region = &document.regions[0];
    assert!(region.typed_opcodes.contains(&Opcode::Volume(-3.0)));
    assert!(region.typed_opcodes.contains(&Opcode::Pan(5.0)));
    assert!(region.typed_opcodes.contains(&Opcode::LoVel(10)));
    assert!(region.typed_opcodes.contains(&Opcode::HiVel(100)));
    assert!(region.typed_opcodes.contains(&Opcode::Key(38)));
}

#[test]
fn local_region_overrides_inherited_opcode() {
    let document = parse_sfz(
        r#"
<global>
volume=-3
key=36
<region>
sample=snare.wav
volume=-1
key=38
"#,
    )
    .unwrap();

    let region = &document.regions[0];
    assert!(region.typed_opcodes.contains(&Opcode::Volume(-1.0)));
    assert!(!region.typed_opcodes.contains(&Opcode::Volume(-3.0)));
    assert!(region.typed_opcodes.contains(&Opcode::Key(38)));
    assert!(!region.typed_opcodes.contains(&Opcode::Key(36)));
}

#[test]
fn substitutes_defines_in_later_keys_and_values() {
    let document = parse_sfz(
        r#"
<control>
#define $EXT wav
#define $RELEASE 72
label_cc$RELEASE=Release ($RELEASE)
<region>
sample=kick.$EXT
"#,
    )
    .unwrap();

    let control = document.control.unwrap();
    assert_eq!(control.label_cc[0].key, "label_cc72");
    assert_eq!(control.label_cc[0].value, "Release (72)");
    assert!(document.regions[0]
        .typed_opcodes
        .contains(&Opcode::Sample("kick.wav".into())));
}

#[test]
fn resolves_includes_with_fake_resolver() {
    struct FakeResolver(HashMap<String, String>);

    impl IncludeResolver for FakeResolver {
        fn resolve(&self, path: &str) -> Result<String, IncludeError> {
            self.0
                .get(path)
                .cloned()
                .ok_or_else(|| IncludeError::new("missing include"))
        }
    }

    let resolver = FakeResolver(HashMap::from([(
        "part.sfz".to_string(),
        "<region>\nsample=hat.wav\nkey=42\n".to_string(),
    )]));
    let document = parse_sfz_with_options(
        r#"#include "part.sfz""#,
        ParseOptions::with_include_resolver(&resolver),
    )
    .unwrap();

    assert_eq!(document.regions.len(), 1);
    assert!(document.regions[0]
        .typed_opcodes
        .contains(&Opcode::Sample("hat.wav".into())));
    assert!(document
        .diagnostics
        .iter()
        .all(|diagnostic| !diagnostic.message.contains("not resolved")));
}

#[test]
fn invalid_midi_key_produces_diagnostic() {
    let document = parse_sfz("<region>\nkey=128").unwrap();

    assert_eq!(document.regions[0].diagnostics.len(), 1);
    assert_eq!(document.regions[0].diagnostics[0].severity, Severity::Error);
    assert_eq!(
        document.regions[0].diagnostics[0].opcode.as_deref(),
        Some("key")
    );
    assert!(document
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.opcode.as_deref() == Some("key")));
}

#[test]
fn parses_dynamic_control_label_syntax() {
    let document = parse_sfz(
        r#"
<control>
#define $RELEASE 72
label_cc$RELEASE=Release ($RELEASE)
"#,
    )
    .unwrap();

    let control = document.control.unwrap();
    assert_eq!(control.label_cc[0].key, "label_cc72");
    assert_eq!(control.label_cc[0].value, "Release (72)");
}

#[test]
fn parses_curve_values() {
    let document = parse_sfz(
        r#"
<curve>
curve_index=1
v000=0
v127=1
"#,
    )
    .unwrap();

    let curve = &document.blocks[0];
    assert_eq!(curve.kind, HeaderKind::Curve);
    assert_eq!(curve.opcodes.len(), 3);
    assert_eq!(curve.opcodes[1].key, "v000");
    assert_eq!(curve.opcodes[2].key, "v127");
}

#[test]
fn fixture_and_readme_style_example_parse() {
    let document = parse_sfz(include_str!("fixtures/basic.sfz")).unwrap();

    assert_eq!(document.regions.len(), 1);
    assert!(document.control.is_some());
    assert!(document.regions[0]
        .unknown_opcodes
        .iter()
        .any(|opcode| opcode.key == "custom_opcode"));
    assert!(document.regions[0]
        .typed_opcodes
        .contains(&Opcode::Sample("kick.wav".into())));
    assert!(document.regions[0].typed_opcodes.contains(&Opcode::Key(36)));
}
