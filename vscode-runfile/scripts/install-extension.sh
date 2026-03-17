#!/bin/sh

editor="${npm_config_editor:-$(command -v cursor >/dev/null 2>&1 && echo cursor || echo code)}"

if [ "$editor" != "cursor" ] && [ "$editor" != "code" ]; then
  echo "Unsupported editor CLI: $editor"
  exit 1
fi

if ! command -v "$editor" >/dev/null 2>&1; then
  echo "Editor CLI not found: $editor"
  exit 1
fi

VERSION=$(npm exec pkgv)
npm run build && "$editor" --install-extension ./vscode-runfile-${VERSION}.vsix
