pub mod setup_kubeconfig {
    use std::env;
    use std::path::PathBuf;
    extern crate yaml_serde;
    use std::fs::File;
    use yaml_serde::Value;

    /// Builds the path to the kubeconfig file.
    ///
    /// Resolution order:
    /// 1. The `KUBECONFIG` environment variable, if set (used verbatim)
    /// 2. Otherwise, the default location `$HOME/.kube/config`
    ///
    /// # Returns
    /// The resolved kubeconfig file path, or an error if the `HOME` environment
    /// variable is not set.
    ///
    /// # Panics
    /// Panics if the `KUBECONFIG` environment variable contains invalid Unicode.
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

    /// Creates a backup of the kubeconfig file as `<kubeconfig>.kubectx_rs.bak`.
    ///
    /// # Arguments
    /// * `kubeconfig_path` - Path to the existing kubeconfig file to back up
    ///
    /// # Errors
    /// Returns an error if the file cannot be copied (e.g. it does not exist
    /// or is not readable).
    pub fn backup_kubeconfig(kubeconfig_path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
        std::fs::copy(
            kubeconfig_path,
            kubeconfig_path.with_added_extension("kubectx_rs.bak"),
        )?;
        Ok(())
    }

    /// Reads the kubeconfig file and parses its contents into a YAML `Value`.
    ///
    /// # Arguments
    /// * `kubeconfig` - Path to the kubeconfig file to read (consumed by this call)
    ///
    /// # Returns
    /// The parsed YAML document as a `Value`.
    ///
    /// # Errors
    /// Returns an error if the file cannot be opened, read, or parsed as YAML.
    pub fn kubeconfig_to_yaml(kubeconfig: PathBuf) -> Result<Value, Box<dyn std::error::Error>> {
        let yaml_data: Value =
            yaml_serde::from_str(&std::io::read_to_string(File::open(kubeconfig)?)?)?;

        Ok(yaml_data)
    }
}

pub mod mutate_contexts {
    extern crate yaml_serde;
    use crate::cli::kubeconfig::setup_kubeconfig::backup_kubeconfig;
    use std::path::PathBuf;
    use yaml_serde::Value;

    /// Atomically replaces the kubeconfig file with new contents via a staged temp file.
    ///
    /// Copies the kubeconfig to `<kubeconfig>.kubectx_rs.staged`, writes the new
    /// data into the staged file, then renames it over the original.
    ///
    /// # Arguments
    /// * `kubeconfig` - Path to the kubeconfig file to update
    /// * `data` - The complete new YAML contents to write
    ///
    /// # Errors
    /// Returns an error if any of the copy, write, or rename steps fail.
    pub fn copy_and_edit(
        kubeconfig: &PathBuf,
        data: &String,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let temp_ext = String::from("kubectx_rs.staged");
        std::fs::copy(kubeconfig, kubeconfig.with_added_extension(&temp_ext))?;
        std::fs::write(kubeconfig.with_added_extension(&temp_ext), data)?;
        std::fs::rename(kubeconfig.with_added_extension(&temp_ext), kubeconfig)?;
        Ok(())
    }

    /// Returns the value of the `current-context` key from the kubeconfig YAML.
    ///
    /// # Arguments
    /// * `kube_context_yaml` - The kubeconfig document, as returned by `kubeconfig_to_yaml()`
    ///
    /// # Returns
    /// A `Value` holding the current context name.
    ///
    /// # Errors
    /// Returns an error if the `current-context` key is missing or is not a string.
    pub fn get_current_context(
        kube_context_yaml: &Value,
    ) -> Result<Value, Box<dyn std::error::Error>> {
        let current_context = kube_context_yaml["current-context"]
            .as_str()
            .ok_or("current-context key not found")?;

        Ok(current_context.into())
    }

    /// Returns the names of every context (cluster) defined in the kubeconfig YAML.
    ///
    /// # Arguments
    /// * `kube_context_yaml` - The kubeconfig document, as returned by `kubeconfig_to_yaml()`
    ///
    /// # Returns
    /// A `Vec<String>` of context names. Empty if the `contexts` key is absent.
    ///
    /// # Errors
    /// Returns an error if any context entry is missing its `name` key.
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

    /// Checks that the chosen context exists in the kubeconfig.
    ///
    /// # Arguments
    /// * `kube_context_yaml` - The kubeconfig document, as returned by `kubeconfig_to_yaml()`
    /// * `new_context` - The context name the user wants to switch to (as a string `Value`)
    ///
    /// # Returns
    /// The validated context name as a `String`.
    ///
    /// # Errors
    /// Returns an error if the context is not present in the kubeconfig,
    /// or is not a string.
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

    /// Sets `current-context` in the kubeconfig to `new_context`, backing up the
    /// file before writing. No write occurs if `new_context` is already the
    /// current context.
    ///
    /// # Arguments
    /// * `kubeconfig` - Path to the kubeconfig file to update (consumed by this call)
    /// * `kube_context_yaml` - The kubeconfig document, as returned by `kubeconfig_to_yaml()`
    /// * `new_context` - The context name to switch to (as a string `Value`)
    ///
    /// # Returns
    /// The updated YAML document serialized as a `String`.
    ///
    /// # Errors
    /// Returns an error if the context does not exist, the YAML cannot be
    /// serialized, or the backup/write steps fail.
    pub fn set_context(
        kubeconfig: PathBuf,
        kube_context_yaml: &Value,
        new_context: Value,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let mut updated_yaml = kube_context_yaml.clone();
        let current_context = get_current_context(kube_context_yaml)?;

        validate_context(&updated_yaml, new_context.clone())?;

        if current_context != new_context {
            updated_yaml["current-context"] = new_context;
            println!(
                "Updated the current context to use {}",
                updated_yaml["current-context"]
                    .as_str()
                    .ok_or("current-context key not found")?
            );

            let yaml_data = yaml_serde::to_string(&updated_yaml)?;
            backup_kubeconfig(&kubeconfig.to_path_buf())?;
            copy_and_edit(&kubeconfig, &yaml_data)?;

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

    /// Deletes the given context from the kubeconfig, backing up the file
    /// before writing.
    ///
    /// If the deleted context is also the current context, `current-context`
    /// is reset to an empty string.
    ///
    /// # Arguments
    /// * `kubeconfig` - Path to the kubeconfig file to update (consumed by this call)
    /// * `cluster_name` - The context name to delete (as a string `Value`)
    /// * `kube_config_yaml` - The kubeconfig document, as returned by `kubeconfig_to_yaml()`
    ///
    /// # Returns
    /// The updated YAML document after the context has been removed.
    ///
    /// # Errors
    /// Returns an error if the context does not exist, the YAML cannot be
    /// serialized, or the backup/write steps fail.
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
        copy_and_edit(&kubeconfig, &yaml_data)?;

        println!("Deleted context {}", context);
        Ok(deleted_yaml)
    }
}
