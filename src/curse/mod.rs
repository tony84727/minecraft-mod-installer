use regex::Regex;
use std::borrow::Cow;
use std::path::PathBuf;
use std::{fs, io};

pub mod manifest;

pub enum ManifestError {
    ParseError(serde_json::Error),
    ZipError(zip::result::ZipError),
}

pub enum DirectoryInstallError {
    IO(std::io::Error),
    ZIP(zip::result::ZipError),
}

pub struct CurseModpackArchive {
    pub file: fs::File,
}

struct PathPrefix {
    pattern: Regex,
}

impl PathPrefix {
    fn new(prefix: &str) -> Self {
        PathPrefix {
            pattern: Regex::new(format!("^{}/?", regex::escape(prefix)).as_str()).unwrap(),
        }
    }

    pub fn is_prefixed(&self, input: &str) -> bool {
        self.pattern.is_match(input)
    }

    pub fn relative<'a>(&self, path: &'a str) -> Cow<'a, str> {
        self.pattern.replace(path, "")
    }
}

impl CurseModpackArchive {
    pub fn get_manifest(&mut self) -> Result<manifest::Manifest, ManifestError> {
        let mut archive =
            zip::ZipArchive::new(&mut self.file).map_err(|err| ManifestError::ZipError(err))?;
        let mut manifest = archive
            .by_name("manifest.json")
            .map_err(|err| ManifestError::ZipError(err))?;
        serde_json::from_reader(&mut manifest).map_err(|err| ManifestError::ParseError(err))
    }

    pub async fn extract_and_install_directory(
        &mut self,
        destination: &str,
        directory: &str,
    ) -> Result<(), DirectoryInstallError> {
        let destination = PathBuf::from(destination);
        let mut archive =
            zip::ZipArchive::new(&mut self.file).map_err(|err| DirectoryInstallError::ZIP(err))?;
        let prefix = PathPrefix::new(directory);
        for i in 0..archive.len() {
            let mut file = archive
                .by_index(i)
                .map_err(|err| DirectoryInstallError::ZIP(err))?;
            let filename = file.name();
            if file.is_file() && prefix.is_prefixed(filename) {
                let local_file_path = destination.join(&*prefix.relative(filename));
                tokio::fs::create_dir_all(local_file_path.parent().unwrap())
                    .await
                    .map_err(|err| DirectoryInstallError::IO(err))?;
                let mut destination = fs::File::create(local_file_path)
                    .map_err(|err| DirectoryInstallError::IO(err))?;
                io::copy(&mut file, &mut destination)
                    .map_err(|err| DirectoryInstallError::IO(err))?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_path_prefix_is_prefixed() {
        let prefix = PathPrefix::new("overrides");
        assert!(prefix.is_prefixed("overrides/"));
        assert!(prefix.is_prefixed("overrides/file"));
        assert!(prefix.is_prefixed("overrides/directory/file"));
        assert!(prefix.is_prefixed("overrides"));
        assert!(!prefix.is_prefixed("directory/overrides"));
    }

    #[test]
    fn test_path_prefix_relative() {
        let prefix = PathPrefix::new("overrides");
        assert_eq!("file", prefix.relative("overrides/file"));
        assert_eq!(
            "directory/file",
            prefix.relative("overrides/directory/file")
        );
        assert_eq!("", prefix.relative("overrides"));
    }
}
