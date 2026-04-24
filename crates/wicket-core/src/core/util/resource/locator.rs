use std::fs::File;
use std::io::Error;
use std::path::{Path, PathBuf};

use crate::markup::{FileResourceStream, ResourceStream};

/// Abstraction for finding and loading markup resources.
pub trait ResourceStreamLocator {
    /// Attempt to locate a resource at the given path.
    /// Returns `Ok(File)` if found, `Err` if not found.
    fn locate(
        &self,
        path: &Path,
        variation: &Option<String>,
        extension: &Option<String>,
    ) -> Result<Box<dyn ResourceStream>, Error>;
}

pub struct FileResourceStreamLocator {
    roots: Vec<PathBuf>,
}

impl FileResourceStreamLocator {
    /// The root directory to start searching from (e.g., "/data" or "/opt/assets")
    pub fn new(roots: Vec<PathBuf>) -> Self {
        Self { roots }
    }
    fn get_pathbuf(
        relative_path: &Path,
        variation: &Option<String>,
        extension: &Option<String>,
    ) -> PathBuf
    where
        Self: Sized,
    {
        let mut resource_path = variation
            .as_ref()
            .and_then(|v| {
                relative_path.file_name().and_then(|fname| {
                    fname.to_str().map(|name| {
                        let mut name_variation = name.to_owned();
                        name_variation.push('_');
                        name_variation.push_str(v.as_str());
                        relative_path.to_path_buf().with_file_name(name_variation)
                    })
                })
            })
            .unwrap_or(relative_path.to_path_buf());

        resource_path = extension
            .as_ref()
            .map(|ext| {
                let path = resource_path.clone();
                path.with_extension(ext)
            })
            .unwrap_or(resource_path);

        resource_path
    }
}

impl ResourceStreamLocator for FileResourceStreamLocator {
    /// Given the relative path to the resource, apply the root paths and extract and return the
    /// file resource.
    fn locate(
        &self,
        relative_path: &Path,
        variation: &Option<String>,
        extension: &Option<String>,
    ) -> Result<Box<dyn ResourceStream>, Error> {
        //Build the resource name.
        let resource_path = Self::get_pathbuf(relative_path, variation, extension);
        // Search through all configured root folders
        for root in &self.roots {
            let full_path = root.to_path_buf().join(&resource_path);

            if full_path.exists() {
                let file = File::open(full_path)?;
                let file_resource_stream = Box::from(FileResourceStream {
                    file,
                    variation: variation.clone(),
                });
                return Ok(file_resource_stream);
            }
        }

        let roots_display = self
            .roots
            .iter()
            .map(|p| p.to_string_lossy())
            .collect::<Vec<_>>()
            .join(", ");

        let error = format!(
            "Error locating resource. Relative file:'{}' does not exist in locations: '{}'",
            resource_path.to_str().unwrap_or("None!"),
            roots_display
        );
        Err(Error::other(error))
    }
}

#[cfg(test)]
mod test {
    use wicket_util::constants::file_ext;

    use crate::core::util::resource::locator::FileResourceStreamLocator;
    use std::path::Path;

    #[test]
    pub fn path_test() {
        let path = FileResourceStreamLocator::get_pathbuf(
            Path::new("prog"),
            &Some("1".to_owned()),
            &Some(file_ext::HTML.to_owned()),
        );
        assert_eq!("prog_1.html", path.to_str().unwrap());
        let path = FileResourceStreamLocator::get_pathbuf(
            Path::new("prog"),
            &None,
            &Some(file_ext::HTML.to_owned()),
        );
        assert_eq!("prog.html", path.to_str().unwrap());

        let path =
            FileResourceStreamLocator::get_pathbuf(Path::new("prog"), &Some("1".to_owned()), &None);
        assert_eq!("prog_1", path.to_str().unwrap());
    }
}
