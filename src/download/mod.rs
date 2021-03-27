use bytes::{Buf, Bytes};
use std::{fs, io};

#[derive(Debug)]
pub enum DownloadError {
    IO(io::Error),
    Network(reqwest::Error),
}

async fn download_url(url: &str) -> reqwest::Result<Bytes> {
    reqwest::get(url).await?.bytes().await
}

pub async fn download(url: &str, install_path: &str) -> Result<(), DownloadError> {
    let downloaded_file = download_url(url)
        .await
        .map_err(|err| DownloadError::Network(err))?;
    let mut file = fs::File::create(install_path).map_err(|err| DownloadError::IO(err))?;
    async { io::copy(&mut downloaded_file.reader(), &mut file) }
        .await
        .map(|_size| ())
        .map_err(|err| DownloadError::IO(err))
}
