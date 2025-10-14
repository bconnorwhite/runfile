use anyhow::Result;
use run::execute_command;

fn main() -> Result<()> {
  let args: Vec<String> = std::env::args().collect();
  let command_args = args[1..].to_vec();
  execute_command(&command_args)?;
  Ok(())
}
