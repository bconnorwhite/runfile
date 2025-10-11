pub mod phases;
pub mod pipeline;

use anyhow::Result;
use std::path::PathBuf;

use crate::phases::{TokenizePhase, ParsePhase};

// Re-export the main Pipeline struct for convenience
pub use crate::pipeline::Pipeline;

/// Find a runfile in the current directory or parent directories
pub fn find_runfile() -> Result<PathBuf> {
  let pipeline = Pipeline::new();
  pipeline.find_runfile()
}

/// Parse a runfile content string and return the parsed Runfile structure
pub fn parse_runfile(content: &str) -> Result<crate::phases::parse::Runfile> {
  let tokenize = TokenizePhase::new();
  let parse = ParsePhase::new();

  let tokens = tokenize.tokenize(content)?;
  let runfile = parse.parse(tokens)?;

  Ok(runfile)
}

/// Execute a command from a runfile
pub fn execute_command(runfile_content: &str, command_name: &str, cli_args: Vec<String>) -> Result<()> {
  let pipeline = Pipeline::new();

  // Parse the runfile content
  let tokens = pipeline.tokenize.tokenize(runfile_content)?;
  let runfile = pipeline.parse.parse(tokens)?;

  // Resolve and run the command
  let command = pipeline.resolve.resolve(runfile, command_name)?;
  pipeline.run.run(command, cli_args)?;

  Ok(())
}
