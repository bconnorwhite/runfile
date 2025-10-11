# Runfile VSCode Extension

Syntax highlighting support for Runfile in Visual Studio Code.

## Features

- Syntax highlighting for Runfile commands, arguments, flags, and shell scripts
- Support for embedded shell scripts
- Comment highlighting
- Group header highlighting

## Installation

1. Copy this extension to your VSCode extensions directory
2. Reload VSCode
3. Open any `Runfile` or `*.runfile` file to see syntax highlighting

## Development

To test the extension:

1. Open this folder in VSCode
2. Press `F5` to launch Extension Development Host
3. Open a Runfile to see syntax highlighting

## Language Support

This extension provides syntax highlighting for:

- **Command definitions** - Lines ending with `:`
- **Arguments** - Indented identifiers with optional `?` suffix
- **Flags** - Short (`-r`) and long (`--release`) flags
- **Group headers** - Comment lines with dashes
- **Comments** - Lines starting with `#`
- **Shell scripts** - Indented script content

## License

MIT
