use crate::Result;

cfg_if::cfg_if! {
    if #[cfg(target_arch = "wasm32")] {
        /// Key-value persistence seam for Sage's config and keychain files.
        pub trait KvStore: std::fmt::Debug {
            fn read(&self, name: &str) -> Result<Option<Vec<u8>>>;
            fn write(&self, name: &str, bytes: &[u8]) -> Result<()>;
        }
    } else {
        /// Key-value persistence seam for Sage's config and keychain files.
        pub trait KvStore: std::fmt::Debug + Send + Sync {
            fn read(&self, name: &str) -> Result<Option<Vec<u8>>>;
            fn write(&self, name: &str, bytes: &[u8]) -> Result<()>;
        }
    }
}

/// Filesystem-backed store rooted at the Sage data directory.
#[cfg(feature = "native")]
#[derive(Debug)]
pub struct FsStore {
    root: std::path::PathBuf,
}

#[cfg(feature = "native")]
impl FsStore {
    pub fn new(root: std::path::PathBuf) -> Self {
        Self { root }
    }
}

#[cfg(feature = "native")]
impl KvStore for FsStore {
    fn read(&self, name: &str) -> Result<Option<Vec<u8>>> {
        let path = self.root.join(name);

        if path.try_exists()? {
            Ok(Some(std::fs::read(path)?))
        } else {
            Ok(None)
        }
    }

    fn write(&self, name: &str, bytes: &[u8]) -> Result<()> {
        let path = self.root.join(name);

        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        std::fs::write(path, bytes)?;

        Ok(())
    }
}
