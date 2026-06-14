use std::env;
use std::fs::File;
use std::path::PathBuf;
use std::process::exit;
extern crate yaml_serde;
use yaml_serde::Value;

// TODO: return an actual result and use ? instead of .unwrap()
fn main() {
    let input_context: Vec<String> = env::args().collect();

    let kubeconfig_path = build_kubeconfig_path().unwrap();
    let kube_yaml = kubeconfig_to_yaml(kubeconfig_path.clone()).unwrap();

    let new_context = Value::String(String::from(&input_context[1]));
    set_context(&kubeconfig_path, &kube_yaml, new_context).unwrap();
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
#[allow(dead_code)]
fn list_all_contexts(kube_context_yaml: &Value) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let mut all_contexts = Vec::new();
    let contexts = kube_context_yaml["contexts"].as_sequence();

    if let Some(contexts) = contexts {
        for ctx in contexts {
            all_contexts.push(
                ctx["name"]
                    .as_str()
                    .ok_or("name key not found")?
                    .to_string(),
            );
        }
    }
    Ok(all_contexts)
}

fn set_context(
    kubeconfig: &PathBuf,
    kube_context_yaml: &Value,
    new_context: Value,
) -> Result<String, Box<dyn std::error::Error>> {
    let mut updated_yaml = kube_context_yaml.clone();
    let current_context = get_current_kube_context(kube_context_yaml)?;

    if current_context != new_context {
        updated_yaml["current-context"] = new_context.into();
        println!(
            "context set to {}",
            updated_yaml["current-context"].as_str().unwrap()
        );
    } else {
        println!(
            "context is already set to {}",
            new_context.as_str().unwrap()
        );
        exit(0)
    }

    let yaml_data = yaml_serde::to_string(&updated_yaml)?;

    {
        std::fs::write(&kubeconfig, &yaml_data)?;
    }

    Ok(yaml_data)
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
