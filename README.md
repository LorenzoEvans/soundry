# Soundry

Soundry is an experimental Rust parser library for the SFZ instrument format.
The current v0.1 scope focuses on a reliable parser architecture: preserve raw
SFZ structure, resolve practical inheritance, validate a useful typed opcode
subset, and keep unsupported data available for downstream tools.

Soundry uses a three-layer model:

- Raw parse layer: headers, directives, and opcode key/value pairs are preserved.
- Resolution layer: `#define`, optional `#include`, and region inheritance are applied.
- Typed validation layer: known core opcodes are converted to Rust enum variants and invalid values produce diagnostics.

## Current Scope

Soundry currently supports these headers:

- `<control>`
- `<global>`
- `<master>`
- `<group>`
- `<region>`
- `<curve>`
- `<effect>`
- `<midi>`
- Unknown headers, preserved as `HeaderKind::Unknown`

The typed v0.1 opcode subset includes:

- Sample/file: `sample`
- Key and velocity: `key`, `lokey`, `hikey`, `lovel`, `hivel`, `pitch_keycenter`
- Playback and loop points: `offset`, `end`, `loop_mode`, `loop_start`, `loop_end`
- Triggering and grouping: `trigger`, `group`, `off_by`, `off_mode`, `seq_position`, `seq_length`
- Amplifier/mix: `volume`, `pan`, `amp_veltrack`
- Control: `default_path`, `note_offset`, `octave_offset`, `label_ccN`, `label_cc$VAR`, `set_ccN`
- Curves: `curve_index`, `v000` through `v127`

All other opcodes remain available as raw opcode data. Unknown opcodes are not
errors.

## Non-Goals

Soundry v0.1 does not implement audio playback, the full SFZ v1/v2 opcode
universe, or player-specific behavior. It also does not reject an entire SFZ
document because one known opcode has an invalid value; invalid known values are
reported through diagnostics.

## Usage

Add Soundry to your `Cargo.toml`:

```toml
[dependencies]
soundry = "0.1.0"
```

Parse a document:

```rust
use soundry::{parse_sfz, Opcode};

let sfz = r#"
<control>
default_path=Samples/
#define $EXT wav

<global>
volume=-3

<group>
lovel=0
hivel=127

<region>
sample=kick.$EXT
key=36
custom_opcode=preserved
"#;

let document = parse_sfz(sfz)?;

assert_eq!(document.regions.len(), 1);
assert!(document.regions[0]
    .typed_opcodes
    .contains(&Opcode::Sample("kick.wav".into())));
assert!(document.regions[0]
    .unknown_opcodes
    .iter()
    .any(|opcode| opcode.key == "custom_opcode"));
# Ok::<(), soundry::ParseError>(())
```

Inspect resolved regions:

```rust
use soundry::{parse_sfz, Opcode};

let document = parse_sfz(
    r#"
<global>
volume=-3
<group>
lovel=10
<region>
sample=snare.wav
key=38
"#,
)?;

let region = &document.regions[0];
assert!(region.typed_opcodes.contains(&Opcode::Volume(-3.0)));
assert!(region.typed_opcodes.contains(&Opcode::LoVel(10)));
assert!(region.typed_opcodes.contains(&Opcode::Key(38)));
# Ok::<(), soundry::ParseError>(())
```

## Includes and Defines

`#define` directives are applied to later opcode keys and values:

```sfz
#define $EXT wav
sample=kick.$EXT
```

Use `parse_sfz_with_options` and an `IncludeResolver` to load includes. The
parser does not hardwire filesystem access.

```rust
use soundry::{parse_sfz_with_options, IncludeError, IncludeResolver, ParseOptions};

struct Resolver;

impl IncludeResolver for Resolver {
    fn resolve(&self, path: &str) -> Result<String, IncludeError> {
        match path {
            "kick.sfz" => Ok("<region>\nsample=kick.wav\nkey=36\n".into()),
            _ => Err(IncludeError::new("unknown include")),
        }
    }
}

let resolver = Resolver;
let document = parse_sfz_with_options(
    r#"#include "kick.sfz""#,
    ParseOptions::with_include_resolver(&resolver),
)?;

assert_eq!(document.regions.len(), 1);
# Ok::<(), soundry::ParseError>(())
```

If no resolver is supplied, include directives are preserved and a warning
diagnostic is emitted.

## Diagnostics

Known opcode validation failures are reported as structured diagnostics:

```rust
use soundry::{parse_sfz, Severity};

let document = parse_sfz("<region>\nkey=128")?;
assert_eq!(document.regions[0].diagnostics[0].severity, Severity::Error);
# Ok::<(), soundry::ParseError>(())
```

## Roadmap

- Add more typed SFZ v1/v2 opcode groups.
- Improve source spans beyond line/column starts.
- Add filesystem include resolver helpers behind an opt-in API.
- Expand compatibility fixtures from real-world SFZ libraries.
- Document exact player-specific differences where Soundry intentionally stays neutral.

## Reference Material

- <https://sfzformat.com/headers/>
- <https://sfzformat.com/tutorials/basics/>
- <https://www.linuxsampler.org/sfz/>
- <https://github.com/sfz/tests>
