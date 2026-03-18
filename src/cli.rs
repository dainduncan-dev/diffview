use clap::Parser;
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(name = "diffview", version, about = "Side-by-side git diff viewer")]
pub struct Cli {
    #[arg(long, help = "Read diff from stdin")]
    pub stdin: bool,

    #[arg(long, value_name = "PATH", help = "Read diff from file")]
    pub diff_file: Option<PathBuf>,

    #[arg(last = true, value_name = "GIT_DIFF_ARGS")]
    pub git_args: Vec<String>,
}
