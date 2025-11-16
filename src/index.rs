use crate::test_spec::TestSpec;
use serde::{Deserialize, Serialize};
use serde_json::to_string_pretty;
use std::collections::{BTreeMap, hash_map::DefaultHasher};
use std::fs;
use std::fs::{File, OpenOptions, create_dir_all};
use std::hash::{Hash, Hasher};
use std::io::{BufReader, Write};
use std::path::Path;
use anyhow::anyhow;

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq, Eq)]
pub struct Index {
    pub hash: u64,
    pub index: BTreeMap<String, Vec<String>>,
}

impl Index {
    pub const INDEX_NAME: &'static str = ".cache/index.json";
    pub const DEFAULT_TAG: &'static str = "default";
    const TEST_PATH: &'static str = "./test";

    pub fn new(hash: u64, map: &BTreeMap<String, Vec<String>>) -> Self {
        Index {
            hash,
            index: map.clone(),
        }
    }

    pub fn empty() -> Self {
        Self {
            hash: 0,
            index: BTreeMap::new(),
        }
    }

    pub fn generate_index(path: &Path) -> anyhow::Result<Index> {
        if !path.is_dir() {
            return Err(anyhow!(format!("The path is not a directory: {}", path.to_str().unwrap())));
        }
        let all_files = get_all_test_files(path);
        let mut index_map: BTreeMap<String, Vec<String>> = BTreeMap::new();
        let hash = get_hash(&all_files);
        for i in all_files {
            let file = File::open(i.clone())?;
            let reader = BufReader::new(file);
            let test: TestSpec = serde_json::from_reader(reader)?;
            for tag in &test.tags {
                if let Some(vec) = index_map.get_mut(tag) {
                    vec.push(i.clone())
                } else {
                    index_map.insert(tag.clone(), vec![i.clone()]);
                }
            }
            for tag in &test.tags {
                index_map.entry(tag.clone()).or_default().push(i.clone());
            }
            if test.tags.is_empty() {
                index_map.entry(Index::DEFAULT_TAG.to_string()).or_default().push(i.clone());
            }
        }
        let index = Index::new(hash, &index_map);
        let index_string = to_string_pretty(&index)?;
        let path = Path::new(Index::INDEX_NAME);

        // create parent directories if they don’t exist
        if let Some(parent) = path.parent() {
            create_dir_all(parent)?;
        }

        // now create/write the file
        let mut file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true) // optional, overwrite existing file
            .open(path)?;
        file.write_all(index_string.as_bytes())?;
        Ok(index)
    }

    pub fn validate(&self, generated_hash: u64) -> bool {
        self.hash == generated_hash
    }

    pub fn load_tagged_tests_paths(scope: &[String]) -> anyhow::Result<Vec<String>> {
        let mut index: Index = Index::empty();
        let mut valid_index: bool = false;
        if Path::new(Index::INDEX_NAME).exists() {
            let file = File::open(Index::INDEX_NAME)?;
            let reader = BufReader::new(file);
            match serde_json::from_reader(reader) {
                Ok(dex) => {
                    index = dex;
                    if let Ok(vec) = Index::get_all_tests_paths()
                        && index.validate(get_hash(&vec))
                    {
                        valid_index = true;
                    }
                }
                Err(e) => {
                    println!("{:?}", e);
                }
            }
        }
        if !valid_index {
            // Index does not exists or isn't valid, so need to build the index first
            index = Index::generate_index(Path::new(Index::TEST_PATH))?;
        }
        let mut test_paths = vec![];
        for (k, v) in &index.index {
            if scope.contains(k) {
                for path in v {
                    println!("added test to current run: '{}' ", path);
                    test_paths.push(path.clone());
                }
            }
        }
        Ok(test_paths)
    }

    pub fn get_all_tests_paths() -> anyhow::Result<Vec<String>> {
        Ok(get_all_test_files(Path::new(Index::TEST_PATH)))
    }
}

fn get_all_test_files(start_dir: &Path) -> Vec<String> {
    let mut dirs = vec![start_dir.to_path_buf()];
    let mut index: Vec<String> = Vec::new();

    while let Some(dir) = dirs.pop() {
        if let Ok(entries) = fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    dirs.push(path);
                } else if let Some(ext) = path.extension()
                    && ext == "json"
                {
                    index.push(path.to_string_lossy().parse().unwrap());
                }
            }
        }
    }
    index
}

fn get_hash(vec: &Vec<String>) -> u64 {
    let mut hasher = DefaultHasher::new();
    vec.hash(&mut hasher);
    hasher.finish()
}
