use anyhow::Result;
use super::tokenize::Token;
use std::io::Write;
use ansi_term::Colour;

#[derive(Debug, Clone)]
pub struct Runfile {
  pub groups: Vec<Group>,
  pub commands: Vec<Command>,
}

#[derive(Debug, Clone)]
pub struct Group {
  pub name: String,
}

#[derive(Debug, Clone)]
pub struct Command {
  pub names: Vec<String>,
  pub description: Option<String>,
  pub group: Option<String>,
  pub args: Vec<Argument>,
  pub flags: Vec<Flag>,
  pub script: String,
  pub shebang: String,
}

#[derive(Debug, Clone)]
pub struct Argument {
  pub name: String,
  pub optional: bool,
  pub is_varargs: bool,
  pub description: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Flag {
  pub short: Option<char>,
  pub long: String,
  pub takes_value: bool,
  pub type_hint: Option<String>,
  pub description: Option<String>,
}

pub struct ParsePhase;

impl ParsePhase {
  pub fn new() -> Self {
    Self
  }

  pub fn parse(&self, tokens: Vec<Token>) -> Result<Runfile> {
    let mut groups = Vec::new();
    let mut commands = Vec::new();
    let mut current_group: Option<String> = None;
    let mut current_command: Option<Command> = None;
    let mut in_script = false;

    for token in tokens {
      match token {
        Token::GroupHeader { name } => {
          // Save any current command
          if let Some(cmd) = current_command.take() {
            commands.push(cmd);
          }

          current_group = Some(name.clone());
          groups.push(Group {
            name,
          });
          in_script = false;
        }
        Token::CommandName { name, inline_args, inline_flags, comment } => {
          // Save any current command
          if let Some(cmd) = current_command.take() {
            commands.push(cmd);
          }

          // Convert inline args and flags to proper structures
          let args: Vec<Argument> = inline_args.into_iter()
            .map(|(name, optional, is_varargs)| Argument {
              name,
              optional,
              is_varargs,
              description: None,
            })
            .collect();

          let flags: Vec<Flag> = inline_flags.into_iter()
            .map(|(long, short, takes_value, type_hint)| Flag {
              short,
              long,
              takes_value,
              type_hint,
              description: None,
            })
            .collect();

          current_command = Some(Command {
            names: name.clone(),
            description: comment,
            group: current_group.clone(),
            args,
            flags,
            script: String::new(),
            shebang: "#!/bin/sh".to_string(),
          });
          in_script = false;
        }
        Token::Argument { name, optional, is_varargs, comment } => {
          if let Some(ref mut cmd) = current_command {
            if !in_script {
              cmd.args.push(Argument {
                name,
                optional,
                is_varargs,
                description: comment,
              });
            } else {
              // This is part of the script, not an argument definition
              if !cmd.script.is_empty() {
                cmd.script.push('\n');
              }
              cmd.script.push_str(&format!("{}", name));
            }
          }
        }
        Token::Flag { long_name, short, takes_value, type_hint, comment } => {
          if let Some(ref mut cmd) = current_command {
            if !in_script {
              cmd.flags.push(Flag {
                short,
                long: long_name,
                takes_value,
                type_hint,
                description: comment,
              });
            } else {
              // This is part of the script, not a flag definition
              if !cmd.script.is_empty() {
                cmd.script.push('\n');
              }
              cmd.script.push_str(&format!("-{}, --{}", short.unwrap_or(' '), long_name));
            }
          }
        }
        Token::ScriptLine { content: line } => {
          if let Some(ref mut cmd) = current_command {
            if !in_script {
              // Check for shebang on first script line
              if line.trim().starts_with("#!") {
                cmd.shebang = line.trim().to_string();
              }
              in_script = true;
            }

            if !cmd.script.is_empty() {
              cmd.script.push('\n');
            }
            cmd.script.push_str(&line);
          }
        }
        Token::Comment { content } => {
          // Comments in script body are preserved
          if let Some(ref mut cmd) = current_command {
            if in_script {
              if !cmd.script.is_empty() {
                cmd.script.push('\n');
              }
              cmd.script.push_str(&content);
            } else {
              // This is a comment that appears before any script line
              // We should treat it as part of the script
              if !cmd.script.is_empty() {
                cmd.script.push('\n');
              }
              cmd.script.push_str(&content);
              in_script = true; // Mark that we're now in script mode
            }
          }
        }
      }
    }

    // Save the last command
    if let Some(cmd) = current_command {
      commands.push(cmd);
    }

    // Filter out empty groups
    let groups: Vec<Group> = groups.into_iter()
      .filter(|group| !group.name.is_empty())
      .collect();

    Ok(Runfile { groups, commands })
  }
}

impl Runfile {
  /// Generate help output to stdout
  pub fn generate_help_output(&self, colors: bool) {
    self.generate_help_output_to_buffer(colors, &mut std::io::stdout());
  }
  /// Generate help output for this runfile
  pub fn generate_help(&self, colors: bool) -> String {
    let mut output = Vec::new();
    self.generate_help_output_to_buffer(colors, &mut output);
    String::from_utf8(output).unwrap_or_default()
  }
  /// Generate help output to a buffer
  ///
  /// # Formatting Rules
  ///
  /// ## Layout Structure
  /// - Group headers at column 0 (no indent)
  /// - Commands indented 2 spaces
  /// - Arguments/flags indented 4 spaces
  /// - Ungrouped commands appear first (if any), then grouped commands
  ///
  /// ## Alias Display
  /// - Show all names: `alias1, alias2, mainname` in definition order
  /// - The `command.aliases` vec already contains all names including primary
  ///
  /// ## Alignment
  /// - Calculate globally across all commands, args, and flags
  /// - Comments align to a single point based on longest element
  /// - Measured from start of line (including indent)
  /// - Elements without descriptions: no trailing spaces or comment marker
  ///
  /// ## Spacing
  /// - Blank line after each group's commands
  /// - Blank line after ungrouped commands section
  /// - Empty runfile: just a newline
  fn generate_help_output_to_buffer(&self, colors: bool, output: &mut dyn Write) {
    // Handle empty runfiles
    if self.commands.is_empty() {
      writeln!(output).unwrap();
      return;
    }

    // Helper function to format descriptions with or without colors
    let format_description = |description: &str| -> String {
      if description.is_empty() {
        String::new()
      } else if colors {
        Colour::Fixed(8).paint(format!(" # {}", description)).to_string()
      } else {
        format!(" # {}", description)
      }
    };

    // Group commands by their groups
    let mut grouped_commands = std::collections::HashMap::new();
    for command in &self.commands {
      let group_name = command.group.as_deref().unwrap_or("General");
      grouped_commands.entry(group_name).or_insert_with(Vec::new).push(command);
    }

    // Calculate global max widths across all commands
    let mut global_max_command_len = 0;
    let mut global_max_param_len = 0;

    for command in &self.commands {
      let command_display = if !command.names.is_empty() {
        command.names.join(", ")
      } else {
        "".to_string()
      };
      global_max_command_len = global_max_command_len.max(command_display.len());

      for arg in &command.args {
        let arg_display = if arg.is_varargs {
          format!("...{}", arg.name)
        } else if arg.optional {
          format!("{}?", arg.name)
        } else {
          arg.name.clone()
        };
        global_max_param_len = global_max_param_len.max(arg_display.len());
      }
      for flag in &command.flags {
        let flag_display = if let Some(short) = flag.short {
          format!("-{}, --{}", short, flag.long)
        } else {
          format!("--{}", flag.long)
        };
        global_max_param_len = global_max_param_len.max(flag_display.len());
      }
    }

    // Calculate alignment points - comments should align to the widest command or param
    // Commands are indented 2 spaces, params are indented 4 spaces
    // We need to find the widest element INCLUDING indent, then round up
    let max_with_indent = (2 + global_max_command_len).max(4 + global_max_param_len);

    // Round up to nearest multiple of 2
    let align_point = ((max_with_indent + 1) / 2) * 2; // Round up to nearest even

    // Both commands and params use the same alignment point (measured from start of text, not line)
    // This represents the width that text + padding should occupy BEFORE the space that format_description adds
    // So we subtract 1 to account for that space
    let command_align_point = align_point - 1;
    let param_align_point = align_point - 1;

    // Track which groups we've printed
    let mut printed_groups = std::collections::HashSet::new();

    // Print ungrouped commands first (only if they exist)
    if let Some(commands) = grouped_commands.get("General") {
      for command in commands {
        // Build command display with aliases
        let command_display = if !command.names.is_empty() {
          command.names.join(", ")
        } else {
          "".to_string()
        };

        let description = command.description.as_deref().unwrap_or("");

        if description.is_empty() {
          // For commands without descriptions, don't add trailing spaces
          writeln!(output, "{}", command_display).unwrap();
        } else {
          let command_padding = " ".repeat(command_align_point.saturating_sub(command_display.len()));
          let formatted_description = format_description(description);
          writeln!(output, "{}{}{}", command_display, command_padding, formatted_description).unwrap();
        }

        for arg in &command.args {
          let arg_display = if arg.is_varargs {
            format!("...{}", arg.name)
          } else {
            let optional = if arg.optional { "?" } else { "" };
            format!("{}{}", arg.name, optional)
          };
          let description = arg.description.as_deref().unwrap_or("");
          let formatted_description = format_description(description);

          if description.is_empty() {
            // For items without descriptions, don't add trailing spaces
            writeln!(output, "  {}", arg_display).unwrap();
          } else {
            let padding = " ".repeat(param_align_point.saturating_sub(arg_display.len()));
            writeln!(output, "  {}{}{}", arg_display, padding, formatted_description).unwrap();
          }
        }
        for flag in &command.flags {
          let short_part = if let Some(short) = flag.short {
            format!("-{}, ", short)
          } else {
            String::new()
          };
          let flag_display = format!("{}--{}", short_part, flag.long);
          let description = flag.description.as_deref().unwrap_or("");
          let formatted_description = format_description(description);

          if description.is_empty() {
            // For items without descriptions, don't add trailing spaces
            writeln!(output, "  {}", flag_display).unwrap();
          } else {
            let padding = " ".repeat(param_align_point.saturating_sub(flag_display.len()));
            writeln!(output, "  {}{}{}", flag_display, padding, formatted_description).unwrap();
          }
        }
      }
      printed_groups.insert("General".to_string());
    }

    // Print each group
    for group in &self.groups {
      if let Some(commands) = grouped_commands.get(group.name.as_str()) {
        if !colors {
          writeln!(output, "{}", group.name).unwrap();
        } else {
          writeln!(output, "{}", Colour::White.bold().paint(&group.name)).unwrap();
        }
        printed_groups.insert(group.name.clone());

        for command in commands {
          // Build command display with aliases
          let command_display = if !command.names.is_empty() {
            command.names.join(", ")
          } else {
            "".to_string()
          };

          let description = command.description.as_deref().unwrap_or("");

          if description.is_empty() {
            // For commands without descriptions, don't add trailing spaces
            writeln!(output, "  {}", command_display).unwrap();
          } else {
            let command_padding = " ".repeat(command_align_point.saturating_sub(command_display.len()));
            let formatted_description = format_description(description);
            writeln!(output, "  {}{}{}", command_display, command_padding, formatted_description).unwrap();
          }

          for arg in &command.args {
            let arg_display = if arg.is_varargs {
              format!("...{}", arg.name)
            } else {
              let optional = if arg.optional { "?" } else { "" };
              format!("{}{}", arg.name, optional)
            };
            let description = arg.description.as_deref().unwrap_or("");
            let formatted_description = format_description(description);

            if description.is_empty() {
              // For items without descriptions, don't add trailing spaces
              writeln!(output, "    {}", arg_display).unwrap();
            } else {
              let padding = " ".repeat(param_align_point.saturating_sub(arg_display.len()));
              writeln!(output, "    {}{}{}", arg_display, padding, formatted_description).unwrap();
            }
          }
          for flag in &command.flags {
            let short_part = if let Some(short) = flag.short {
              format!("-{}, ", short)
            } else {
              String::new()
            };
            let flag_display = format!("{}--{}", short_part, flag.long);
            let description = flag.description.as_deref().unwrap_or("");
            let formatted_description = format_description(description);

            if description.is_empty() {
              // For items without descriptions, don't add trailing spaces
              writeln!(output, "    {}", flag_display).unwrap();
            } else {
              let padding = " ".repeat(param_align_point.saturating_sub(flag_display.len()));
              writeln!(output, "    {}{}{}", flag_display, padding, formatted_description).unwrap();
            }
          }
        }
        writeln!(output).unwrap();
      }
    }

  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use super::super::tokenize::TokenizePhase;

  #[test]
  fn test_parse_simple_command() {
    let tokenizer = TokenizePhase::new();
    let parser = ParsePhase::new();

    let content = "command:\n  arg?\n  --flag\n  echo \"Hello\"";
    let tokens = tokenizer.tokenize(content).unwrap();
    let runfile = parser.parse(tokens).unwrap();

    assert_eq!(runfile.commands.len(), 1);
    let cmd = &runfile.commands[0];
    assert_eq!(cmd.names, vec!["command"]);
    assert_eq!(cmd.args.len(), 1);
    assert_eq!(cmd.args[0].name, "arg");
    assert_eq!(cmd.args[0].optional, true);
    assert_eq!(cmd.args[0].is_varargs, false);
    assert_eq!(cmd.flags.len(), 1);
    assert_eq!(cmd.flags[0].long, "flag");
    assert_eq!(cmd.flags[0].takes_value, false);
    assert_eq!(cmd.script.trim(), "echo \"Hello\"");
  }

  #[test]
  fn test_parse_command_with_flags() {
    let tokenizer = TokenizePhase::new();
    let parser = ParsePhase::new();

    let content = "build:\n  -r, --release\n  --debug\n  echo \"Building\"";
    let tokens = tokenizer.tokenize(content).unwrap();
    let runfile = parser.parse(tokens).unwrap();

    assert_eq!(runfile.commands.len(), 1);
    let cmd = &runfile.commands[0];
    assert_eq!(cmd.names, vec!["build"]);
    assert_eq!(cmd.flags.len(), 2);
    assert_eq!(cmd.flags[0].short, Some('r'));
    assert_eq!(cmd.flags[0].long, "release");
    assert_eq!(cmd.flags[0].takes_value, false);
    assert_eq!(cmd.flags[1].long, "debug");
    assert_eq!(cmd.flags[1].takes_value, false);
  }

  #[test]
  fn test_parse_command_with_inline_args_flags() {
    let tokenizer = TokenizePhase::new();
    let parser = ParsePhase::new();

    let content = "command arg -f, --flag -o, --other:\n  echo \"Running command\"";
    let tokens = tokenizer.tokenize(content).unwrap();
    let runfile = parser.parse(tokens).unwrap();

    assert_eq!(runfile.commands.len(), 1);
    let cmd = &runfile.commands[0];
    assert_eq!(cmd.names, vec!["command"]);
    assert_eq!(cmd.args.len(), 1);
    assert_eq!(cmd.args[0].name, "arg");
    assert_eq!(cmd.args[0].optional, false);
    assert_eq!(cmd.args[0].is_varargs, false);
    assert_eq!(cmd.flags.len(), 2);
    assert_eq!(cmd.flags[0].short, Some('f'));
    assert_eq!(cmd.flags[0].long, "flag");
    assert_eq!(cmd.flags[0].takes_value, false);
    assert_eq!(cmd.flags[1].short, Some('o'));
    assert_eq!(cmd.flags[1].long, "other");
    assert_eq!(cmd.flags[1].takes_value, false);
  }

  #[test]
  fn test_parse_command_with_aliases() {
    let tokenizer = TokenizePhase::new();
    let parser = ParsePhase::new();

    let content = "b, build, compile:\n  echo \"Building\"";
    let tokens = tokenizer.tokenize(content).unwrap();
    let runfile = parser.parse(tokens).unwrap();

    assert_eq!(runfile.commands.len(), 1);
    let cmd = &runfile.commands[0];
    assert_eq!(cmd.names, vec!["b", "build", "compile"]);
    assert_eq!(cmd.script.trim(), "echo \"Building\"");
  }

  #[test]
  fn test_parse_command_with_comment() {
    let tokenizer = TokenizePhase::new();
    let parser = ParsePhase::new();

    let content = "# This is a comment\nhello:\n  echo \"Hello\"";
    let tokens = tokenizer.tokenize(content).unwrap();
    let runfile = parser.parse(tokens).unwrap();

    assert_eq!(runfile.commands.len(), 1);
    let cmd = &runfile.commands[0];
    assert_eq!(cmd.names, vec!["hello"]);
    assert_eq!(cmd.description, Some("This is a comment".to_string()));
  }

  #[test]
  fn test_parse_command_with_multiple_comments() {
    let tokenizer = TokenizePhase::new();
    let parser = ParsePhase::new();

    let content = "# First comment\n# Second comment\nhello:\n  echo \"Hello\"";
    let tokens = tokenizer.tokenize(content).unwrap();
    let runfile = parser.parse(tokens).unwrap();

    assert_eq!(runfile.commands.len(), 1);
    let cmd = &runfile.commands[0];
    assert_eq!(cmd.names, vec!["hello"]);
    assert_eq!(cmd.description, Some("First comment Second comment".to_string()));
  }

  #[test]
  fn test_parse_command_with_varargs() {
    let tokenizer = TokenizePhase::new();
    let parser = ParsePhase::new();

    let content = "run ...args:\n  echo \"Running with args\"";
    let tokens = tokenizer.tokenize(content).unwrap();
    let runfile = parser.parse(tokens).unwrap();

    assert_eq!(runfile.commands.len(), 1);
    let cmd = &runfile.commands[0];
    assert_eq!(cmd.names, vec!["run"]);
    assert_eq!(cmd.args.len(), 1);
    assert_eq!(cmd.args[0].name, "args");
    assert_eq!(cmd.args[0].optional, true);
    assert_eq!(cmd.args[0].is_varargs, true);

    // Test help output
    let help = runfile.generate_help(false);
    assert!(help.contains("...args"), "Help should contain '...args' but got: {}", help);
  }

  #[test]
  fn test_parse_command_with_varargs_indented() {
    let tokenizer = TokenizePhase::new();
    let parser = ParsePhase::new();

    let content = "run\n  target\n  ...args:\n  echo \"Running with args\"";
    let tokens = tokenizer.tokenize(content).unwrap();

    // Debug: print tokens
    eprintln!("Tokens:");
    for token in &tokens {
      eprintln!("  {:?}", token);
    }

    let runfile = parser.parse(tokens).unwrap();

    assert_eq!(runfile.commands.len(), 1);
    let cmd = &runfile.commands[0];
    assert_eq!(cmd.names, vec!["run"]);
    assert_eq!(cmd.args.len(), 2);
    assert_eq!(cmd.args[0].name, "target");
    assert_eq!(cmd.args[1].name, "args");
    assert_eq!(cmd.args[1].optional, true);
    assert_eq!(cmd.args[1].is_varargs, true);

    // Test help output
    let help = runfile.generate_help(false);
    eprintln!("Help output:\n{}", help);
    assert!(help.contains("...args"), "Help should contain '...args' but got: {}", help);
  }

  #[test]
  fn test_parse_varargs_runfile() {
    let tokenizer = TokenizePhase::new();
    let parser = ParsePhase::new();

    let content = std::fs::read_to_string("tests/samples/varargs.runfile").unwrap();
    let tokens = tokenizer.tokenize(&content).unwrap();

    // Debug: print tokens
    eprintln!("Tokens from varargs.runfile:");
    for token in &tokens {
      eprintln!("  {:?}", token);
    }

    let runfile = parser.parse(tokens).unwrap();

    // Debug: print parsed commands
    eprintln!("\nParsed commands:");
    for cmd in &runfile.commands {
      eprintln!("Command: {:?}", cmd.names);
      for arg in &cmd.args {
        eprintln!("  Arg: name={}, optional={}, is_varargs={}",
                 arg.name, arg.optional, arg.is_varargs);
      }
    }

    // Test help output
    let help = runfile.generate_help(false);
    eprintln!("\nHelp output from varargs.runfile:\n{}", help);
    assert!(help.contains("...args"), "Help should contain '...args' but got: {}", help);
  }

  #[test]
  fn test_parse_command_with_value_flag() {
    let tokenizer = TokenizePhase::new();
    let parser = ParsePhase::new();

    let content = "build:\n  --output=<file>\n  echo \"Building\"";
    let tokens = tokenizer.tokenize(content).unwrap();
    let runfile = parser.parse(tokens).unwrap();

    assert_eq!(runfile.commands.len(), 1);
    let cmd = &runfile.commands[0];
    assert_eq!(cmd.names, vec!["build"]);
    assert_eq!(cmd.flags.len(), 1);
    assert_eq!(cmd.flags[0].long, "output");
    assert_eq!(cmd.flags[0].takes_value, true);
    assert_eq!(cmd.flags[0].type_hint, Some("file".to_string()));
  }

  #[test]
  fn test_parse_command_with_shebang() {
    let tokenizer = TokenizePhase::new();
    let parser = ParsePhase::new();

    let content = "python:\n  #!/usr/bin/env python3\n  print('Hello')";
    let tokens = tokenizer.tokenize(content).unwrap();
    let runfile = parser.parse(tokens).unwrap();

    assert_eq!(runfile.commands.len(), 1);
    let cmd = &runfile.commands[0];
    assert_eq!(cmd.names, vec!["python"]);
    assert_eq!(cmd.shebang, "#!/usr/bin/env python3");
    assert!(cmd.script.contains("print('Hello')"));
  }

  #[test]
  fn test_parse_command_with_argument_comment() {
    let tokenizer = TokenizePhase::new();
    let parser = ParsePhase::new();

    let content = "build:\n  target # The build target\n  echo \"Building\"";
    let tokens = tokenizer.tokenize(content).unwrap();
    let runfile = parser.parse(tokens).unwrap();

    assert_eq!(runfile.commands.len(), 1);
    let cmd = &runfile.commands[0];
    assert_eq!(cmd.names, vec!["build"]);
    assert_eq!(cmd.args.len(), 1);
    assert_eq!(cmd.args[0].name, "target");
    assert_eq!(cmd.args[0].description, Some("The build target".to_string()));
  }

  #[test]
  fn test_parse_command_with_flag_comment() {
    let tokenizer = TokenizePhase::new();
    let parser = ParsePhase::new();

    let content = "build:\n  --debug # Enable debug mode\n  echo \"Building\"";
    let tokens = tokenizer.tokenize(content).unwrap();
    let runfile = parser.parse(tokens).unwrap();

    assert_eq!(runfile.commands.len(), 1);
    let cmd = &runfile.commands[0];
    assert_eq!(cmd.names, vec!["build"]);
    assert_eq!(cmd.flags.len(), 1);
    assert_eq!(cmd.flags[0].long, "debug");
    assert_eq!(cmd.flags[0].description, Some("Enable debug mode".to_string()));
  }

  #[test]
  fn test_parse_group_header() {
    let tokenizer = TokenizePhase::new();
    let parser = ParsePhase::new();

    let content = "# ----------\n# Build Commands\n# ----------\n\nbuild:\n  echo \"Building\"";
    let tokens = tokenizer.tokenize(content).unwrap();
    let runfile = parser.parse(tokens).unwrap();

    assert_eq!(runfile.groups.len(), 1);
    assert_eq!(runfile.groups[0].name, "Build Commands");
    assert_eq!(runfile.commands.len(), 1);
    assert_eq!(runfile.commands[0].group, Some("Build Commands".to_string()));
  }

  #[test]
  fn test_parse_multiple_commands() {
    let tokenizer = TokenizePhase::new();
    let parser = ParsePhase::new();

    let content = "build:\n  echo \"Building\"\n\nrun:\n  echo \"Running\"";
    let tokens = tokenizer.tokenize(content).unwrap();
    let runfile = parser.parse(tokens).unwrap();

    assert_eq!(runfile.commands.len(), 2);
    assert_eq!(runfile.commands[0].names, vec!["build"]);
    assert_eq!(runfile.commands[1].names, vec!["run"]);
  }

  #[test]
  fn test_parse_empty_runfile() {
    let tokenizer = TokenizePhase::new();
    let parser = ParsePhase::new();

    let content = "";
    let tokens = tokenizer.tokenize(content).unwrap();
    let runfile = parser.parse(tokens).unwrap();

    assert_eq!(runfile.groups.len(), 0);
    assert_eq!(runfile.commands.len(), 0);
  }

  #[test]
  fn test_parse_command_with_script_comments() {
    let tokenizer = TokenizePhase::new();
    let parser = ParsePhase::new();

    let content = "build:\n  # This is a script comment\n  echo \"Building\"\n  # Another comment\n  echo \"Done\"";
    let tokens = tokenizer.tokenize(content).unwrap();
    let runfile = parser.parse(tokens).unwrap();

    assert_eq!(runfile.commands.len(), 1);
    let cmd = &runfile.commands[0];
    assert_eq!(cmd.names, vec!["build"]);
    assert!(cmd.script.contains("# This is a script comment"));
    assert!(cmd.script.contains("echo \"Building\""));
    assert!(cmd.script.contains("# Another comment"));
    assert!(cmd.script.contains("echo \"Done\""));
  }

  #[test]
  fn test_parse_command_with_inline_aliases_and_args() {
    let tokenizer = TokenizePhase::new();
    let parser = ParsePhase::new();

    let content = "b, build target -r, --release:\n  echo \"Building\"";
    let tokens = tokenizer.tokenize(content).unwrap();
    let runfile = parser.parse(tokens).unwrap();

    assert_eq!(runfile.commands.len(), 1);
    let cmd = &runfile.commands[0];
    assert_eq!(cmd.names, vec!["b", "build"]);
    assert_eq!(cmd.args.len(), 1);
    assert_eq!(cmd.args[0].name, "target");
    assert_eq!(cmd.flags.len(), 1);
    assert_eq!(cmd.flags[0].short, Some('r'));
    assert_eq!(cmd.flags[0].long, "release");
  }
}
