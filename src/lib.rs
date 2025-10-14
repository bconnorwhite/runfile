pub mod phases;
pub mod pipeline;

use std::path::PathBuf;

use anyhow::Result;

use crate::phases::{ParsePhase, TokenizePhase};
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

/// Execute a command

pub fn execute_command(args: &[String]) -> Result<()> {
  let pipeline = Pipeline::new();
  if args.is_empty() {
    // No command provided, show help
    pipeline.show_help(true)?;
  } else {
    let command_name = &args[0];
    let cli_args = args[1..].to_vec();
    pipeline.execute_command_inherit(command_name, cli_args)?;
  }
  Ok(())
}
