use clap::Parser;
use standby::commands::Cli;

fn main() {
    let cli = Cli::parse();

    if let Err(e) = cli.execute() {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}
