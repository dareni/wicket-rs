use std::fs::File;
use std::io::Error;
use std::path::{Path, PathBuf};

/// Abstraction for finding and loading markup resources.
pub trait ResourceStreamLocator {
    /// Attempt to locate a resource at the given path.
    /// Returns `Ok(File)` if found, `Err` if not found.
    fn locate(
        &self,
        path: &Path,
        variation: Option<&str>,
        extension: Option<&str>,
    ) -> Result<File, Error>;
}

pub struct FileResourceStreamLocator {
    roots: Vec<PathBuf>,
}

impl FileResourceStreamLocator {
    /// The root directory to start searching from (e.g., "/data" or "/opt/assets")
    pub fn new(roots: Vec<PathBuf>) -> Self {
        Self { roots }
    }
}

impl ResourceStreamLocator for FileResourceStreamLocator {
    /// Given the relative path to the resource, apply the root paths and extract and return the
    /// file resource.
    fn locate(
        &self,
        relative_path: &Path,
        variation: Option<&str>,
        extension: Option<&str>,
    ) -> Result<File, Error> {
        //Build the resource name.

        let mut resource_path = variation
            .and_then(|v| {
                relative_path.file_name().and_then(|fname| {
                    fname.to_str().map(|name| {
                        let mut name_variation = name.to_owned();
                        name_variation.push('_');
                        name_variation.push_str(v);
                        relative_path.to_path_buf().with_file_name(name_variation)
                    })
                })
            })
            .unwrap_or(relative_path.to_path_buf());

        resource_path = extension
            .map(|ext| {
                let path = resource_path.clone();
                path.with_extension(ext)
            })
            .unwrap_or(resource_path);

        // Search through all configured root folders
        for root in &self.roots {
            let full_path = root.to_path_buf().join(&resource_path);

            if full_path.exists() {
                let cursor = File::open(full_path);
                return cursor;
            }
        }
        let mut roots = String::with_capacity(200);
        for (idx, root) in self.roots.iter().enumerate() {
            if idx != 0 {
                roots.push_str(", ");
            }
            roots.push_str(root.to_str().unwrap_or("None!"));
        }

        let error = format!(
            "Error locating resource. Relative file:'{}' does not exist in locations: '{}'",
            resource_path.to_str().unwrap_or("None!"),
            roots
        );
        Err(Error::other(error))
    }
}
