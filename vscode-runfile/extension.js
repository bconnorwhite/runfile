const vscode = require('vscode');

/**
 * Folding for Runfile:
 * - Groups: multi-line headers (# -+, # Name, # -+) fold entire section
 * - Commands: a command line (non-indented, not comment) folds its indented body
 */
function activate(context) {
  const provider = {
    provideFoldingRanges(document) {
      const ranges = [];
      const lineCount = document.lineCount;

      const isSep = (text) => /^\s*#\s*-+\s*$/.test(text);
      const isName = (text) => /^\s*#\s+.+$/.test(text) && !isSep(text);
      const isSingleLineHeader = (text) => /^\s*#\s*-+\s*[^-].*?\s*-+\s*$/.test(text);
      const isCommand = (text) => {
        const trimmed = text.trim();
        if (!trimmed || trimmed.startsWith('#')) return false;
        if (/^echo\b/.test(trimmed) || /^#!/.test(trimmed)) return false;
        if (/^\s/.test(text)) return false; // must be non-indented
        return true;
      };
      const isIndented = (text) => /^\s{2}.+/.test(text);

      // Find group ranges (multi-line and single-line headers)
      let i = 0;
      while (i < lineCount) {
        const lineText = document.lineAt(i).text;

        // Multi-line header
        if (isSep(lineText) && i + 2 < lineCount) {
          const nameLine = document.lineAt(i + 1).text;
          const endLine = document.lineAt(i + 2).text;
          if (isName(nameLine) && isSep(endLine)) {
            // Group starts at i, content starts at i+3
            const start = i;
            let j = i + 3;
            while (j < lineCount) {
              const t = document.lineAt(j).text;
              if (isSep(t) && j + 2 < lineCount) {
                const nl = document.lineAt(j + 1).text;
                const el = document.lineAt(j + 2).text;
                if (isName(nl) && isSep(el)) {
                  break;
                }
              }
              j++;
            }
            const end = Math.max(start, j - 1);
            ranges.push(new vscode.FoldingRange(start, end, vscode.FoldingRangeKind.Region));
            i = i + 3;
            continue;
          }
        }

        // Single-line header
        if (isSingleLineHeader(lineText)) {
          const start = i;
          let j = i + 1;
          while (j < lineCount) {
            const t = document.lineAt(j).text;
            if (isSingleLineHeader(t)) break;
            if (isSep(t) && j + 2 < lineCount) {
              const nl = document.lineAt(j + 1).text;
              const el = document.lineAt(j + 2).text;
              if (isName(nl) && isSep(el)) break;
            }
            j++;
          }
          const end = Math.max(start, j - 1);
          ranges.push(new vscode.FoldingRange(start, end, vscode.FoldingRangeKind.Region));
          i++;
          continue;
        }

        i++;
      }

      // Fold command bodies
      for (let k = 0; k < lineCount; k++) {
        const text = document.lineAt(k).text;
        if (!isCommand(text)) continue;
        let end = k;
        let m = k + 1;
        while (m < lineCount) {
          const t = document.lineAt(m).text;
          if (t.trim() === '') { m++; continue; }
          if (isIndented(t) || t.trim().startsWith('#')) {
            end = m;
            m++;
            continue;
          }
          break;
        }
        if (end > k) {
          ranges.push(new vscode.FoldingRange(k, end, vscode.FoldingRangeKind.Region));
        }
      }

      return ranges;
    }
  };

  context.subscriptions.push(
    vscode.languages.registerFoldingRangeProvider({ language: 'runfile' }, provider)
  );
}

function deactivate() {}

module.exports = { activate, deactivate };


