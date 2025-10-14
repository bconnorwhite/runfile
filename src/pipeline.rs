use std::{fs, path::PathBuf, process::Output};

use anyhow::Result;

use crate::phases::{ParsePhase, ResolvePhase, RunPhase, TokenizePhase, run::OutputMode};

#[derive(Default)]
pub struct PipelineOptions {
  pub directory: Option<PathBuf>,
}

pub struct Pipeline {
  pub tokenize: TokenizePhase,
  pub parse: ParsePhase,
  pub resolve: ResolvePhase,
  pub run: RunPhase,
  pub options: PipelineOptions,
}

impl Default for Pipeline {
  fn default() -> Self {
    Self::with_options(PipelineOptions::default())
  }
}

impl Pipeline {
  pub fn new() -> Self {
    Self::with_options(PipelineOptions::default())
  }
  pub fn with_options(options: PipelineOptions) -> Self {
    Self {
      tokenize: TokenizePhase::new(),
      parse: ParsePhase::new(),
      resolve: ResolvePhase::new(),
      run: RunPhase::new(),
      options,
    }
  }
  pub fn find_runfile(&self) -> Result<PathBuf> {
    let mut current_dir = if let Some(dir) = &self.options.directory {
      dir.clone()
    } else {
      std::env::current_dir()?
    };
    loop {
      let runfile_path = current_dir.join("Runfile");
      if runfile_path.exists() {
        return Ok(runfile_path);
      }
      if let Some(parent) = current_dir.parent() {
        current_dir = parent.to_path_buf();
      } else {
        break;
      }
    }
    Err(anyhow::anyhow!(
      "No Runfile found in current directory or parent directories"
    ))
  }
  pub fn execute_command_inherit(&self, command_name: &str, cli_args: Vec<String>) -> Result<()> {
    // Phase 1: Find and read Runfile
    let runfile_path = self.find_runfile()?;
    let content = fs::read_to_string(&runfile_path)?;
    // Phase 2: Tokenize
    let tokens = self.tokenize.tokenize(&content)?;
    // Phase 3: Parse
    let runfile = self.parse.parse(tokens)?;
    // Phase 4: Resolve
    let command = self.resolve.resolve(runfile, command_name)?;
    // Phase 5: Run with inherit mode
    self.run.run(command, cli_args, OutputMode::Inherit)?;
    Ok(())
  }
  pub fn execute_command(&self, command_name: &str, cli_args: Vec<String>) -> Result<Output> {
    // Phase 1: Find and read Runfile
    let runfile_path = self.find_runfile()?;
    let content = fs::read_to_string(&runfile_path)?;
    // Phase 2: Tokenize
    let tokens = self.tokenize.tokenize(&content)?;
    // Phase 3: Parse
    let runfile = self.parse.parse(tokens)?;
    // Phase 4: Resolve
    let command = self.resolve.resolve(runfile, command_name)?;
    // Phase 5: Run with capture mode
    let output = self.run.run(command, cli_args, OutputMode::Capture)?;
    output.ok_or_else(|| anyhow::anyhow!("Expected output from capture mode"))
  }
  pub fn show_help(&self, colors: bool) -> Result<()> {
    // Find and read Runfile
    let runfile_path = self.find_runfile()?;
    let content = fs::read_to_string(&runfile_path)?;
    // Tokenize and parse
    let tokens = self.tokenize.tokenize(&content)?;
    let runfile = self.parse.parse(tokens)?;
    // Generate help output
    runfile.generate_help_output(colors);
    Ok(())
  }
}

#[cfg(test)]
mod tests {
  use std::fs;

  use tempfile::TempDir;

  use super::*;

  #[test]
  fn test_find_runfile_in_current_dir() {
    // Create a temporary directory for the test
    let temp_dir = TempDir::new().unwrap();

    let pipeline = Pipeline::with_options(PipelineOptions {
      directory: Some(temp_dir.path().to_path_buf()),
    });

    // Create a temporary Runfile in the temp directory
    let test_content = "test:\n  echo \"Hello\"";
    fs::write(temp_dir.path().join("Runfile"), test_content).unwrap();

    let result = pipeline.find_runfile();
    assert!(result.is_ok());
  }

  #[test]
  fn test_find_runfile_not_found() {
    // Create a temporary directory with no Runfile
    let temp_dir = TempDir::new().unwrap();

    let pipeline = Pipeline::with_options(PipelineOptions {
      directory: Some(temp_dir.path().to_path_buf()),
    });
    let result = pipeline.find_runfile();

    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("No Runfile found"));
  }

  #[test]
  fn test_execute_command_with_suffix_varargs_zero_args() {
    let temp_dir = TempDir::new().unwrap();

    // Suffix varargs form should be accepted and optional
    let runfile_content = "test args...:\n  echo OK\n";
    fs::write(temp_dir.path().join("Runfile"), runfile_content).unwrap();

    let pipeline = Pipeline::with_options(PipelineOptions {
      directory: Some(temp_dir.path().to_path_buf()),
    });
    let result = pipeline.execute_command("test", vec![]);

    // Should succeed with zero args for varargs
    assert!(
      result.is_ok(),
      "expected success with zero args, got: {:?}",
      result
    );

    // Verify output contains "OK"
    let output = result.unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(
      stdout.contains("OK"),
      "expected output to contain 'OK', got: {}",
      stdout
    );
  }
}
