use std::fs;

pub mod manifest;

pub enum ManifestError {
    ParseError(serde_json::Error),
    ZipError(zip::result::ZipError),
}

pub struct CurseModpackArchive {
    pub file: fs::File,
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
}
