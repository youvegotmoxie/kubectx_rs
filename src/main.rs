use std::env;
use std::fs::File;
use std::path::PathBuf;
use std::process::exit;
extern crate yaml_serde;
use yaml_serde::Value;

// TODO:
// Add error handling for supplying invalid an cluster value, eg supplied input context isn't in the kubeconfig's map of clusters
// The case where no input is supplied the fallback should be to list all contexts
// Backup the kubeconfig before modifying the file
// Make a copy of the original kubeconfig -> edit the copy -> move the copy to the original path -> delete
// Move non-main functions to a helper crate

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let input_args: Vec<String> = env::args().collect();

    let path = build_kubeconfig_path()?;
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

/// Builds the path to the kubeconfig file, either from the KUBECONFIG env var or the default path
fn build_kubeconfig_path() -> Result<PathBuf, Box<dyn std::error::Error>> {
    let mut kubeconfig = PathBuf::new();
    // Create the path to ~/.kube/config for reading the file
    // Read the KUBECONFIG env var if set and use that
    match env::var("KUBECONFIG") {
        Ok(kube_config_path) => kubeconfig.push(&kube_config_path),
        // If no KUBECONFIG env var is set then use the HOME env var and tack on the rest of the default path
        Err(env::VarError::NotPresent) => {
            kubeconfig.push(env::var("HOME")?);
            kubeconfig.push(".kube");
            kubeconfig.push("config");
        }
        Err(_) => {
            panic!("Unable to read kubectl config file");
        }
    }
    Ok(kubeconfig)
}

/// Reads the kubeconfig file and returns its contents as a yaml Value type
fn kubeconfig_to_yaml(kubeconfig: PathBuf) -> Result<Value, Box<dyn std::error::Error>> {
    let yaml_data: Value =
        yaml_serde::from_str(&std::io::read_to_string(File::open(&kubeconfig)?)?)?;

    Ok(yaml_data)
}

/// Takes kubeconfig as YAML Value from kubeconfig_to_yaml and returns the current context
fn get_current_kube_context(
    kube_context_yaml: &Value,
) -> Result<Value, Box<dyn std::error::Error>> {
    let current_context = kube_context_yaml["current-context"]
        .as_str()
        .ok_or("current-context key not found")?;

    Ok(current_context.into())
}

/// Takes kubeconfig as YAML Value from kubeconfig_to_yaml and returns the current context
fn list_all_contexts(kube_context_yaml: &Value) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let mut all_contexts = Vec::new();
    let contexts = kube_context_yaml["contexts"].as_sequence();

    if let Some(contexts) = contexts {
        for ctx in contexts {
            all_contexts.push(
                ctx["name"]
                    .as_str()
                    .ok_or("cluster name key not found")?
                    .to_string(),
            );
        }
    }
    Ok(all_contexts)
}

// if no context is supplied then print output of get_current_kube_context
fn set_context(
    kubeconfig: &PathBuf,
    kube_context_yaml: &Value,
    new_context: Value,
) -> Result<String, Box<dyn std::error::Error>> {
    let mut updated_yaml = kube_context_yaml.clone();
    let current_context = get_current_kube_context(&kube_context_yaml)?;

    if current_context != new_context {
        updated_yaml["current-context"] = new_context.into();
        println!(
            "Updated the current context to use {}",
            updated_yaml["current-context"]
                .as_str()
                .ok_or("current-context key not found")?
        );
        let yaml_data = yaml_serde::to_string(&updated_yaml)?;
        std::fs::write(&kubeconfig, &yaml_data)?;
        Ok(yaml_data)
    } else {
        println!(
            "The cluster context is already set to {}",
            new_context
                .as_str()
                .ok_or("Unable to get current-context key")?
        );
        exit(0)
    }
}
