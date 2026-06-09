use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "gh-pr-lander", version, about)]
pub struct Cli {
    /// Use a specific config file instead of the global one.
    #[arg(short, long, value_name = "PATH")]
    pub config: Option<PathBuf>,
}
