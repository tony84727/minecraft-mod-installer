use std::collections::HashSet;
use std::{fs, io};

use bytes::Buf;

use crate::config::ServerSetupConfig;
use crate::curse::manifest::{File, ModInstallError};
use crate::curse::CurseModpackArchive;
use futures::{Stream, StreamExt};
use std::iter::FromIterator;
use std::path::PathBuf;

mod config;
mod curse;
mod download;

const CONFIG_FILE_NAME: &str = "server-setup-config.yaml";
const MODPACK_DOWNLOAD_LOCATION: &str = "modpack-download.zip";
const CONCURRENT_DOWNLOAD_REQUESTS: usize = 200;

enum FetchModPackError {
    FetchError(reqwest::Error),
    IO(io::Error),
    ManifestError(curse::ManifestError),
}

enum InstallError {
    FetchModPackError(FetchModPackError),
    ModInstallError(ModInstallError),
}

struct ServerInstaller<'c> {
    pub config: &'c ServerSetupConfig,
}

impl<'c> ServerInstaller<'c> {
    pub async fn install(&self) -> Result<(), InstallError> {
        self.download_modpack()
            .await
            .map_err(|err| InstallError::FetchModPackError(err))?;
        let archive = fs::File::open(MODPACK_DOWNLOAD_LOCATION)
            .map_err(|err| InstallError::FetchModPackError(FetchModPackError::IO(err)))?;
        let mut archive = CurseModpackArchive { file: archive };
        let manifest = archive.get_manifest().map_err(|err| {
            InstallError::FetchModPackError(FetchModPackError::ManifestError(err))
        })?;
        let install_list = InstallList {
            files: &manifest.files,
            ignored_project_ids: HashSet::from_iter(
                self.config
                    .install
                    .format_specific
                    .ignore_project
                    .clone()
                    .into_iter(),
            ),
        };
        let mut installation = install_list
            .install_all("mods")
            .await
            .map_err(|err| InstallError::ModInstallError(err))?;
        while let Some(result) = installation.next().await {
            if let Err(err) = result {
                return Err(InstallError::ModInstallError(err));
            }
        }
        Ok(())
    }

    async fn download_modpack(&self) -> Result<(), FetchModPackError> {
        let mut downloaded_file = fs::File::create(MODPACK_DOWNLOAD_LOCATION)
            .map_err(|err| FetchModPackError::IO(err))?;
        let body = ServerInstaller::fetch_link(&self.config.install.modpack_url)
            .await
            .map_err(|err| FetchModPackError::FetchError(err))?;
        io::copy(&mut body.reader(), &mut downloaded_file)
            .map(|_size| ())
            .map_err(|io_err| FetchModPackError::IO(io_err))
    }

    async fn fetch_link(link: &str) -> reqwest::Result<bytes::Bytes> {
        reqwest::get(link).await?.bytes().await
    }
}

struct InstallList<'f> {
    files: &'f [File],
    ignored_project_ids: HashSet<i32>,
}

impl InstallList<'_> {
    pub async fn install_all(
        &self,
        directory: &str,
    ) -> Result<impl Stream<Item = Result<(), ModInstallError>> + '_, ModInstallError> {
        let directory = PathBuf::from(directory);
        tokio::fs::create_dir_all(&directory)
            .await
            .map_err(|err| ModInstallError::IO(err))?;
        let installations = self.get_target_files().into_iter();
        let mapped =
            futures::stream::iter(installations).map(move |file| file.install(directory.clone()));
        Ok(mapped.buffer_unordered(CONCURRENT_DOWNLOAD_REQUESTS))
    }

    pub fn get_target_files(&self) -> Vec<&File> {
        self.files
            .clone()
            .into_iter()
            .filter(|file| !self.ignored_project_ids.contains(&file.project_id))
            .collect()
    }
}

#[tokio::main]
async fn main() {
    let config: ServerSetupConfig = {
        let config_file = fs::File::open(CONFIG_FILE_NAME)
            .expect(format!("unable to load config file: {}", CONFIG_FILE_NAME).as_str());
        serde_yaml::from_reader(config_file).expect("invalid config format")
    };
    let installer = ServerInstaller { config: &config };
    match installer.install().await {
        Ok(()) => println!("install complete"),
        Err(err) => match err {
            InstallError::FetchModPackError(err) => match err {
                FetchModPackError::IO(err) => {
                    println!("{}", err)
                }
                FetchModPackError::FetchError(err) => {
                    println!("{}", err)
                }
                FetchModPackError::ManifestError(err) => match err {
                    curse::ManifestError::ParseError(_err) => {
                        println!("unable to parse manifest")
                    }
                    curse::ManifestError::ZipError(_err) => {
                        println!("unable to unzip the modpack to get manifest")
                    }
                },
            },
            InstallError::ModInstallError(err) => match err {
                ModInstallError::Network(err) => {
                    println!("network error: {}", err);
                }
                ModInstallError::IO(err) => {
                    println!("io error: {}", err)
                }
                ModInstallError::UrlParseError(err) => {
                    println!("parse error: {}", err)
                }
                ModInstallError::DownloadError(err) => {
                    println!("download error: {:?}", err)
                }
            },
        },
    };
}

#[cfg(test)]
mod tests {
    use crate::curse::manifest::File;
    use crate::InstallList;
    use std::collections::HashSet;
    use std::iter::FromIterator;

    #[test]
    fn test_install_list_get_targets() {
        fn create_fake_file(id: i32) -> File {
            File {
                project_id: id,
                file_id: id,
                required: true,
            }
        }
        let fake_files = &[
            create_fake_file(1),
            create_fake_file(2),
            create_fake_file(3),
            create_fake_file(4),
            create_fake_file(5),
        ];
        let install_list = InstallList {
            files: fake_files,
            ignored_project_ids: HashSet::from_iter(vec![1, 2, 3, 4].into_iter()),
        };
        assert_eq!(
            vec![5],
            install_list
                .get_target_files()
                .iter()
                .map(|f| f.project_id)
                .collect::<Vec<i32>>()
        );
    }
}
