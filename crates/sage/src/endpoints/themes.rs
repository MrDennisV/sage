#[cfg(not(feature = "native"))]
use indexmap::IndexMap;
use sage_api::{
    DeleteUserTheme, DeleteUserThemeResponse, GetNftData, GetUserTheme, GetUserThemeResponse,
    GetUserThemes, GetUserThemesResponse, SaveUserTheme, SaveUserThemeResponse,
};
use sage_database::SqlExecutor;
use serde_json::Value;
#[cfg(feature = "native")]
use tokio::fs;

use crate::{Error, Result, Sage};

impl<E: SqlExecutor> Sage<E> {
    /// Pulls the `data.theme` node out of an NFT's metadata and stamps the NFT
    /// id in as the theme name, which is how a saved theme is identified later.
    /// Returns `None` when the NFT has no metadata to read a theme from.
    pub async fn nft_theme_json(&self, nft_id: &str) -> Result<Option<String>> {
        let response = self
            .get_nft_data(GetNftData {
                nft_id: nft_id.to_string(),
            })
            .await?;

        let Some(metadata_json) = response.data.and_then(|data| data.metadata_json) else {
            return Ok(None);
        };

        let metadata: Value =
            serde_json::from_str(&metadata_json).map_err(|_| Error::InvalidThemeJson)?;

        let mut theme = metadata
            .get("data")
            .and_then(|data| data.get("theme"))
            .ok_or(Error::MissingThemeData)?
            .clone();

        if let Some(object) = theme.as_object_mut() {
            object.insert("name".to_string(), Value::String(nft_id.to_string()));
        }

        serde_json::to_string_pretty(&theme)
            .map(Some)
            .map_err(|_| Error::InvalidThemeJson)
    }
}

// Each theme is a directory holding a `theme.json`, under the Sage data
// directory.
#[cfg(feature = "native")]
impl Sage {
    pub async fn delete_user_theme(&self, req: DeleteUserTheme) -> Result<DeleteUserThemeResponse> {
        if req.nft_id.is_empty() {
            return Ok(DeleteUserThemeResponse {});
        }

        let themes_dir = self.path.join("themes");
        let theme_dir = themes_dir.join(&req.nft_id);

        if !theme_dir.exists() {
            return Ok(DeleteUserThemeResponse {});
        }

        fs::remove_dir_all(&theme_dir).await?;

        Ok(DeleteUserThemeResponse {})
    }

    pub async fn get_user_theme(&self, req: GetUserTheme) -> Result<GetUserThemeResponse> {
        if req.nft_id.is_empty() {
            return Ok(GetUserThemeResponse { theme: None });
        }

        let themes_dir = self.path.join("themes");
        let theme_dir = themes_dir.join(&req.nft_id);
        let theme_json_path = theme_dir.join("theme.json");

        if !theme_json_path.exists() {
            return Ok(GetUserThemeResponse { theme: None });
        }

        let theme_json = fs::read_to_string(&theme_json_path).await?;

        Ok(GetUserThemeResponse {
            theme: Some(theme_json),
        })
    }

    pub async fn save_user_theme(&self, req: SaveUserTheme) -> Result<SaveUserThemeResponse> {
        if req.nft_id.is_empty() {
            return Ok(SaveUserThemeResponse {});
        }

        let themes_dir = self.path.join("themes");

        if !themes_dir.exists() {
            fs::create_dir_all(&themes_dir).await?;
        }

        let nft_theme_dir = themes_dir.join(&req.nft_id);

        if !nft_theme_dir.exists() {
            fs::create_dir_all(&nft_theme_dir).await?;
        }

        if let Some(theme_json) = self.nft_theme_json(&req.nft_id).await? {
            fs::write(nft_theme_dir.join("theme.json"), theme_json).await?;
        }

        Ok(SaveUserThemeResponse {})
    }

    pub async fn get_user_themes(&self, _req: GetUserThemes) -> Result<GetUserThemesResponse> {
        let themes_dir = self.path.join("themes");
        let mut themes = Vec::new();

        if !themes_dir.exists() {
            return Ok(GetUserThemesResponse { themes });
        }

        match fs::read_dir(&themes_dir).await {
            Ok(mut entries) => {
                while let Some(entry) = entries.next_entry().await? {
                    let path = entry.path();

                    if path.is_dir() {
                        let theme_json_path = path.join("theme.json");

                        if theme_json_path.exists() {
                            match fs::read_to_string(&theme_json_path).await {
                                Ok(theme_content) => {
                                    themes.push(theme_content);
                                }
                                Err(e) => {
                                    eprintln!(
                                        "Failed to read theme.json in {}: {e}",
                                        path.display()
                                    );
                                }
                            }
                        }
                    }
                }
            }
            Err(e) => {
                eprintln!("Failed to read themes directory: {e}");
            }
        }

        Ok(GetUserThemesResponse { themes })
    }
}

/// The browser has no directories, so themes go into the key-value store that
/// already holds the config and the keychain. They share a single entry mapping
/// NFT id to theme JSON, because the store cannot enumerate keys and listing
/// every saved theme is exactly what `get_user_themes` has to do.
#[cfg(not(feature = "native"))]
const THEMES_KEY: &str = "themes.json";

#[cfg(not(feature = "native"))]
impl<E: SqlExecutor> Sage<E> {
    pub fn get_user_theme(&self, req: GetUserTheme) -> Result<GetUserThemeResponse> {
        Ok(GetUserThemeResponse {
            theme: self.user_themes()?.get(&req.nft_id).cloned(),
        })
    }

    pub fn get_user_themes(&self, _req: GetUserThemes) -> Result<GetUserThemesResponse> {
        Ok(GetUserThemesResponse {
            themes: self.user_themes()?.into_values().collect(),
        })
    }

    pub async fn save_user_theme(&self, req: SaveUserTheme) -> Result<SaveUserThemeResponse> {
        if req.nft_id.is_empty() {
            return Ok(SaveUserThemeResponse {});
        }

        if let Some(theme) = self.nft_theme_json(&req.nft_id).await? {
            let mut themes = self.user_themes()?;
            themes.insert(req.nft_id, theme);
            self.save_user_themes(&themes)?;
        }

        Ok(SaveUserThemeResponse {})
    }

    pub fn delete_user_theme(&self, req: DeleteUserTheme) -> Result<DeleteUserThemeResponse> {
        let mut themes = self.user_themes()?;

        if themes.shift_remove(&req.nft_id).is_some() {
            self.save_user_themes(&themes)?;
        }

        Ok(DeleteUserThemeResponse {})
    }

    /// The saved themes, kept in insertion order so the interface lists them
    /// the same way every time.
    fn user_themes(&self) -> Result<IndexMap<String, String>> {
        let Some(bytes) = self.store.read(THEMES_KEY)? else {
            return Ok(IndexMap::new());
        };

        serde_json::from_slice(&bytes)
            .map_err(|error| Error::Store(format!("invalid theme store: {error}")))
    }

    fn save_user_themes(&self, themes: &IndexMap<String, String>) -> Result<()> {
        let bytes = serde_json::to_vec(themes)
            .map_err(|error| Error::Store(format!("cannot encode themes: {error}")))?;

        self.store.write(THEMES_KEY, &bytes)
    }
}
