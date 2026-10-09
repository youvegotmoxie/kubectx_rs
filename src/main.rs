use clap::Parser;

mod cli;
use cli::args::Cli;
use cli::kubeconfig::mutate_contexts::*;
use cli::kubeconfig::setup_kubeconfig::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = cli::args::Cli::parse();
    let mut path = kubeconfig_path()?;
    let mut kube_yaml = kubeconfig_to_yaml(path.to_path_buf())?;

    // If -c/--config is set then overridde the KUBECONFIG env var
    if let Some(kube_config_path) = cli.config.as_deref() {
        path = kube_config_path.into();
        kube_yaml = kubeconfig_to_yaml(path.to_path_buf())?;
    }

    match cli {
        // If -d/--delte is set then delete the cluster from KUBECONFIG
        // This only removes the entry from the contexts map
        Cli {
            delete: Some(name), ..
        } => {
            delete_context(path, name, &kube_yaml)?;
        }
        // If a cluster name is provided and passes validation, set this to current-context
        Cli {
            set: Some(name), ..
        } => {
            set_context(path, &kube_yaml, name)?;
        }
        // If -n/--namespace is set, set the namespace for the current context
        Cli {
            namespace: Some(name),
            ..
        } => {
            set_namespace(path, &kube_yaml, name)?;
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
