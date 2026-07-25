use chia_protocol::Bytes32;
use thiserror::Error;

#[cfg(feature = "native")]
use crate::ThumbnailError;

#[derive(Debug, Error)]
pub enum UriError {
    #[error("Failed to fetch NFT data: {0}")]
    Fetch(#[from] reqwest::Error),

    #[error("Missing or invalid content type")]
    InvalidContentType,

    #[error("Mime type mismatch, expected {expected} but found {found}")]
    MimeTypeMismatch { expected: String, found: String },

    #[error("Hash mismatch, expected {expected} but found {found}")]
    HashMismatch { expected: Bytes32, found: Bytes32 },

    #[error("No URIs provided")]
    NoUris,

    // Thumbnails are generated with image codecs that only the native build
    // links in.
    #[cfg(feature = "native")]
    #[error("Failed to create thumbnail: {0}")]
    Thumbnail(#[from] ThumbnailError),
}
