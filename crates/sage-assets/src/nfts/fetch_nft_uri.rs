use chia_protocol::Bytes32;
use chia_sha2::Sha256;
use futures_lite::StreamExt;
use futures_util::stream::FuturesUnordered;
use mime_sniffer::MimeTypeSniffer;
use reqwest::{StatusCode, header::CONTENT_TYPE};
use tracing::debug;

use crate::UriError;

use super::{Data, Thumbnail, ThumbnailError, thumbnail as make_thumbnail};

/// Sizes an image down. Decoding is enough work to stall whatever else is
/// waiting, so where there is a thread pool it runs there; the browser has one
/// thread and takes the pause.
#[cfg(feature = "native")]
async fn resize(blob: &[u8], mime_type: &str) -> Result<Option<Thumbnail>, ThumbnailError> {
    let blob = blob.to_vec();
    let mime_type = mime_type.to_string();

    tokio::task::spawn_blocking(move || make_thumbnail(&blob, &mime_type))
        .await
        .map_err(|error| std::io::Error::other(error.to_string()))?
}

#[cfg(not(feature = "native"))]
async fn resize(blob: &[u8], mime_type: &str) -> Result<Option<Thumbnail>, ThumbnailError> {
    make_thumbnail(blob, mime_type)
}

pub async fn fetch_uri(uri: String, testnet: bool) -> Result<Data, UriError> {
    let response = reqwest::get(&uri).await?;

    let mime_type = match response.headers().get(CONTENT_TYPE) {
        Some(header) => Some(
            header
                .to_str()
                .map(ToString::to_string)
                .map_err(|_| UriError::InvalidContentType)?,
        ),
        None => None,
    };

    let blob = response.bytes().await?.to_vec();

    let mime_type = if let Some(mime_type) = mime_type {
        mime_type
    } else {
        blob.as_slice()
            .sniff_mime_type()
            .unwrap_or("image/png")
            .to_string()
    };

    let mut hasher = Sha256::new();
    hasher.update(&blob);
    let hash = Bytes32::new(hasher.finalize());

    let mut thumbnail = match mintgarden_thumbnail(hash, testnet).await {
        Ok(thumbnail) => thumbnail,
        Err(error) => {
            debug!("Failed to fetch MintGarden thumbnail for {uri}: {error}");
            None
        }
    };

    if thumbnail.is_none() {
        thumbnail = match resize(&blob, &mime_type).await {
            Ok(thumbnail) => thumbnail,
            Err(error) => {
                debug!("No thumbnail created for {uri}: {error}");
                None
            }
        };
    }

    Ok(Data {
        blob,
        mime_type,
        hash,
        thumbnail,
    })
}

pub async fn fetch_uris_without_hash(uris: Vec<String>, testnet: bool) -> Result<Data, UriError> {
    let mut futures = FuturesUnordered::new();

    for uri in uris {
        futures.push(fetch_uri(uri, testnet));
    }

    let mut data = None;

    while let Some(result) = futures.next().await {
        let item = result?;

        let Some(old_data) = data.take() else {
            data = Some(item);
            continue;
        };

        if old_data.hash != item.hash {
            return Err(UriError::HashMismatch {
                expected: old_data.hash,
                found: item.hash,
            });
        }

        if old_data.mime_type != item.mime_type {
            return Err(UriError::MimeTypeMismatch {
                expected: old_data.mime_type,
                found: item.mime_type,
            });
        }

        data = Some(old_data);
    }

    data.ok_or(UriError::NoUris)
}

pub async fn fetch_uris_with_hash(uris: Vec<String>, hash: Bytes32, testnet: bool) -> Option<Data> {
    let mut futures = FuturesUnordered::new();

    for uri in uris {
        futures.push(async move { (uri.clone(), fetch_uri(uri, testnet).await) });
    }

    while let Some((uri, result)) = futures.next().await {
        let Ok(item) = result else {
            debug!("Failed to fetch NFT URI {uri}, expected hash {hash}");
            continue;
        };

        if hash != item.hash {
            return None;
        }

        return Some(item);
    }

    None
}

pub async fn mintgarden_thumbnail(
    data_hash: Bytes32,
    testnet: bool,
) -> Result<Option<Thumbnail>, UriError> {
    let url = format!(
        "https://assets.{}.mintgarden.io/thumbnails/{data_hash}_512.webp",
        if testnet { "testnet" } else { "mainnet" }
    );

    let response = reqwest::get(&url).await?;

    if response.status() != StatusCode::OK {
        return Ok(None);
    }

    let bytes = response.bytes().await?;

    Ok(make_thumbnail(&bytes, "image/webp")?)
}
