use clap::Parser;
use std::path::PathBuf;
use yaml_serde::Value;

#[derive(Parser)]
#[command(version, about, long_about = None)]
pub struct Cli {
    #[arg(short, long, value_name = "NAME")]
    pub delete: Option<Value>,

    #[arg(short, long, value_name = "PATH")]
    pub config: Option<PathBuf>,

    #[arg(value_name = "NAME", value_enum)]
    pub set: Option<Value>,
}
