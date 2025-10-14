use anyhow::Result;

// Type aliases for complex return types
type InlineArg = (String, bool, bool);

// (name, optional, is_varargs)
type InlineFlag = (String, Option<char>, bool, Option<String>);

// (name, short, takes_value, type_hint)
type ArgsAndFlagsResult = (Vec<InlineArg>, Vec<InlineFlag>);

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
  GroupHeader {
    name: String,
  },
  CommandName {
    name: Vec<String>,
    inline_args: Vec<InlineArg>,
    inline_flags: Vec<InlineFlag>,
    comment: Option<String>,
  },
  Argument {
    name: String,
    optional: bool,
    is_varargs: bool,
    comment: Option<String>,
  },
  Flag {
    long_name: String,
    short: Option<char>,
    takes_value: bool,
    type_hint: Option<String>,
    comment: Option<String>,
  },
  ScriptLine {
    content: String,
  },
  Comment {
    content: String,
  },
}

#[derive(Default)]
pub struct TokenizePhase;

impl TokenizePhase {
  pub fn new() -> Self {
    Self
  }
  /// Check if a line is a group header separator (starts with # followed by one or more dashes)
  fn is_separator_line(&self, line: &str) -> bool {
    let trimmed = line.trim();
    trimmed.starts_with("# ") && trimmed.len() > 2 && trimmed[2..].chars().all(|c| c == '-')
  }
  /// Check if a line is a command line (either ends with colon or is a simple command name)
  fn is_command_line(&self, line: &str) -> bool {
    let trimmed = line.trim();
    // Must not be a comment, echo, or shebang
    if trimmed.starts_with('#') || trimmed.starts_with("echo") || trimmed.starts_with("#!/") {
      return false;
    }
    // Must not be indented (arguments and flags are indented)
    if line.starts_with("  ") {
      return false;
    }
    // Check if line ends with colon
    let has_colon = trimmed.ends_with(':');
    let command_line = if has_colon {
      trimmed.strip_suffix(':').unwrap().trim()
    } else {
      trimmed
    };
    // Special case: if the line is just ":", it should be treated as a command line for error handling
    if command_line.is_empty() {
      return true;
    }
    let parts: Vec<&str> = command_line.split_whitespace().collect();
    if parts.is_empty() {
      return false;
    }
    // Find where aliases end and flags/args begin
    let mut i = 0;
    let mut prev_had_comma = false;
    // Parse aliases first
    while i < parts.len() {
      let part = parts[i];
      // Stop at flags or special args
      if part.starts_with('-') || part.contains('?') || part.contains("...") || part.contains('=') {
        break;
      }
      // If this part contains or ends with a comma, it's part of aliases
      if part.contains(',') {
        prev_had_comma = true;
      } else if prev_had_comma {
        // Previous part had a comma, so this is still an alias
        prev_had_comma = false;
      } else if i == 0 {
        // First part without comma - single alias
      } else {
        // No comma, not first part, not after comma - this is an argument
        break;
      }
      i += 1;
    }
    // If we have aliases, it's a command line
    // If it has a colon, the colon must come after all args/flags
    if i > 0 {
      if has_colon {
        // Colon must come after all args and flags
        // If there are args/flags after aliases, they should all be before the colon
        return true; // The colon is at the end, so all args/flags are before it
      } else {
        // No colon - this is a simple command name
        return true;
      }
    }
    false
  }
  pub fn tokenize(&self, content: &str) -> Result<Vec<Token>> {
    let mut tokens = Vec::new();
    let lines: Vec<&str> = content.lines().collect();
    let mut i = 0;
    while i < lines.len() {
      let line = lines[i];
      let trimmed = line.trim();
      // Check for multi-line group header: # -+ \n # Group Name \n # -+
      if self.is_separator_line(trimmed) && i + 2 < lines.len() {
        let next_line = lines[i + 1].trim();
        let third_line = lines[i + 2].trim();
        if next_line.starts_with("# ") && self.is_separator_line(third_line) {
          let group_name = next_line
            .strip_prefix("# ")
            .unwrap_or("")
            .trim()
            .to_string();
          tokens.push(Token::GroupHeader { name: group_name });
          i += 3; // Skip the next two lines
          continue;
        }
      }
      // Check if this is a command line and look for comments above it
      // New syntax: colon must come after all flags and args, not directly after command
      if self.is_command_line(line) {
        // Look for comments on the line above
        let mut comment_lines = Vec::new();
        let mut j = i;
        while j > 0 {
          j -= 1;
          let prev_line = lines[j].trim();
          if prev_line.starts_with('#') {
            // Skip group header separators and group names
            if !self.is_separator_line(prev_line) {
              // Check if this is a group name by looking at the surrounding context
              // A line is a group name if it's preceded by a separator line AND followed by a separator line
              let is_group_name = if j > 0 && j + 1 < lines.len() {
                let prev_prev_line = lines[j - 1].trim();
                let next_line = lines[j + 1].trim();
                self.is_separator_line(prev_prev_line) && self.is_separator_line(next_line)
              } else {
                false
              };
              if !is_group_name {
                comment_lines.insert(
                  0,
                  prev_line.strip_prefix('#').unwrap_or("").trim().to_string(),
                );
              }
            }
          } else {
            // Break on empty or non-comment lines
            break;
          }
        }
        let comment = if comment_lines.is_empty() {
          None
        } else {
          Some(comment_lines.join(" "))
        };
        let token = self.parse_line_with_comment(line, comment)?;
        if let Some(token) = token {
          tokens.push(token);
        }
      } else {
        // Check if this is a comment that might be attached to a command
        // Special case: shebang lines should be processed as script lines
        if trimmed.starts_with('#') && !self.is_separator_line(trimmed) && !trimmed.starts_with("#!/") {
          // Check if there's a command coming up (after any number of consecutive comments)
          let mut j = i + 1;
          let mut found_command = false;
          while j < lines.len() {
            let next_line = lines[j].trim();
            if next_line.is_empty() {
              j += 1;
            } else if next_line.starts_with('#') && !self.is_separator_line(next_line) {
              // Another comment, keep looking
              j += 1;
            } else if self.is_command_line(next_line) {
              // Found a command, skip this comment
              found_command = true;
              break;
            } else {
              // Not a command, process this comment normally
              break;
            }
          }
          if found_command {
            // Skip this comment line and continue to the next iteration
            i += 1;
            continue;
          }
        }
        // Process normally
        let token = self.parse_line(line)?;
        if let Some(token) = token {
          tokens.push(token);
        }
      }
      i += 1;
    }
    Ok(tokens)
  }
  fn parse_line_with_comment(&self, line: &str, comment: Option<String>) -> Result<Option<Token>> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
      return Ok(None);
    }
    // Command definition: command_name: (but not script lines)
    if self.is_command_line(line) {
      // Check for comments on the same line as command - these should not be allowed
      if trimmed.contains(" # ") {
        // This is an error - comments should be on the line above, not same line
        return Err(anyhow::anyhow!(
          "Command comments must be on the line above the command, not on the same line"
        ));
      }
      // Parse command with potential inline args and flags
      let command_line = if trimmed.ends_with(':') {
        trimmed.strip_suffix(':').unwrap().trim()
      } else {
        trimmed
      };
      // Parse the command line: name[, alias]* [arg|flag]*
      let (aliases, args_and_flags) = self.parse_command_line(command_line)?;
      let (inline_args, inline_flags) = if args_and_flags.is_empty() {
        (Vec::new(), Vec::new())
      } else {
        self.parse_args_and_flags(args_and_flags)?
      };
      return Ok(Some(Token::CommandName {
        name: aliases,
        inline_args,
        inline_flags,
        comment,
      }));
    }
    // Rest of the parsing logic for non-command lines
    self.parse_line(line)
  }
  fn parse_command_line(&self, line: &str) -> Result<(Vec<String>, Vec<String>)> {
    let parts: Vec<&str> = line.split_whitespace().collect();
    let mut aliases = Vec::new();
    let mut args_and_flags = Vec::new();
    let mut i = 0;
    let mut prev_had_comma = false;
    // Parse aliases first
    while i < parts.len() {
      let part = parts[i];
      // Stop at flags or special args
      if part.starts_with('-') || part.contains('?') || part.contains("...") || part.contains('=') {
        break;
      }
      // If this part contains or ends with a comma, it's part of aliases
      if part.contains(',') {
        let alias_parts: Vec<&str> = part.split(',').map(|s| s.trim()).collect();
        for alias in alias_parts {
          if !alias.is_empty() {
            aliases.push(alias.to_string());
          }
        }
        prev_had_comma = true;
      } else if prev_had_comma {
        // Previous part had a comma, so this is still an alias
        aliases.push(part.to_string());
        prev_had_comma = false;
      } else if aliases.is_empty() {
        // First part without comma - single alias
        aliases.push(part.to_string());
      } else {
        // No comma, not first part, not after comma - this is an argument
        break;
      }
      i += 1;
    }
    // Parse remaining args and flags
    while i < parts.len() {
      args_and_flags.push(parts[i].to_string());
      i += 1;
    }
    if aliases.is_empty() {
      return Err(anyhow::anyhow!("Command must have at least one name"));
    }
    Ok((aliases, args_and_flags))
  }
  fn parse_args_and_flags(&self, parts: Vec<String>) -> Result<ArgsAndFlagsResult> {
    let mut args = Vec::new();
    let mut flags = Vec::new();
    let mut i = 0;
    while i < parts.len() {
      let part = &parts[i];
      if part.starts_with("...") || part.ends_with("...") {
        // Varargs (support both prefix ...args and suffix args...)
        let arg_name = if part.starts_with("...") {
          part.strip_prefix("...").unwrap_or("args").to_string()
        } else {
          part.strip_suffix("...").unwrap_or("args").to_string()
        };
        args.push((arg_name, true, true));
        i += 1;
      } else if part.starts_with('-') {
        // This is a flag
        if part.ends_with(',') && i + 1 < parts.len() {
          // Comma-separated flag: -f, --flag
          let short_part = part.strip_suffix(',').unwrap();
          let long_part = &parts[i + 1];
          let short = short_part.strip_prefix('-').and_then(|s| s.chars().next());
          let (long_name, takes_value, type_hint) = self.parse_flag_name(long_part)?;
          flags.push((long_name, short, takes_value, type_hint));
          i += 2; // Skip the next part since we processed it
        } else if part.starts_with("--") {
          // Long flag only: --flag or --flag=<type>
          let (long_name, takes_value, type_hint) = self.parse_flag_name(part)?;
          flags.push((long_name, None, takes_value, type_hint));
          i += 1;
        } else if part.len() == 2 && part.starts_with('-') {
          // Short flag only: -f
          let short = part.chars().nth(1).unwrap();
          flags.push((format!("{}", short), Some(short), false, None));
          i += 1;
        } else {
          i += 1;
        }
      } else {
        // This is an argument
        let (arg_name, optional) = if part.ends_with('?') {
          (part.strip_suffix('?').unwrap().to_string(), true)
        } else {
          (part.to_string(), false)
        };
        args.push((arg_name, optional, false));
        i += 1;
      }
    }
    Ok((args, flags))
  }
  fn parse_flag_name(&self, flag: &str) -> Result<(String, bool, Option<String>)> {
    let flag = flag.strip_prefix("--").unwrap_or(flag);
    if flag.contains('=') {
      // Value flag: --output=<file>
      let parts: Vec<&str> = flag.split('=').collect();
      if parts.len() == 2 {
        let name = parts[0].to_string();
        let type_hint = parts[1]
          .strip_prefix('<')
          .and_then(|s| s.strip_suffix('>'))
          .map(|s| s.to_string());
        return Ok((name, true, type_hint));
      }
    }
    // Boolean flag: --flag
    Ok((flag.to_string(), false, None))
  }
  fn parse_line(&self, line: &str) -> Result<Option<Token>> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
      return Ok(None);
    }
    // Command definition: command_name: (but not script lines)
    if self.is_command_line(line) {
      // Check for comments on the same line as command - these should not be allowed
      if trimmed.contains(" # ") {
        // This is an error - comments should be on the line above, not same line
        return Err(anyhow::anyhow!(
          "Command comments must be on the line above the command, not on the same line"
        ));
      }
      // Parse command with potential inline args and flags
      let command_line = if trimmed.ends_with(':') {
        trimmed.strip_suffix(':').unwrap().trim()
      } else {
        trimmed
      };
      // Parse the command line: name[, alias]* [arg|flag]*
      let (aliases, args_and_flags) = self.parse_command_line(command_line)?;
      let (inline_args, inline_flags) = if args_and_flags.is_empty() {
        (Vec::new(), Vec::new())
      } else {
        self.parse_args_and_flags(args_and_flags)?
      };
      return Ok(Some(Token::CommandName {
        name: aliases,
        inline_args,
        inline_flags,
        comment: None,
      }));
    }
    // Indented argument or flag: must be exactly 2 spaces, no more
    // Allow shebang lines even when indented
    if line.starts_with("  ") && !line.starts_with("   ") && (!trimmed.starts_with('#') || trimmed.starts_with("#!/")) {
      let content = trimmed;
      // Extract comment if present
      let (content_part, comment) = if let Some(comment_start) = content.find(" # ") {
        let (before, after) = content.split_at(comment_start);
        let comment_text = after
          .trim()
          .strip_prefix('#')
          .unwrap_or(after.trim())
          .trim();
        (before.trim(), Some(comment_text.to_string()))
      } else {
        (content, None)
      };
      // Check if it's a shebang line
      if content_part.starts_with("#!/") {
        return Ok(Some(Token::ScriptLine {
          content: line.to_string(),
        }));
      }
      // Check if it's a flag: -s, --long or --long
      if content_part.starts_with('-') {
        let parts: Vec<&str> = content_part.split(',').map(|s| s.trim()).collect();
        if parts.len() == 2 {
          // -s, --long format
          let short = parts[0].strip_prefix('-').and_then(|s| s.chars().next());
          let long_part = parts[1];
          // Strip trailing colon if present
          let clean_long_part = long_part.strip_suffix(':').unwrap_or(long_part);
          let (long_name, takes_value, type_hint) = self.parse_flag_name(clean_long_part)?;
          return Ok(Some(Token::Flag {
            long_name,
            short,
            takes_value,
            type_hint,
            comment,
          }));
        } else if content_part.starts_with("--") {
          // --long format
          // Strip trailing colon if present
          let clean_content = content_part.strip_suffix(':').unwrap_or(content_part);
          let (long_name, takes_value, type_hint) = self.parse_flag_name(clean_content)?;
          return Ok(Some(Token::Flag {
            long_name,
            short: None,
            takes_value,
            type_hint,
            comment,
          }));
        }
      } else {
        // Check if it's an argument (no dashes, simple identifier)
        // Arguments must be a single word (no spaces after removing comment)
        if !content_part.contains(' ') && !content_part.is_empty() && !content_part.starts_with('-') {
          // Strip trailing colon if present
          let clean_content = content_part.strip_suffix(':').unwrap_or(content_part);
          let (arg_name, optional, is_varargs) = if clean_content.starts_with("...") {
            (
              clean_content
                .strip_prefix("...")
                .unwrap_or("args")
                .to_string(),
              true,
              true,
            )
          } else if clean_content.ends_with("...") {
            (
              clean_content
                .strip_suffix("...")
                .unwrap_or("args")
                .to_string(),
              true,
              true,
            )
          } else if clean_content.ends_with('?') {
            (
              clean_content.strip_suffix('?').unwrap().to_string(),
              true,
              false,
            )
          } else {
            (clean_content.to_string(), false, false)
          };
          return Ok(Some(Token::Argument {
            name: arg_name,
            optional,
            is_varargs,
            comment,
          }));
        }
      }
    }
    // Comment or script line
    if trimmed.starts_with('#') {
      // Check if it's a shebang (starts with #!)
      if trimmed.starts_with("#!/") {
        Ok(Some(Token::ScriptLine {
          content: line.to_string(),
        }))
      } else {
        Ok(Some(Token::Comment {
          content: trimmed.to_string(),
        }))
      }
    } else {
      Ok(Some(Token::ScriptLine {
        content: line.to_string(),
      }))
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  // Group Header Tests

  #[test]
  fn test_group_header_multi_line_various_dashes() {
    let tokenizer = TokenizePhase::new();

    let test_cases = vec![
      ("# -\n# Group 1\n# -", "Group 1"),
      ("# --\n# Group 2\n# --", "Group 2"),
      ("# -----\n# Group 3\n# -----", "Group 3"),
      ("# ----------\n# Group 4\n# ----------", "Group 4"),
    ];

    for (input, expected) in test_cases {
      let tokens = tokenizer.tokenize(input).unwrap();
      assert_eq!(
        tokens[0],
        Token::GroupHeader {
          name: expected.to_string()
        }
      );
    }
  }

  // Command Tests
  #[test]
  fn test_simple_command() {
    let tokenizer = TokenizePhase::new();
    let content = "hello:";
    let tokens = tokenizer.tokenize(content).unwrap();
    assert_eq!(
      tokens[0],
      Token::CommandName {
        name: vec!["hello".to_string()],
        inline_args: vec![],
        inline_flags: vec![],
        comment: None
      }
    );
  }

  #[test]
  fn test_command_with_comment() {
    let tokenizer = TokenizePhase::new();
    let content = "# This is a comment\nhello:";
    let tokens = tokenizer.tokenize(content).unwrap();
    assert_eq!(
      tokens[0],
      Token::CommandName {
        name: vec!["hello".to_string()],
        inline_args: vec![],
        inline_flags: vec![],
        comment: Some("This is a comment".to_string())
      }
    );
  }

  #[test]
  fn test_command_multiple_comments() {
    let tokenizer = TokenizePhase::new();
    let content = "# First comment\n# Second comment\nhello:";
    let tokens = tokenizer.tokenize(content).unwrap();
    assert_eq!(
      tokens[0],
      Token::CommandName {
        name: vec!["hello".to_string()],
        inline_args: vec![],
        inline_flags: vec![],
        comment: Some("First comment Second comment".to_string())
      }
    );
  }

  #[test]
  fn test_command_comment_skips_group_headers() {
    let tokenizer = TokenizePhase::new();
    let content = "# -----\n# Group Name\n# -----\n# This is a comment\nhello:";
    let tokens = tokenizer.tokenize(content).unwrap();
    assert_eq!(
      tokens[0],
      Token::GroupHeader {
        name: "Group Name".to_string()
      }
    );
    assert_eq!(
      tokens[1],
      Token::CommandName {
        name: vec!["hello".to_string()],
        inline_args: vec![],
        inline_flags: vec![],
        comment: Some("This is a comment".to_string())
      }
    );
  }

  // Alias Tests
  #[test]
  fn test_single_alias() {
    let tokenizer = TokenizePhase::new();
    let content = "build:";
    let tokens = tokenizer.tokenize(content).unwrap();
    assert_eq!(
      tokens[0],
      Token::CommandName {
        name: vec!["build".to_string()],
        inline_args: vec![],
        inline_flags: vec![],
        comment: None
      }
    );
  }

  #[test]
  fn test_multiple_aliases_comma_separated() {
    let tokenizer = TokenizePhase::new();
    let content = "b, build:";
    let tokens = tokenizer.tokenize(content).unwrap();
    assert_eq!(
      tokens[0],
      Token::CommandName {
        name: vec!["b".to_string(), "build".to_string()],
        inline_args: vec![],
        inline_flags: vec![],
        comment: None
      }
    );
  }

  #[test]
  fn test_multiple_aliases_multiple_parts() {
    let tokenizer = TokenizePhase::new();
    let content = "b, build, compile:";
    let tokens = tokenizer.tokenize(content).unwrap();
    assert_eq!(
      tokens[0],
      Token::CommandName {
        name: vec!["b".to_string(), "build".to_string(), "compile".to_string()],
        inline_args: vec![],
        inline_flags: vec![],
        comment: None
      }
    );
  }

  #[test]
  fn test_aliases_with_inline_args() {
    let tokenizer = TokenizePhase::new();
    let content = "b, build target:";
    let tokens = tokenizer.tokenize(content).unwrap();
    assert_eq!(
      tokens[0],
      Token::CommandName {
        name: vec!["b".to_string(), "build".to_string()],
        inline_args: vec![("target".to_string(), false, false)],
        inline_flags: vec![],
        comment: None
      }
    );
  }

  #[test]
  fn test_aliases_with_inline_flags() {
    let tokenizer = TokenizePhase::new();
    let content = "r, run --debug:";
    let tokens = tokenizer.tokenize(content).unwrap();
    assert_eq!(
      tokens[0],
      Token::CommandName {
        name: vec!["r".to_string(), "run".to_string()],
        inline_args: vec![],
        inline_flags: vec![("debug".to_string(), None, false, None)],
        comment: None
      }
    );
  }

  // Argument Tests
  #[test]
  fn test_required_argument() {
    let tokenizer = TokenizePhase::new();
    let content = "command:\n  arg";
    let tokens = tokenizer.tokenize(content).unwrap();
    assert_eq!(
      tokens[1],
      Token::Argument {
        name: "arg".to_string(),
        optional: false,
        is_varargs: false,
        comment: None
      }
    );
  }

  #[test]
  fn test_optional_argument() {
    let tokenizer = TokenizePhase::new();
    let content = "command:\n  arg?";
    let tokens = tokenizer.tokenize(content).unwrap();
    assert_eq!(
      tokens[1],
      Token::Argument {
        name: "arg".to_string(),
        optional: true,
        is_varargs: false,
        comment: None
      }
    );
  }

  #[test]
  fn test_varargs() {
    let tokenizer = TokenizePhase::new();
    let content = "command:\n  ...args";
    let tokens = tokenizer.tokenize(content).unwrap();
    assert_eq!(
      tokens[1],
      Token::Argument {
        name: "args".to_string(),
        optional: true,
        is_varargs: true,
        comment: None
      }
    );
  }

  #[test]
  fn test_argument_with_comment() {
    let tokenizer = TokenizePhase::new();
    let content = "command:\n  arg # This is an argument";
    let tokens = tokenizer.tokenize(content).unwrap();
    assert_eq!(
      tokens[1],
      Token::Argument {
        name: "arg".to_string(),
        optional: false,
        is_varargs: false,
        comment: Some("This is an argument".to_string())
      }
    );
  }

  // Flag Tests
  #[test]
  fn test_long_flag() {
    let tokenizer = TokenizePhase::new();
    let content = "command:\n  --flag";
    let tokens = tokenizer.tokenize(content).unwrap();
    assert_eq!(
      tokens[1],
      Token::Flag {
        long_name: "flag".to_string(),
        short: None,
        takes_value: false,
        type_hint: None,
        comment: None
      }
    );
  }

  #[test]
  fn test_short_and_long_flag() {
    let tokenizer = TokenizePhase::new();
    let content = "command:\n  -r, --release";
    let tokens = tokenizer.tokenize(content).unwrap();
    assert_eq!(
      tokens[1],
      Token::Flag {
        long_name: "release".to_string(),
        short: Some('r'),
        takes_value: false,
        type_hint: None,
        comment: None
      }
    );
  }

  #[test]
  fn test_value_flag() {
    let tokenizer = TokenizePhase::new();
    let content = "command:\n  --output=<file>";
    let tokens = tokenizer.tokenize(content).unwrap();
    assert_eq!(
      tokens[1],
      Token::Flag {
        long_name: "output".to_string(),
        short: None,
        takes_value: true,
        type_hint: Some("file".to_string()),
        comment: None
      }
    );
  }

  #[test]
  fn test_flag_with_comment() {
    let tokenizer = TokenizePhase::new();
    let content = "command:\n  --debug # Enable debug mode";
    let tokens = tokenizer.tokenize(content).unwrap();
    assert_eq!(
      tokens[1],
      Token::Flag {
        long_name: "debug".to_string(),
        short: None,
        takes_value: false,
        type_hint: None,
        comment: Some("Enable debug mode".to_string())
      }
    );
  }

  // Inline Args and Flags Tests
  #[test]
  fn test_inline_args() {
    let tokenizer = TokenizePhase::new();
    let content = "command arg1 arg2?:";
    let tokens = tokenizer.tokenize(content).unwrap();
    assert_eq!(
      tokens[0],
      Token::CommandName {
        name: vec!["command".to_string()],
        inline_args: vec![
          ("arg1".to_string(), false, false),
          ("arg2".to_string(), true, false)
        ],
        inline_flags: vec![],
        comment: None
      }
    );
  }

  #[test]
  fn test_inline_flags() {
    let tokenizer = TokenizePhase::new();
    let content = "command -d, --debug --output=<file>:";
    let tokens = tokenizer.tokenize(content).unwrap();
    assert_eq!(
      tokens[0],
      Token::CommandName {
        name: vec!["command".to_string()],
        inline_args: vec![],
        inline_flags: vec![
          ("debug".to_string(), Some('d'), false, None),
          ("output".to_string(), None, true, Some("file".to_string()))
        ],
        comment: None
      }
    );
  }

  #[test]
  fn test_inline_varargs() {
    let tokenizer = TokenizePhase::new();
    let content = "command ...args:";
    let tokens = tokenizer.tokenize(content).unwrap();
    assert_eq!(
      tokens[0],
      Token::CommandName {
        name: vec!["command".to_string()],
        inline_args: vec![("args".to_string(), true, true)],
        inline_flags: vec![],
        comment: None
      }
    );
  }

  // Script Line Tests
  #[test]
  fn test_script_lines() {
    let tokenizer = TokenizePhase::new();
    let content = "  echo \"Hello world\"\n  echo \"Another line\"";
    let tokens = tokenizer.tokenize(content).unwrap();
    assert_eq!(
      tokens[0],
      Token::ScriptLine {
        content: "  echo \"Hello world\"".to_string()
      }
    );
    assert_eq!(
      tokens[1],
      Token::ScriptLine {
        content: "  echo \"Another line\"".to_string()
      }
    );
  }

  #[test]
  fn test_script_lines_with_shebang() {
    let tokenizer = TokenizePhase::new();
    let content = "#!/bin/bash\necho \"Hello\"";
    let tokens = tokenizer.tokenize(content).unwrap();
    assert_eq!(
      tokens[0],
      Token::ScriptLine {
        content: "#!/bin/bash".to_string()
      }
    );
    assert_eq!(
      tokens[1],
      Token::ScriptLine {
        content: "echo \"Hello\"".to_string()
      }
    );
  }

  // Comment Tests
  #[test]
  fn test_standalone_comments() {
    let tokenizer = TokenizePhase::new();
    let content = "# This is a comment\n# Another comment";
    let tokens = tokenizer.tokenize(content).unwrap();
    assert_eq!(
      tokens[0],
      Token::Comment {
        content: "# This is a comment".to_string()
      }
    );
    assert_eq!(
      tokens[1],
      Token::Comment {
        content: "# Another comment".to_string()
      }
    );
  }

  // Error Tests
  #[test]
  fn test_command_with_inline_comment_error() {
    let tokenizer = TokenizePhase::new();
    let content = "command # This should be an error:";
    let result = tokenizer.tokenize(content);
    assert!(result.is_err());
    assert!(
      result
        .unwrap_err()
        .to_string()
        .contains("Command comments must be on the line above")
    );
  }

  #[test]
  fn test_empty_command_name_error() {
    let tokenizer = TokenizePhase::new();
    let content = ":";
    let result = tokenizer.tokenize(content);
    assert!(result.is_err());
    assert!(
      result
        .unwrap_err()
        .to_string()
        .contains("Command must have at least one name")
    );
  }

  // Edge Cases
  #[test]
  fn test_unicode_command() {
    let tokenizer = TokenizePhase::new();
    let content = "测试:";
    let tokens = tokenizer.tokenize(content).unwrap();
    assert_eq!(
      tokens[0],
      Token::CommandName {
        name: vec!["测试".to_string()],
        inline_args: vec![],
        inline_flags: vec![],
        comment: None
      }
    );
  }

  #[test]
  fn test_unicode_arguments() {
    let tokenizer = TokenizePhase::new();
    let content = "command:\n  🚀? # Rocket argument";
    let tokens = tokenizer.tokenize(content).unwrap();
    assert_eq!(
      tokens[1],
      Token::Argument {
        name: "🚀".to_string(),
        optional: true,
        is_varargs: false,
        comment: Some("Rocket argument".to_string())
      }
    );
  }

  #[test]
  fn test_special_characters_in_names() {
    let tokenizer = TokenizePhase::new();
    let content = "test-special_chars:";
    let tokens = tokenizer.tokenize(content).unwrap();
    assert_eq!(
      tokens[0],
      Token::CommandName {
        name: vec!["test-special_chars".to_string()],
        inline_args: vec![],
        inline_flags: vec![],
        comment: None
      }
    );
  }

  #[test]
  fn test_empty_lines() {
    let tokenizer = TokenizePhase::new();
    let content = "\n\ncommand:\n\n  arg\n\n";
    let tokens = tokenizer.tokenize(content).unwrap();
    assert_eq!(
      tokens[0],
      Token::CommandName {
        name: vec!["command".to_string()],
        inline_args: vec![],
        inline_flags: vec![],
        comment: None
      }
    );
    assert_eq!(
      tokens[1],
      Token::Argument {
        name: "arg".to_string(),
        optional: false,
        is_varargs: false,
        comment: None
      }
    );
  }

  // Complex Integration Tests
  #[test]
  fn test_complete_runfile() {
    let tokenizer = TokenizePhase::new();
    let content = "# ----------\n# Basic Commands\n# ----------\n\n# Simple command\nhello:\n  echo \"Hello, World!\"\n\n# Command with args\nbuild:\n  --debug     # Enable debug mode\n  --release   # Build in release mode\n  echo \"Building\"";
    let tokens = tokenizer.tokenize(content).unwrap();

    assert_eq!(
      tokens[0],
      Token::GroupHeader {
        name: "Basic Commands".to_string()
      }
    );
    assert_eq!(
      tokens[1],
      Token::CommandName {
        name: vec!["hello".to_string()],
        inline_args: vec![],
        inline_flags: vec![],
        comment: Some("Simple command".to_string())
      }
    );
    assert_eq!(
      tokens[2],
      Token::ScriptLine {
        content: "  echo \"Hello, World!\"".to_string()
      }
    );
    assert_eq!(
      tokens[3],
      Token::CommandName {
        name: vec!["build".to_string()],
        inline_args: vec![],
        inline_flags: vec![],
        comment: Some("Command with args".to_string())
      }
    );
    assert_eq!(
      tokens[4],
      Token::Flag {
        long_name: "debug".to_string(),
        short: None,
        takes_value: false,
        type_hint: None,
        comment: Some("Enable debug mode".to_string())
      }
    );
    assert_eq!(
      tokens[5],
      Token::Flag {
        long_name: "release".to_string(),
        short: None,
        takes_value: false,
        type_hint: None,
        comment: Some("Build in release mode".to_string())
      }
    );
    assert_eq!(
      tokens[6],
      Token::ScriptLine {
        content: "  echo \"Building\"".to_string()
      }
    );
  }
}
