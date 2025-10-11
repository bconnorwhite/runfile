use anyhow::Result;
use run::Pipeline;

fn main() -> Result<()> {
  let args: Vec<String> = std::env::args().collect();
  let pipeline = Pipeline::new();

  if args.len() < 2 {
    // No command provided, show help
    pipeline.show_help(true)?;
  } else {
    let command_name = &args[1];
    let command_args = args[2..].to_vec();
    pipeline.execute_command(command_name, command_args)?;
  }

  Ok(())
}
