use std::env;
use std::fs::File;
use std::path::PathBuf;
extern crate yaml_serde;
use yaml_serde::Value;

// TODO: return an actual result and use ? instead of .unwrap()
fn main() {
    // ~/.kube/config as a yaml Value type
    let kube_yaml = kubeconfig_to_yaml().unwrap();
    let current_ctx = get_current_kube_context(&kube_yaml).unwrap();
    let all_contexts = list_all_contexts(&kube_yaml).unwrap();

    println!("current context: {}", current_ctx);

    for ctx in all_contexts.iter() {
        println!("{}", ctx);
    }
}

/// Reads the kubeconfig file and returns its contents as a yaml Value type
fn kubeconfig_to_yaml() -> Result<Value, Box<dyn std::error::Error>> {
    let mut kubeconfig = PathBuf::new();
    // Create the path to ~/.kube/config for reading the file
    // Read the KUBECONFIG env var if set and use that
    match env::var("KUBECONFIG") {
        Ok(kube_config_path) => kubeconfig.push(&kube_config_path),
        // If no KUBECONFIG env var is set then use the HOME env var and tack on the rest of the default path
        Err(env::VarError::NotPresent) => {
            kubeconfig.push(env::var("HOME").unwrap_or_default());
            kubeconfig.push(".kube");
            kubeconfig.push("config");
        }
        Err(_) => {
            panic!("Unable to read kubectl config file");
        }
    }

    let yaml_data: Value =
        yaml_serde::from_str(&std::io::read_to_string(File::open(&kubeconfig)?)?)?;

    Ok(yaml_data)
}

/// Takes kubeconfig as YAML Value from kubeconfig_to_yaml and returns the current context
fn get_current_kube_context(
    kube_context_yaml: &Value,
) -> Result<String, Box<dyn std::error::Error>> {
    let current_context = kube_context_yaml["current-context"]
        .as_str()
        .ok_or("current-context key not found")?
        .to_string();

    Ok(current_context)
}

/// Takes kubeconfig as YAML Value from kubeconfig_to_yaml and returns the current context
fn list_all_contexts(kube_context_yaml: &Value) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let mut all_contexts = Vec::new();
    let contexts = kube_context_yaml["contexts"].as_sequence();

    if let Some(contexts) = contexts {
        for context in contexts {
            all_contexts.push(
                context["name"]
                    .as_str()
                    .ok_or("name key not found")?
                    .to_string(),
            );
        }
    }
    Ok(all_contexts)
}
// what are we doing?
// function 1
// read KUBECONFIG
// parse to get contexts
//
// function 2
// list contexts to user
// detect fzf and if fzf is available output to that (library?)
//
// have tab completion, if kubectx-rs is invoked with no arguments and no fzf
// generate tab completion based on the contents of the context data structure
//
// function 3
// set context to one chosen by user
// this should take a user supplied cluster context; use what's selected from either tab completion, fzf input or stdin at runtime
//
