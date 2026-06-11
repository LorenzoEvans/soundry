# Soundry Architecture

Soundry uses three separate layers so real-world SFZ files can be parsed without
losing unsupported data.

## Raw Parse Layer

The raw parser reads SFZ text into `HeaderBlock` values. Each block has a
`HeaderKind`, raw `RawOpcode` assignments, and `Directive` entries for
`#define` and `#include`.

This layer deliberately preserves:

- Unknown headers as `HeaderKind::Unknown`.
- Unknown opcodes as raw key/value pairs.
- Invalid known opcode values as raw key/value pairs.
- Source line and column starts where available.

The raw layer is intentionally permissive. SFZ files often contain
player-specific extensions, and rejecting those extensions would make Soundry
less useful as a parser library.

## Resolution Layer

The resolution layer turns raw blocks into document-level views:

- Defines are substituted into later opcode keys and values.
- Includes can be expanded through a caller-provided `IncludeResolver`.
- Global, master, and group opcode state is inherited by following regions.
- Region-local opcodes override inherited opcodes with the same key.

Soundry v0.1 implements a simple document-order inheritance model:

1. `<global>` resets global state and clears master/group state.
2. `<master>` resets master state and clears group state.
3. `<group>` resets group state.
4. `<region>` receives global, master, group, then local opcodes.
5. Later opcodes with the same key override earlier inherited values.

## Typed Validation Layer

The validation layer maps a practical v0.1 opcode subset into the `Opcode` enum.
Known invalid values produce `Diagnostic` entries. Unknown opcodes are not
diagnostics and remain available through `unknown_opcodes`.

Examples:

- `key=36` becomes `Opcode::Key(36)`.
- `key=128` stays in `raw_opcodes` and emits an error diagnostic.
- `custom_filter=abc` stays in `unknown_opcodes` without error.

## Adding New Opcodes

To add a typed opcode:

1. Add a variant to `Opcode` or a supporting enum in `src/opcode_mapping.rs`.
2. Add the key to `parse_opcode`.
3. Add validation helpers if the value has range or enum constraints.
4. Add the key to `is_known_opcode_key`.
5. Add tests for valid parsing, invalid diagnostics, and raw preservation.

Do not remove raw storage when adding typed support. Typed opcodes are a view
over preserved source data, not a replacement for it.
