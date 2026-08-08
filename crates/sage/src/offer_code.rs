use reqwest::StatusCode;
use serde::{Deserialize, Serialize};

use crate::{Error, Result};

/// Exchanges a Chia Network offer code, the kind printed on physical offer
/// cards, for the offer it stands for.
pub async fn download_cni_offercode(code: String) -> Result<String> {
    #[derive(Serialize)]
    struct Request {
        code: String,
    }

    #[derive(Deserialize)]
    struct Response {
        offer: String,
    }

    let response = reqwest::Client::new()
        .post("https://offercodes.chia.net/download_offer")
        .json(&Request { code: code.clone() })
        .send()
        .await?;

    if response.status() != StatusCode::OK {
        return Err(Error::OfferCode(format!(
            "Invalid offer code {code}: Server responded with code {}",
            response.status()
        )));
    }

    Ok(response.json::<Response>().await?.offer)
}
