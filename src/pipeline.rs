use anyhow::Result;
use std::path::PathBuf;
use std::fs;

use crate::phases::{TokenizePhase, ParsePhase, ResolvePhase, RunPhase};

pub struct Pipeline {
  pub tokenize: TokenizePhase,
  pub parse: ParsePhase,
  pub resolve: ResolvePhase,
  pub run: RunPhase,
}

impl Pipeline {
  pub fn new() -> Self {
    Self {
      tokenize: TokenizePhase::new(),
      parse: ParsePhase::new(),
      resolve: ResolvePhase::new(),
      run: RunPhase::new(),
    }
  }

  pub fn find_runfile(&self) -> Result<PathBuf> {
    let mut current_dir = std::env::current_dir()?;

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

    Err(anyhow::anyhow!("No Runfile found in current directory or parent directories"))
  }

  pub fn execute_command(&self, command_name: &str, cli_args: Vec<String>) -> Result<()> {
    // Phase 1: Find and read Runfile
    let runfile_path = self.find_runfile()?;
    let content = fs::read_to_string(&runfile_path)?;

    // Phase 2: Tokenize
    let tokens = self.tokenize.tokenize(&content)?;

    // Phase 3: Parse
    let runfile = self.parse.parse(tokens)?;

    // Phase 4: Resolve
    let command = self.resolve.resolve(runfile, command_name)?;

    // Phase 5: Run
    self.run.run(command, cli_args)?;

    Ok(())
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
  use super::*;
  use std::fs;
  use tempfile::TempDir;

  #[test]
  fn test_find_runfile_in_current_dir() {
    use std::env;
    use tempfile::TempDir;

    // Create a temporary directory for the test
    let temp_dir = TempDir::new().unwrap();
    let original_dir = env::current_dir().unwrap();

    // Change to the temporary directory
    env::set_current_dir(&temp_dir).unwrap();

    let pipeline = Pipeline::new();

    // Create a temporary Runfile in the temp directory
    let test_content = "test:\n  echo \"Hello\"";
    fs::write("Runfile", test_content).unwrap();

    let result = pipeline.find_runfile();
    assert!(result.is_ok());

    // Restore original directory
    env::set_current_dir(original_dir).unwrap();
    // TempDir will be automatically cleaned up when it goes out of scope
  }

  #[test]
  fn test_find_runfile_not_found() {
    // Create a temporary directory with no Runfile
    let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path();

    // Change to the temporary directory
    let original_dir = std::env::current_dir().unwrap();
    std::env::set_current_dir(&temp_path).unwrap();

    let pipeline = Pipeline::new();
    let result = pipeline.find_runfile();

    // Restore original directory
    std::env::set_current_dir(&original_dir).unwrap();

    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("No Runfile found"));
  }
}
