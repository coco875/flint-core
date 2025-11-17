use crate::index::Index;
use anyhow::Result;
use std::path::{Path, PathBuf};

/// Test file loader for discovering test files in the filesystem
pub struct TestLoader;

impl TestLoader {
    /// Collect test files from a path (file or directory)
    ///
    /// # Arguments
    ///
    /// * `path` - Path to a single test file or directory containing tests
    /// * `recursive` - If true and path is a directory, recursively search subdirectories
    ///
    /// # Returns
    ///
    /// A sorted vector of PathBuf pointing to test JSON files
    pub fn collect_test_files(path: &Path, recursive: bool) -> Result<Vec<PathBuf>> {
        let mut test_files = Vec::new();

        if path.is_file() {
            // Single file - add if it's a JSON file
            if Self::is_json_file(path) {
                test_files.push(path.to_path_buf());
            }
        } else if path.is_dir() {
            // Directory - collect JSON files
            if recursive {
                Self::collect_recursive(path, &mut test_files)?;
            } else {
                Self::collect_non_recursive(path, &mut test_files)?;
            }
        }

        // Sort for consistent ordering
        test_files.sort();
        Ok(test_files)
    }

    /// Collect all test files recursively from a directory
    ///
    /// This uses an iterative stack-based approach for better performance
    /// with large directory trees.
    pub fn collect_all_test_files(root_path: &Path) -> Result<Vec<PathBuf>> {
        let mut test_files = Vec::new();
        Self::collect_recursive(root_path, &mut test_files)?;
        test_files.sort();
        Ok(test_files)
    }

    /// Collect test files by tags using the index system
    ///
    /// This method uses the Index to efficiently load tests that match any of the provided tags.
    /// The Index automatically manages caching and regeneration based on file changes.
    ///
    /// # Arguments
    ///
    /// * `tags` - Slice of tag names to filter tests by
    ///
    /// # Returns
    ///
    /// A vector of PathBuf pointing to test JSON files that have at least one of the specified tags
    ///
    /// # Environment Variables
    ///
    /// * `TEST_PATH` - Base directory for tests (default: "./test")
    /// * `INDEX_NAME` - Path to the index cache file (default: ".cache/index.json")
    /// * `DEFAULT_TAG` - Tag assigned to tests with no tags (default: "default")
    pub fn collect_by_tags(tags: &[String]) -> Result<Vec<PathBuf>> {
        let paths = Index::load_tagged_tests_paths(tags)?;
        Ok(paths.into_iter().map(PathBuf::from).collect())
    }

    /// Collect all tests using the index system
    ///
    /// This is more efficient than collect_all_test_files when the index is already built,
    /// as it uses cached file discovery.
    pub fn collect_all_indexed() -> Result<Vec<PathBuf>> {
        let paths = Index::get_all_tests_paths()?;
        Ok(paths.into_iter().map(PathBuf::from).collect())
    }

    /// Check if a file is a JSON file by extension
    fn is_json_file(path: &Path) -> bool {
        path.extension()
            .and_then(|s| s.to_str())
            .map(|ext| ext.eq_ignore_ascii_case("json"))
            .unwrap_or(false)
    }

    /// Collect JSON files from immediate directory only (non-recursive)
    fn collect_non_recursive(dir: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() && Self::is_json_file(&path) {
                files.push(path);
            }
        }
        Ok(())
    }

    /// Collect JSON files recursively using stack-based iteration
    fn collect_recursive(root: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
        let mut stack = vec![root.to_path_buf()];

        while let Some(current_dir) = stack.pop() {
            for entry in std::fs::read_dir(&current_dir)? {
                let entry = entry?;
                let path = entry.path();

                if path.is_dir() {
                    stack.push(path);
                } else if path.is_file() && Self::is_json_file(&path) {
                    files.push(path);
                }
            }
        }

        Ok(())
    }
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
    fn test_collect_non_recursive() {
        let temp_dir = TempDir::new().unwrap();

        // Create files in root
        create_test_file(temp_dir.path(), "test1.json");
        create_test_file(temp_dir.path(), "test2.json");

        // Create subdirectory with file
        let sub_dir = temp_dir.path().join("subdir");
        fs::create_dir(&sub_dir).unwrap();
        create_test_file(&sub_dir, "test3.json");

        let files = TestLoader::collect_test_files(temp_dir.path(), false).unwrap();

        // Should only find 2 files (not the one in subdir)
        assert_eq!(files.len(), 2);
        assert!(files.iter().all(|f| f.parent().unwrap() == temp_dir.path()));
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

    #[test]
    fn test_collect_all_test_files() {
        let temp_dir = TempDir::new().unwrap();

        create_test_file(temp_dir.path(), "test1.json");
        create_test_file(temp_dir.path(), "test2.json");

        let sub_dir = temp_dir.path().join("subdir");
        fs::create_dir(&sub_dir).unwrap();
        create_test_file(&sub_dir, "test3.json");

        let files = TestLoader::collect_all_test_files(temp_dir.path()).unwrap();

        // Should find all 3 files
        assert_eq!(files.len(), 3);
    }

    #[test]
    fn test_files_are_sorted() {
        let temp_dir = TempDir::new().unwrap();

        // Create files in non-alphabetical order
        create_test_file(temp_dir.path(), "z_test.json");
        create_test_file(temp_dir.path(), "a_test.json");
        create_test_file(temp_dir.path(), "m_test.json");

        let files = TestLoader::collect_test_files(temp_dir.path(), false).unwrap();

        assert_eq!(files.len(), 3);
        // Verify they're sorted
        assert!(
            files[0]
                .file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .starts_with("a_")
        );
        assert!(
            files[1]
                .file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .starts_with("m_")
        );
        assert!(
            files[2]
                .file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .starts_with("z_")
        );
    }

    #[test]
    fn test_mixed_file_types() {
        let temp_dir = TempDir::new().unwrap();

        // Create various file types
        create_test_file(temp_dir.path(), "test.json");
        fs::write(temp_dir.path().join("test.txt"), "text").unwrap();
        fs::write(temp_dir.path().join("test.md"), "markdown").unwrap();
        fs::write(temp_dir.path().join("no_extension"), "data").unwrap();

        let files = TestLoader::collect_test_files(temp_dir.path(), false).unwrap();

        // Should only find the JSON file
        assert_eq!(files.len(), 1);
        assert!(
            files[0]
                .file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .ends_with(".json")
        );
    }
}
