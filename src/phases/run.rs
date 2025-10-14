use std::{
  collections::{HashMap, HashSet},
  process::{Command as ProcessCommand, Output, Stdio},
};

use anyhow::{Result, anyhow};

use super::parse::Command;

// Type aliases for complex return types
type CliArgsResult = (Vec<String>, HashSet<String>, HashMap<String, String>);

#[derive(Clone, Copy, Debug)]
pub enum OutputMode {
  Inherit,
  Capture,
}

#[derive(Default)]
pub struct RunPhase;

impl RunPhase {
  pub fn new() -> Self {
    Self
  }
  pub fn run(&self, command: Command, cli_args: Vec<String>, mode: OutputMode) -> Result<Option<Output>> {
    // Parse CLI arguments and flags
    let (provided_args, provided_flags, provided_flag_values) = self.parse_cli_args(&command, cli_args)?;
    // Validate required arguments are provided
    self.validate_required_args(&command, &provided_args)?;
    // Set up environment variables
    let mut env_vars = HashMap::new();
    // Set argument values (both UPPER_SNAKE and lower_snake)
    for (i, arg) in command.args.iter().enumerate() {
      if let Some(value) = provided_args.get(i) {
        // UPPER_SNAKE for values
        env_vars.insert(arg.name.to_uppercase(), value.clone());
        // lower_snake for convenience (same value)
        env_vars.insert(arg.name.clone(), value.clone());
      }
    }
    // Set flag values (both UPPER_SNAKE and lower_snake)
    for flag in &command.flags {
      if let Some(value) = provided_flag_values.get(&flag.long) {
        // Value flag: set both UPPER_SNAKE and lower_snake
        env_vars.insert(flag.long.to_uppercase(), value.clone());
        env_vars.insert(flag.long.clone(), format!("--{}={}", flag.long, value));
      } else if provided_flags.contains(&flag.long) {
        // Boolean flag: set both UPPER_SNAKE and lower_snake
        env_vars.insert(flag.long.to_uppercase(), "true".to_string());
        // Use the flag the user provided (short or long)
        let flag_string = if let Some(short) = flag.short {
          format!("-{}", short)
        } else {
          format!("--{}", flag.long)
        };
        env_vars.insert(flag.long.clone(), flag_string);
      }
    }
    // Execute the script
    self.execute_script(&command, env_vars, mode)
  }
  fn parse_cli_args(&self, command: &Command, cli_args: Vec<String>) -> Result<CliArgsResult> {
    let mut provided_args = Vec::new();
    let mut provided_flags = HashSet::new();
    let mut provided_flag_values = HashMap::new();
    let mut i = 0;
    // Find varargs argument if it exists
    let varargs_arg = command.args.iter().find(|arg| arg.is_varargs);
    while i < cli_args.len() {
      let arg = &cli_args[i];
      if arg.starts_with("--") {
        // Long flag
        if arg.contains('=') {
          // Value flag: --flag=value
          let parts: Vec<&str> = arg.splitn(2, '=').collect();
          if parts.len() == 2 {
            let flag_name = parts[0].strip_prefix("--").unwrap().to_string();
            let flag_value = parts[1].to_string();
            if let Some(flag) = command
              .flags
              .iter()
              .find(|f| f.long == flag_name && f.takes_value)
            {
              provided_flag_values.insert(flag.long.clone(), flag_value);
            } else {
              return Err(anyhow!("Unknown value flag: --{}", flag_name));
            }
          }
        } else {
          // Boolean flag: --flag
          let flag_name = arg.strip_prefix("--").unwrap().to_string();
          if let Some(flag) = command
            .flags
            .iter()
            .find(|f| f.long == flag_name && !f.takes_value)
          {
            provided_flags.insert(flag.long.clone());
          } else {
            return Err(anyhow!("Unknown flag: --{}", flag_name));
          }
        }
      } else if arg.starts_with('-') && arg.len() == 2 {
        // Short flag
        let short_char = arg.chars().nth(1).unwrap();
        if let Some(flag) = command.flags.iter().find(|f| f.short == Some(short_char)) {
          if flag.takes_value {
            // Value flag: need to get the value from next argument
            if i + 1 < cli_args.len() {
              let flag_value = cli_args[i + 1].clone();
              provided_flag_values.insert(flag.long.clone(), flag_value);
              i += 1; // Skip the value argument
            } else {
              return Err(anyhow!("Flag -{} requires a value", short_char));
            }
          } else {
            // Boolean flag
            provided_flags.insert(flag.long.clone());
          }
        } else {
          return Err(anyhow!("Unknown short flag: -{}", short_char));
        }
      } else {
        // Positional argument
        provided_args.push(arg.clone());
      }
      i += 1;
    }
    // Handle varargs: collect remaining args if varargs is present
    if let Some(varargs_arg) = varargs_arg {
      let varargs_position = command
        .args
        .iter()
        .position(|a| a.name == varargs_arg.name)
        .unwrap();
      if provided_args.len() > varargs_position {
        // Collect all remaining args into varargs
        let varargs: Vec<String> = provided_args.split_off(varargs_position);
        let varargs_string = varargs.join(" ");
        provided_args.push(varargs_string);
      }
    }
    Ok((provided_args, provided_flags, provided_flag_values))
  }
  fn validate_required_args(&self, command: &Command, provided_args: &[String]) -> Result<()> {
    for arg in &command.args {
      if !arg.optional {
        let arg_index = command
          .args
          .iter()
          .position(|a| a.name == arg.name)
          .unwrap();
        if arg_index >= provided_args.len() {
          return Err(anyhow!("Required argument '{}' not provided", arg.name));
        }
      }
    }
    Ok(())
  }
  fn execute_script(
    &self,
    command: &Command,
    env_vars: HashMap<String, String>,
    mode: OutputMode,
  ) -> Result<Option<Output>> {
    // Extract the shell from shebang
    let shell = if command.shebang.starts_with("#!") {
      command.shebang.strip_prefix("#!").unwrap().trim()
    } else {
      "sh"
    };
    // Create the command
    let mut cmd = ProcessCommand::new(shell);
    cmd.arg("-c").arg(&command.script);
    // Set environment variables
    for (key, value) in env_vars {
      cmd.env(&key, &value);
    }
    // Execute based on mode
    match mode {
      OutputMode::Inherit => {
        cmd.stdout(Stdio::inherit()).stderr(Stdio::inherit());
        let status = cmd.status()?;
        if !status.success() {
          return Err(anyhow!(
            "Command failed with exit code: {}",
            status.code().unwrap_or(-1)
          ));
        }
        Ok(None)
      }
      OutputMode::Capture => {
        cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
        let output = cmd.output()?;
        if !output.status.success() {
          return Err(anyhow!(
            "Command failed with exit code: {}",
            output.status.code().unwrap_or(-1)
          ));
        }
        Ok(Some(output))
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::phases::parse::{Argument, Command, Flag};

  #[test]
  fn test_parse_cli_args() {
    let run_phase = RunPhase::new();
    let command = Command {
      names: vec!["test".to_string()],
      description: None,
      group: None,
      args: vec![
        Argument {
          name: "arg1".to_string(),
          optional: false,
          is_varargs: false,
          description: None,
        },
        Argument {
          name: "arg2".to_string(),
          optional: true,
          is_varargs: false,
          description: None,
        },
      ],
      flags: vec![
        Flag {
          short: Some('r'),
          long: "release".to_string(),
          takes_value: false,
          type_hint: None,
          description: None,
        },
        Flag {
          short: None,
          long: "debug".to_string(),
          takes_value: false,
          type_hint: None,
          description: None,
        },
      ],
      script: "echo test".to_string(),
      shebang: "#!/bin/sh".to_string(),
    };

    let cli_args = vec![
      "value1".to_string(),
      "--release".to_string(),
      "--debug".to_string(),
    ];
    let (args, flags, flag_values) = run_phase.parse_cli_args(&command, cli_args).unwrap();

    assert_eq!(args, vec!["value1"]);
    assert!(flags.contains("release"));
    assert!(flags.contains("debug"));
    assert!(flag_values.is_empty());
  }

  #[test]
  fn test_parse_cli_args_short_flag() {
    let run_phase = RunPhase::new();
    let command = Command {
      names: vec!["test".to_string()],
      description: None,
      group: None,
      args: vec![],
      flags: vec![Flag {
        short: Some('r'),
        long: "release".to_string(),
        takes_value: false,
        type_hint: None,
        description: None,
      }],
      script: "echo test".to_string(),
      shebang: "#!/bin/sh".to_string(),
    };

    let cli_args = vec!["-r".to_string()];
    let (args, flags, flag_values) = run_phase.parse_cli_args(&command, cli_args).unwrap();

    assert_eq!(args.len(), 0);
    assert!(flags.contains("release"));
    assert!(flag_values.is_empty());
  }

  #[test]
  fn test_parse_cli_args_unknown_flag() {
    let run_phase = RunPhase::new();
    let command = Command {
      names: vec!["test".to_string()],
      description: None,
      group: None,
      args: vec![],
      flags: vec![],
      script: "echo test".to_string(),
      shebang: "#!/bin/sh".to_string(),
    };

    let cli_args = vec!["--unknown".to_string()];
    let result = run_phase.parse_cli_args(&command, cli_args);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Unknown flag"));
  }

  #[test]
  fn test_validate_required_args() {
    let run_phase = RunPhase::new();
    let command = Command {
      names: vec!["test".to_string()],
      description: None,
      group: None,
      args: vec![
        Argument {
          name: "required".to_string(),
          optional: false,
          is_varargs: false,
          description: None,
        },
        Argument {
          name: "optional".to_string(),
          optional: true,
          is_varargs: false,
          description: None,
        },
      ],
      flags: vec![],
      script: "echo test".to_string(),
      shebang: "#!/bin/sh".to_string(),
    };

    // Should pass with required arg provided
    let args = vec!["value".to_string()];
    assert!(run_phase.validate_required_args(&command, &args).is_ok());

    // Should fail without required arg
    let args = vec![];
    let result = run_phase.validate_required_args(&command, &args);
    assert!(result.is_err());
    assert!(
      result
        .unwrap_err()
        .to_string()
        .contains("Required argument")
    );
  }
}
