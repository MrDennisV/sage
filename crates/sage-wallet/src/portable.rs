use std::time::Duration;

cfg_if::cfg_if! {
    if #[cfg(target_arch = "wasm32")] {
        pub async fn sleep(duration: Duration) {
            gloo_timers::future::TimeoutFuture::new(
                u32::try_from(duration.as_millis()).unwrap_or(u32::MAX),
            )
            .await;
        }
    } else {
        pub async fn sleep(duration: Duration) {
            tokio::time::sleep(duration).await;
        }
    }
}

/// The current Unix timestamp in seconds, falling back to the epoch if the
/// system clock is set before 1970.
pub fn unix_timestamp() -> u64 {
    #[cfg(not(target_arch = "wasm32"))]
    {
        use std::time::{SystemTime, UNIX_EPOCH};

        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }

    #[cfg(target_arch = "wasm32")]
    {
        (js_sys::Date::now() / 1000.0) as u64
    }
}

pub fn base64_data_uri(blob: &[u8], mime_type: &str) -> String {
    use base64::{Engine, prelude::BASE64_STANDARD};

    format!("data:{mime_type};base64,{}", BASE64_STANDARD.encode(blob))
}
