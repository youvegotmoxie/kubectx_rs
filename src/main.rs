use clap::Parser;

mod cli;
use cli::args::Command;
use cli::kubeconfig::mutate_contexts::*;
use cli::kubeconfig::setup_kubeconfig::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = cli::args::Command::parse();

    // If -c/--config is set then override the KUBECONFIG env var
    let path = match cli.config {
        Some(kube_config_path) => kube_config_path,
        None => kubeconfig_path()?,
    };

    let kube_yaml = kubeconfig_to_yaml(path.clone())?;

    match cli {
        // If -d/--delete is set then delete the cluster from KUBECONFIG
        // This only removes the entry from the contexts map
        Command {
            delete: Some(name), ..
        } => {
            delete_context(path, name, &kube_yaml)?;
        }
        // If a cluster name is provided as the only argument and passes validation, set this to current-context
        Command {
            set: Some(name), ..
        } => {
            set_context(path, &kube_yaml, &name)?;
        }
        Command {
            unset_context: true,
            ..
        } => {
            unset_context(path, &kube_yaml)?;
        }
        // If -n/--namespace is set, set the namespace for the current context
        Command {
            namespace: Some(name),
            ..
        } => {
            set_namespace(path, &kube_yaml, &name)?;
        }
        // Default (no-args) command lists all contexts
        _ => {
            let output = list_all_contexts(&kube_yaml)?;
            for name in output {
                println!("{}", name);
            }
        }
    }

    Ok(())
}
