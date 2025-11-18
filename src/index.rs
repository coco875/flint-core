use crate::loader::TestLoader;
use crate::test_spec::TestSpec;
use anyhow::anyhow;
use serde::{Deserialize, Serialize};
use serde_json::to_string_pretty;
use std::collections::{BTreeMap, hash_map::DefaultHasher};
use std::env;
use std::fs::{File, OpenOptions, create_dir_all};
use std::hash::{Hash, Hasher};
use std::io::{BufReader, Write};
use std::path::{Path, PathBuf};

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq, Eq)]
pub struct Index {
    pub hash: u64,
    pub index: BTreeMap<String, Vec<String>>,
}

impl Index {
    pub fn get_index_name() -> String {
        env::var("INDEX_NAME").unwrap_or(".cache/index.json".to_string())
    }

    pub fn get_default_tag() -> String {
        env::var("DEFAULT_TAG").unwrap_or("default".to_string())
    }
    pub fn get_test_path() -> String {
        env::var("TEST_PATH").unwrap_or("./test".to_string())
    }

    /// Creates a new Index
    ///
    /// # Arguments
    ///
    /// * `hash`: The hash of the all usable files
    /// * `map`: The created index as a BTreeMap
    ///
    /// returns: Index
    ///
    pub fn new(hash: u64, map: &BTreeMap<String, Vec<String>>) -> Self {
        Index {
            hash,
            index: map.clone(),
        }
    }

    /// Creates an empty Index
    pub fn empty() -> Self {
        Self {
            hash: 0,
            index: BTreeMap::new(),
        }
    }

    /// Creates an index other all files
    ///
    /// returns: Result<Index, Error>
    ///
    /// # Environment Variables
    ///
    /// * `TEST_PATH` - Base directory for tests (default: "./test")
    /// * `INDEX_NAME` - Path to the index cache file (default: ".cache/index.json")
    /// * `DEFAULT_TAG` - Tag assigned to tests with no tags (default: "default")
    pub fn generate_index() -> anyhow::Result<Index> {
        let s = Self::get_test_path();
        let path = Path::new(&s);
        if !path.is_dir() {
            return Err(anyhow!(format!(
                "The path is not a directory: {}",
                path.to_str().unwrap()
            )));
        }
        let all_files = TestLoader::collect_all_test_files(path)?;
        let mut index_map: BTreeMap<String, Vec<String>> = BTreeMap::new();
        let hash = get_hash(&all_files);
        for i in all_files {
            let file = File::open(i.clone())?;
            let reader = BufReader::new(file);
            let test: TestSpec = serde_json::from_reader(reader)?;
            // Add test to all tags
            for tag in &test.tags {
                index_map
                    .entry(tag.clone())
                    .or_default()
                    .push(i.to_string_lossy().parse()?);
            }

            // add test to default tag if no tag specified
            if test.tags.is_empty() {
                index_map
                    .entry(Index::get_default_tag().to_string())
                    .or_default()
                    .push(i.to_string_lossy().parse()?);
            }
        }

        // write Index
        let index = Index::new(hash, &index_map);
        let index_string = to_string_pretty(&index)?;
        let name = Index::get_index_name();
        let path = Path::new(&name);

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

    /// Validates if the hash is the same or not
    ///
    /// # Arguments
    ///
    /// * `generated_hash`: the new generated hash
    ///
    /// returns: bool
    ///
    pub fn validate(&self, generated_hash: u64) -> bool {
        self.hash == generated_hash
    }

    /// Searches through the index all tests with specific tags
    /// Also validates if the index is correct or not and recreates the index if not.
    ///
    /// # Arguments
    ///
    /// * `scope`: all tags
    ///
    /// returns: Result<Vec<String, Global>, Error>
    ///
    /// # Environment Variables
    ///
    /// * `TEST_PATH` - Base directory for tests (default: "./test")
    /// * `INDEX_NAME` - Path to the index cache file (default: ".cache/index.json")
    /// * `DEFAULT_TAG` - Tag assigned to tests with no tags (default: "default")
    pub fn get_test_paths_from_scopes(scope: &[String]) -> anyhow::Result<Vec<PathBuf>> {
        let mut index: Index = Index::empty();
        let mut valid_index: bool = false;
        if Path::new(&Index::get_index_name()).exists() {
            let file = File::open(Index::get_index_name())?;
            let reader = BufReader::new(file);

            // get index
            index = serde_json::from_reader(reader)?;

            // verify the index
            if let Ok(vec) = TestLoader::collect_all_test_files(Path::new(&Index::get_test_path()))
                && index.validate(get_hash(&vec))
            {
                valid_index = true;
            }
        }

        // if not valid recreate index
        if !valid_index {
            // Index does not exists or isn't valid, so need to build the index first
            index = Index::generate_index()?;
        }
        let mut test_paths = vec![];
        for (k, v) in &index.index {
            if scope.contains(k) {
                for path in v {
                    // would load the same test more than once if the test will be in more scopes
                    if !test_paths.contains(&PathBuf::from(path)) {
                        test_paths.push(PathBuf::from(path));
                    }
                }
            }
        }
        Ok(test_paths)
    }
}

/// Creates a hash of a vec of PathBuf
///
/// # Arguments
///
/// * `vec`: the fec
///
/// returns: u64
///
fn get_hash(vec: &Vec<PathBuf>) -> u64 {
    let mut hasher = DefaultHasher::new();
    vec.hash(&mut hasher);
    hasher.finish()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_file(dir: &Path, name: &str) -> PathBuf {
        let path = dir.join(name);
        fs::write(&path, "{}").unwrap();
        path
    }

    #[test]
    fn test_collect_single_file() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = create_test_file(temp_dir.path(), "test.json");

        let files = TestLoader::collect_test_files(&test_file, false).unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0], test_file);
    }

    #[test]
    fn test_collect_non_json_file() {
        let temp_dir = TempDir::new().unwrap();
        let txt_file = temp_dir.path().join("test.txt");
        fs::write(&txt_file, "test").unwrap();

        let files = TestLoader::collect_test_files(&txt_file, false).unwrap();
        assert_eq!(files.len(), 0);
    }

    #[test]
    fn test_collect_recursive() {
        let temp_dir = TempDir::new().unwrap();

        // Create files in root
        create_test_file(temp_dir.path(), "test1.json");

        // Create nested subdirectories with files
        let sub_dir1 = temp_dir.path().join("subdir1");
        fs::create_dir(&sub_dir1).unwrap();
        create_test_file(&sub_dir1, "test2.json");

        let sub_dir2 = sub_dir1.join("nested");
        fs::create_dir(&sub_dir2).unwrap();
        create_test_file(&sub_dir2, "test3.json");

        let files = TestLoader::collect_test_files(temp_dir.path(), true).unwrap();

        // Should find all 3 files
        assert_eq!(files.len(), 3);
    }
}
