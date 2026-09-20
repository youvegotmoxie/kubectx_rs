pub mod setup_kubeconfig {
    use std::env;
    use std::path::PathBuf;
    extern crate yaml_serde;
    use std::fs::File;
    use yaml_serde::Value;

    /// Builds the path to the kubeconfig file, either from the KUBECONFIG env var or the default path
    pub fn kubeconfig_path() -> Result<PathBuf, Box<dyn std::error::Error>> {
        let mut kubeconfig = PathBuf::new();
        // Create the path to ~/.kube/config for reading the file
        // Read the KUBECONFIG env var if set and use that
        match env::var("KUBECONFIG") {
            Ok(kube_config_path) => kubeconfig.push(kube_config_path),
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

    /// Creates a backup of the kubeconfig file `kubeconfigname.kubectx_rs.bak`
    pub fn backup_kubeconfig(kubeconfig_path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
        std::fs::copy(
            &kubeconfig_path,
            &kubeconfig_path.with_added_extension("kubectx_rs.bak"),
        )?;
        Ok(())
    }

    /// Reads the kubeconfig file and returns its contents as a yaml Value type
    pub fn kubeconfig_to_yaml(kubeconfig: PathBuf) -> Result<Value, Box<dyn std::error::Error>> {
        let yaml_data: Value =
            yaml_serde::from_str(&std::io::read_to_string(File::open(kubeconfig)?)?)?;

        Ok(yaml_data)
    }
}

pub mod list_get_set_contexts {
    extern crate yaml_serde;
    use crate::cli::kubeconfig::setup_kubeconfig::backup_kubeconfig;
    use std::path::PathBuf;
    use yaml_serde::Value;

    /// Takes kubeconfig as YAML Value from kubeconfig_to_yaml and returns the current context
    pub fn get_current_context(
        kube_context_yaml: &Value,
    ) -> Result<Value, Box<dyn std::error::Error>> {
        let current_context = kube_context_yaml["current-context"]
            .as_str()
            .ok_or("current-context key not found")?;

        Ok(current_context.into())
    }

    /// Takes kubeconfig as YAML Value and returns all context (cluster) names
    pub fn list_all_contexts(
        kube_context_yaml: &Value,
    ) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let mut all_contexts = Vec::new();
        let contexts = kube_context_yaml["contexts"].as_sequence();

        // Iterate through the contexts and extract the cluster names
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

    /// Checks that a user's chosen cluster exists in the kubeconfig
    /// Takes a list of clusters and the user's chosen cluster name and
    /// returns the cluster name if found
    pub fn validate_context(
        kube_context_yaml: &Value,
        new_context: Value,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let all_contexts = list_all_contexts(kube_context_yaml)?;
        let incoming_context_name: String = String::from(new_context.as_str().unwrap());

        if all_contexts
            .iter()
            .any(|name| name == &incoming_context_name)
        {
            Ok(incoming_context_name)
        } else {
            Err("Cluster not found in config".into())
        }
    }

    /// Sets the current-context in the kubeconfig, backing up the file before writing
    pub fn set_context(
        kubeconfig: PathBuf,
        kube_context_yaml: &Value,
        new_context: Value,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let mut updated_yaml = kube_context_yaml.clone();
        let current_context = get_current_context(kube_context_yaml)?;

        validate_context(&updated_yaml, new_context.clone())?;

        if current_context != new_context {
            updated_yaml["current-context"] = new_context.into();
            println!(
                "Updated the current context to use {}",
                updated_yaml["current-context"]
                    .as_str()
                    .ok_or("current-context key not found")?
            );
            let yaml_data = yaml_serde::to_string(&updated_yaml)?;
            backup_kubeconfig(&kubeconfig.to_path_buf())?;

            // // Copy the kubeconfig to a temp file, edit that, then copy it back
            let temp_ext = String::from("kubectx_rs.staged");
            std::fs::copy(&kubeconfig, &kubeconfig.with_added_extension(&temp_ext))?;
            std::fs::write(kubeconfig.with_added_extension(&temp_ext), &yaml_data)?;
            std::fs::rename(&kubeconfig.with_added_extension(&temp_ext), &kubeconfig)?;

            Ok(yaml_data)
        } else {
            println!(
                "The cluster context is already set to {}",
                new_context.as_str().unwrap()
            );
            Ok(new_context
                .as_str()
                .ok_or("Unable to get current-context key")?
                .to_string())
        }
    }
}

pub mod delete_rename_context {
    extern crate yaml_serde;
    use crate::cli::kubeconfig::{list_get_set_contexts::validate_context, setup_kubeconfig::*};
    use std::path::PathBuf;
    use yaml_serde::Value;

    /// Deletes a context from the kubeconfig
    pub fn delete_context(
        kubeconfig: PathBuf,
        cluster_name: Value,
        kube_config_yaml: &Value,
    ) -> Result<Value, Box<dyn std::error::Error>> {
        // Ensure the cluster name exists as a context entry
        let context = validate_context(kube_config_yaml, cluster_name.clone())?;

        let mut deleted_yaml = kube_config_yaml.clone();

        // Read all the contexts into a Vec<Value>
        let contexts = deleted_yaml["contexts"]
            .as_sequence_mut()
            .ok_or("Contexts mapping not found in KUBECONFIG")?;

        // If the supplied cluster name matches keep it, otherwise remove it from the Vec<Value>
        contexts.retain(|ctx| match ctx["name"].as_str() {
            Some(name) => name != context,
            None => true,
        });

        // If the deleted context is the current context set current-context to an empty value
        if deleted_yaml["current-context"].as_str() == Some(context.as_str()) {
            deleted_yaml["current-context"] = Value::String(String::new());
        }

        let yaml_data = yaml_serde::to_string(&deleted_yaml)?;

        // Backup the kubeconfig before modifying it
        backup_kubeconfig(&kubeconfig.to_path_buf())?;

        std::fs::write(kubeconfig, yaml_data)?;
        println!("Deleted context {}", context);
        Ok(deleted_yaml)
    }
}
