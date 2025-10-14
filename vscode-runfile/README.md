# Runfile VSCode Extension

Syntax highlighting support for Runfile in Visual Studio Code.

## Features

- Syntax highlighting for Runfile commands, aliases, inline args/flags, and shell scripts
- Support for embedded shell scripts and indented shebang
- Comment highlighting (including inline comments on args/flags)
- Group header highlighting (single-line and 3-line)
- Folding for entire groups and per-command bodies

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

- **Command definitions** - Aliases (comma-separated), optional inline args (`arg?`, `...args`) and flags (`-s, --long`, `--name=<type>`), optional trailing `:`
- **Arguments** - Indented names (unicode allowed), optional `?`, varargs `...name`, with optional inline comment after ` # `
- **Flags** - Short (`-r`), long (`--release`), combined (`-r, --release`), and value flags (`--output=<file>`)
- **Group headers** - Single-line `# --- Name ---` or multi-line 3-line headers
- **Comments** - Lines starting with `#`, preserved in script blocks
- **Shell scripts** - Indented script content, including indented shebang `#!/...`

### Folding
- Groups (both header styles) fold from header through their contents
- Commands fold from the command line through the indented body
