use std::env;
use std::fs::File;
use std::path::PathBuf;
extern crate yaml_serde;
use yaml_serde::Value;

fn main() {
    let current_context =
        ParseKubeConfig::get_current_context(KubeConfig.read_kubeconfig().unwrap()).unwrap();
    println!(
        "current cluster context: {}",
        current_context.cluster.as_str().unwrap()
    );

    let all = ParseKubeConfig::get_all_clusters(KubeConfig.read_kubeconfig().unwrap()).unwrap();
    println!("all clusters:\n{}", all.join("\n"));
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
trait KubeConfigTrait {
    fn read_kubeconfig(&self) -> Result<String, Box<dyn std::error::Error>>;
}

struct KubeConfig;

#[allow(dead_code)]
trait KubeContextTrait {
    fn get_current_context(
        kube_config: String,
    ) -> Result<ParseKubeConfig, Box<dyn std::error::Error>>;
    fn get_all_clusters(kube_config: String) -> Result<Vec<String>, Box<dyn std::error::Error>>;
    fn set_context(
        kube_config: String,
        new_context: String,
    ) -> Result<Value, Box<dyn std::error::Error>>;
}

struct ParseKubeConfig {
    cluster: Value,
}

impl KubeConfigTrait for KubeConfig {
    /// Read the kubectl config from either the KUBECONFIG env var
    ///
    /// Fall back to reading from the well known ~/.kube/config path if that value isn't set
    fn read_kubeconfig(&self) -> Result<String, Box<dyn std::error::Error>> {
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

        // Read the kubeconfig and return the contents of the file
        let config_path: PathBuf = kubeconfig;
        let data = std::io::read_to_string(File::open(&config_path)?)?;

        Ok(data)
    }
}

impl KubeContextTrait for ParseKubeConfig {
    /// Read kubeconfig and get the YAML key for current-context and return that value
    fn get_current_context(
        kube_config: String,
    ) -> Result<ParseKubeConfig, Box<dyn std::error::Error>> {
        let config = yaml_serde::from_str::<Value>(&kube_config)?;
        let localhost_default_context =
            yaml_serde::from_str::<Value>("current-context: localhost")?;

        let cluster: Value = config
            .get("current-context")
            .unwrap_or(&localhost_default_context)
            .clone();

        Ok(ParseKubeConfig { cluster })
    }

    /// Read kubeconfig and get the YAML key for contexts and return a vector of all cluster names
    fn get_all_clusters(kube_config: String) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let config = yaml_serde::from_str::<Value>(&kube_config)?;
        let mut cluster_list = Vec::new();

        let all_cluster_contexts = config["contexts"].as_sequence().unwrap();

        for entry in all_cluster_contexts.iter() {
            let cluster_name = entry["name"]
                .as_str()
                .ok_or("Missing entry in context")?
                .to_string();
            cluster_list.push(cluster_name);
        }

        Ok(cluster_list)
    }

    fn set_context(
        _kube_config: String,
        _new_context: String,
    ) -> Result<Value, Box<dyn std::error::Error>> {
        unimplemented!()
    }
}
