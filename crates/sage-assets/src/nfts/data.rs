use chia_protocol::Bytes32;

/// A blob fetched from an NFT URI, along with its content hash and the
/// preview images derived from it.
#[derive(Debug, Clone)]
pub struct Data {
    pub blob: Vec<u8>,
    pub mime_type: String,
    pub hash: Bytes32,
    pub thumbnail: Option<Thumbnail>,
}

/// The downscaled previews generated for an image blob.
#[derive(Debug, Clone)]
pub struct Thumbnail {
    pub icon: Vec<u8>,
    pub thumbnail: Vec<u8>,
}
