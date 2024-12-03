use std::collections::HashMap;
use std::path::Path;
use std::{env, fs};
use yaml_rust2::yaml::Hash;
use yaml_rust2::YamlLoader;

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
