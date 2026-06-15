use std::env;
use std::process::exit;
extern crate yaml_serde;
use yaml_serde::Value;
mod cli;
use cli::kubeconfig::{build_config_path::get_path, list_get_set_contexts::*};

// TODO:
// Add error handling for supplying invalid an cluster value, eg supplied input context isn't in the kubeconfig's map of clusters
// Backup the kubeconfig before modifying the file
// Make a copy of the original kubeconfig -> edit the copy -> move the copy to the original path -> delete
// Ability to delete a context
// Ability to unset the current context. this means changing how we handle the empty current-context key
// Isolated shell with $KUBECONFIG set
// `cd -` like ability to switch back to the previously set context -> ties into backup files
// Ability to set the namespace for a given cluster context

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let input_args: Vec<String> = env::args().collect();

    let path = get_path()?;
    let path_buf = &path;
    let kube_yaml = kubeconfig_to_yaml(path_buf.to_path_buf())?;

    if let Some(new_context) = input_args.get(1) {
        set_context(
            &path_buf,
            &kube_yaml,
            Value::String(String::from(new_context)),
        )?;
    } else {
        let all_cluster_names = list_all_contexts(&kube_yaml)?;
        if all_cluster_names.is_empty() {
            eprintln!("No clusters found. Kubeconfig file is either empty or malformed");
            exit(0);
        }
        for name in all_cluster_names {
            println!("{}", name);
        }
    }
    Ok(())
}
