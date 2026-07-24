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
