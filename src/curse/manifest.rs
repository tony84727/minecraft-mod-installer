use serde::{Deserialize, Serialize};
use std::{io, path::PathBuf};

#[derive(Serialize, Deserialize, Debug)]
pub struct ModLoader {
    pub id: String,
}

pub enum ModInstallError {
    Network(reqwest::Error),
    IO(io::Error),
    UrlParseError(url::ParseError),
    DownloadError(crate::download::DownloadError),
}

#[derive(Serialize, Deserialize, Debug)]
pub struct File {
    #[serde(rename = "projectID")]
    pub project_id: i32,
    #[serde(rename = "fileID")]
    pub file_id: i32,
    pub required: bool,
}

impl File {
    pub async fn install(&self, dir: PathBuf) -> Result<(), ModInstallError> {
        let download_url = self
            .query_download_url()
            .await
            .map_err(|err| ModInstallError::Network(err))?;
        let filename = File::filename_from_url(&download_url)
            .map_err(|err| ModInstallError::UrlParseError(err))?;
        println!("installing {}....", filename);
        crate::download::download(
            &download_url,
            dir.join(filename)
                .to_str()
                .expect("unable to get str from PathBuf"),
        )
        .await
        .map_err(|err| ModInstallError::DownloadError(err))
    }

    async fn query_download_url(&self) -> reqwest::Result<String> {
        reqwest::get(format!(
            "https://addons-ecs.forgesvc.net/api/v2/addon/{}/file/{}/download-url",
            self.project_id, self.file_id
        ))
        .await?
        .text()
        .await
    }

    fn filename_from_url(url_str: &str) -> Result<String, url::ParseError> {
        let parsed_url = url::Url::parse(url_str)?;
        let url_path = parsed_url.path();
        Ok(std::path::PathBuf::from(url_path)
            .file_name()
            .expect("unable to get filename")
            .to_str()
            .expect("unable get str from osStr")
            .to_string())
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Minecraft {
    pub version: String,
    #[serde(rename = "modLoaders")]
    pub mod_loaders: Vec<ModLoader>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Manifest {
    pub minecraft: Minecraft,
    pub files: Vec<File>,
    pub overrides: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_parse_manifest() {
        let sample_manifest = include_str!("testdata/manifest.json");
        let parsed: Manifest = serde_json::from_str(sample_manifest).expect("should able to parse");
        assert_eq!(265, parsed.files.len());
        assert_eq!("1.16.5".to_string(), parsed.minecraft.version);
        assert_eq!(1, parsed.minecraft.mod_loaders.len());
        assert_eq!(
            "forge-36.1.0".to_string(),
            parsed.minecraft.mod_loaders[0].id
        );
    }

    #[test]
    fn test_file_filename_from_url() {
        let url = "https://example.com/cats/cat.png?q=100";
        let filename = File::filename_from_url(url).expect("should be a valid url");
        assert_eq!("cat.png", filename);
    }
}
