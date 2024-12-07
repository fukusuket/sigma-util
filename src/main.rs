use std::collections::HashMap;
use std::path::Path;
use std::{env, fs};
use yaml_rust2::yaml::Hash;
use yaml_rust2::{Yaml, YamlLoader};

fn list_yml_files<P: AsRef<Path>>(dir: P) -> Vec<String> {
    let mut yml_files = Vec::new();
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                yml_files.extend(list_yml_files(path));
            } else if let Some(ext) = path.extension() {
                if ext == "yml" {
                    yml_files.push(path.to_string_lossy().to_string());
                }
            }
        }
    }
    yml_files
}

fn extract_expand_keys_recursive(hash: &Hash, expand_keys: &mut HashMap<String, String>) {
    for (key, value) in hash {
        if let Some(key_str) = key.as_str() {
            if key_str.contains("|expand") {
                if let Some(value_str) = value.as_str() {
                    expand_keys.insert(key_str.to_string(), value_str.to_string());
                }
            }
            if let Some(sub_hash) = value.as_hash() {
                extract_expand_keys_recursive(sub_hash, expand_keys);
            }
        }
    }
}
fn extract_expand_keys(detection: &Hash) -> HashMap<String, String> {
    let mut expand_keys = HashMap::new();
    extract_expand_keys_recursive(detection, &mut expand_keys);
    expand_keys
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <directory>", args[0]);
        return;
    }
    let dir = &args[1];
    let yml_files = list_yml_files(dir);
    for file in yml_files {
        if let Ok(content) = fs::read_to_string(&file) {
            if let Ok(docs) = YamlLoader::load_from_str(&content) {
                if let Some(yaml) = docs.first() {
                    if let Some(detection) = yaml["detection"].as_hash() {
                        let expand_keys = extract_expand_keys(detection);
                        if !expand_keys.is_empty() {
                            println!("{:?}", expand_keys);
                        }
                    }
                }
            }
        }
    }
}

fn process_value(value: &Yaml, replacements: &HashMap<String, Vec<String>>) -> Yaml {
    match value {
        Yaml::String(val_str) => {
            let mut replaced_values = vec![];
            for (placeholder, replace_list) in replacements {
                if val_str.contains(placeholder) {
                    replaced_values.extend(
                        replace_list
                            .iter()
                            .cloned()
                            .map(|s| Yaml::String(val_str.replace(placeholder, &s))),
                    );
                }
            }
            if replaced_values.is_empty() {
                Yaml::String(val_str.clone())
            } else {
                Yaml::Array(replaced_values)
            }
        }
        Yaml::Array(array) => {
            let new_array: Vec<Yaml> = array
                .iter()
                .map(|item| process_yaml(item, replacements))
                .collect();
            Yaml::Array(new_array)
        }
        _ => process_yaml(value, replacements),
    }
}

fn process_yaml(yaml: &Yaml, replacements: &HashMap<String, Vec<String>>) -> Yaml {
    match yaml {
        Yaml::Hash(hash) => {
            let mut new_hash = Hash::new();
            for (key, value) in hash {
                if let Yaml::String(key_str) = key {
                    let new_key = key_str.replace("|expand", "");
                    let new_value = process_value(value, replacements);
                    new_hash.insert(Yaml::String(new_key), new_value);
                } else {
                    new_hash.insert(key.clone(), process_yaml(value, replacements));
                }
            }
            Yaml::Hash(new_hash)
        }
        Yaml::Array(array) => {
            let new_array: Vec<Yaml> = array
                .iter()
                .map(|item| process_yaml(item, replacements))
                .collect();
            Yaml::Array(new_array)
        }
        _ => yaml.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use yaml_rust2::YamlLoader;

    #[test]
    fn test_process_yaml() {
        let yaml_str = r#"
        key1: value1
        key2|expand: "test%placeholder%"
        key3:
          subkey1|expand: "%placeholder%"
          subkey2: subvalue2
        key4|expand:
          - item1
        "#;

        let docs = YamlLoader::load_from_str(yaml_str).unwrap();
        let mut replacements = HashMap::new();
        replacements.insert(
            "%placeholder%".to_string(),
            vec!["replace_value1".to_string(), "replace_value2".to_string()],
        );
        let processed_yaml = process_yaml(&docs[0], &replacements);

        let expected_yaml_str = r#"
        key1: value1
        key2: [testreplace_value1, testreplace_value2]
        key3:
          subkey1: [replace_value1, replace_value2]
          subkey2: subvalue2
        key4:
          - item1
        "#;

        let expected_docs = YamlLoader::load_from_str(expected_yaml_str).unwrap();
        assert_eq!(processed_yaml, expected_docs[0]);
    }
}
