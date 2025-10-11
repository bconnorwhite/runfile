use anyhow::{Result, anyhow};
use super::parse::{Runfile, Command};

pub struct ResolvePhase;

impl ResolvePhase {
  pub fn new() -> Self {
    Self
  }

  pub fn resolve(&self, runfile: Runfile, target_command: &str) -> Result<Command> {
    // Find the command by name or alias
    let command = runfile.commands
      .into_iter()
      .find(|cmd| cmd.names.contains(&target_command.to_string()))
      .ok_or_else(|| anyhow!("Command '{}' not found", target_command))?;

    // Validate the command structure
    self.validate_command(&command)?;

    Ok(command)
  }

  fn validate_command(&self, command: &Command) -> Result<()> {
    // Check for duplicate argument names
    let mut arg_names = std::collections::HashSet::new();
    let mut varargs_count = 0;
    let mut varargs_position = None;

    for (i, arg) in command.args.iter().enumerate() {
      if !arg_names.insert(arg.name.clone()) {
        return Err(anyhow!("Duplicate argument name: {}", arg.name));
      }

      if arg.is_varargs {
        varargs_count += 1;
        varargs_position = Some(i);
      }
    }

    // Validate varargs rules
    if varargs_count > 1 {
      return Err(anyhow!("Only one varargs argument (...args) is allowed"));
    }

    if let Some(pos) = varargs_position {
      if pos != command.args.len() - 1 {
        return Err(anyhow!("Varargs argument (...args) must be the last argument"));
      }
    }

    // Check for duplicate flag names
    let mut flag_names = std::collections::HashSet::new();
    for flag in &command.flags {
      if !flag_names.insert(flag.long.clone()) {
        return Err(anyhow!("Duplicate flag name: {}", flag.long));
      }

      if let Some(short) = flag.short {
        let short_str = short.to_string();
        if !flag_names.insert(short_str) {
          return Err(anyhow!("Duplicate short flag: -{}", short));
        }
      }
    }

    // Validate script is not empty
    if command.script.trim().is_empty() {
      return Err(anyhow!("Command '{}' has no script body", command.names.first().unwrap_or(&"unknown".to_string())));
    }

    Ok(())
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::phases::parse::{Runfile, Command, Argument};

  #[test]
  fn test_resolve_finds_command() {
    let resolver = ResolvePhase::new();
    let runfile = Runfile {
      groups: vec![],
      commands: vec![
        Command {
          names: vec!["test".to_string()],
          description: None,
          group: None,
          args: vec![],
          flags: vec![],
          script: "echo test".to_string(),
          shebang: "#!/bin/sh".to_string(),
        }
      ],
    };

    let command = resolver.resolve(runfile, "test").unwrap();
    assert_eq!(command.names, vec!["test"]);
  }

  #[test]
  fn test_resolve_command_not_found() {
    let resolver = ResolvePhase::new();
    let runfile = Runfile {
      groups: vec![],
      commands: vec![],
    };

    let result = resolver.resolve(runfile, "nonexistent");
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Command 'nonexistent' not found"));
  }

  #[test]
  fn test_resolve_duplicate_args() {
    let resolver = ResolvePhase::new();
    let runfile = Runfile {
      groups: vec![],
      commands: vec![
        Command {
          names: vec!["test".to_string()],
          description: None,
          group: None,
          args: vec![
            Argument { name: "arg1".to_string(), optional: false, is_varargs: false, description: None },
            Argument { name: "arg1".to_string(), optional: true, is_varargs: false, description: None },
          ],
          flags: vec![],
          script: "echo test".to_string(),
          shebang: "#!/bin/sh".to_string(),
        }
      ],
    };

    let result = resolver.resolve(runfile, "test");
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Duplicate argument name"));
  }

  #[test]
  fn test_resolve_empty_script() {
    let resolver = ResolvePhase::new();
    let runfile = Runfile {
      groups: vec![],
      commands: vec![
        Command {
          names: vec!["test".to_string()],
          description: None,
          group: None,
          args: vec![],
          flags: vec![],
          script: "".to_string(),
          shebang: "#!/bin/sh".to_string(),
        }
      ],
    };

    let result = resolver.resolve(runfile, "test");
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("has no script body"));
  }
}
