# Soundry v0.1 Completion Plan

## Completed In This Pass

- Library-first crate API with `parse_sfz` and `parse_sfz_with_options`.
- Raw document model preserving headers, directives, and opcode key/value pairs.
- Unknown header and unknown opcode preservation.
- Define substitution in later opcode keys and values.
- Include expansion through `IncludeResolver`.
- Recursion protection for includes.
- Region inheritance from global, master, and group blocks.
- Local region override behavior.
- Typed validation for the initial core opcode subset.
- Structured diagnostics for invalid known opcode values.
- Integration tests and a basic fixture.
- README and architecture documentation.

## Remaining v0.1 Hardening

- Add more fixtures from real SFZ libraries.
- Improve inline parsing for rare constructs where values intentionally contain
  token-like `key=value` text.
- Add optional filesystem include resolver helpers.
- Add richer source spans for multiline and included files.
- Decide whether typed control and curve summaries should become first-class
  document views like resolved regions.
- Audit SFZ inheritance behavior against more players and document any
  intentional simplifications.

## Future Opcode Coverage

The current opcode enum is intentionally small. Future work should add typed
coverage in focused groups:

- MIDI conditions.
- Filters.
- Pitch.
- Envelopes.
- LFOs.
- Effects.
- Player-specific extensions where they can be represented without losing raw
  fallback data.
