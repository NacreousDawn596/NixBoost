use clap::Parser;

pub const VERSION: &str = "1.0.6";

#[derive(Parser)]
#[command(name = "nixboost")]
#[command(author = "nixboost Team")]
#[command(version = VERSION)]
#[command(about = "High-performance NixOS package manager frontend.")]
pub struct Cli {
    #[arg(short = 'S', long)]
    pub sync: bool,
    #[arg(short = 'R', long)]
    pub remove: bool,
    #[arg(short = 's', long)]
    pub search: bool,
    #[arg(short = 'A', long)]
    pub nur: bool,
    #[arg(long)]
    pub history: bool,
    #[arg(long)]
    pub clean: bool,
    #[arg(short = 'l', long)]
    pub list: bool,
    #[arg(long)]
    pub news: bool,
    #[arg(long)]
    pub health: bool,
    #[arg(value_name = "TARGETS")]
    pub targets: Vec<String>,
}
