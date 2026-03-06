# factorio-mod-settings-converter

A command-line tool to convert Factorio `mod-settings.dat` files to JSON/YAML and back.

## Features

- Convert `mod-settings.dat` to JSON or YAML for easy editing.
- Convert JSON or YAML back to `mod-settings.dat`.
- Support for Factorio 2.0+ settings format (including `has_quality` flag).
- Preserves value types (numbers, booleans, strings, lists, dictionaries).

## Installation

### From Source

Ensure you have [Rust](https://rustup.rs/) installed.

```bash
cargo install --path .
```

## Usage

### Convert DAT to JSON

```bash
factorio-mod-settings-converter mod-settings.dat
# Creates mod-settings.json
```

### Convert DAT to YAML

```bash
factorio-mod-settings-converter mod-settings.dat mod-settings.yaml
```

### Convert JSON/YAML to DAT

```bash
factorio-mod-settings-converter mod-settings.json
# Creates mod-settings.dat
```

### Specify Output

```bash
factorio-mod-settings-converter input.dat custom-output.json
```

## License

MIT
