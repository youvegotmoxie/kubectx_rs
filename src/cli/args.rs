use clap::Parser;
use std::path::PathBuf;
use yaml_serde::Value;

#[derive(Parser)]
#[command(version, about, long_about = None)]
pub struct Command {
    #[arg(short, long, value_name = "NAME")]
    pub delete: Option<Value>,

    #[arg(short, long, value_name = "NAMESPACE")]
    pub namespace: Option<Value>,

    #[arg(short, long)]
    pub unset_context: bool,

    #[arg(short, long, value_name = "PATH")]
    pub config: Option<PathBuf>,

    #[arg(value_name = "NAME", value_enum)]
    pub set: Option<Value>,
}
