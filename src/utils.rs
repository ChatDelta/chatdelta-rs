use crate::ClientError;
use std::future::Future;
use std::time::Duration;

/// Execute an async operation with retry logic.
///
/// The provided closure is executed up to `retries + 1` times, waiting
/// an exponentially increasing delay between attempts.
pub async fn execute_with_retry<F, Fut, T>(retries: u32, mut op: F) -> Result<T, ClientError>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T, ClientError>>,
{
    let mut last_error = None;
    for attempt in 0..=retries {
        match op().await {
            Ok(value) => return Ok(value),
            Err(e) => last_error = Some(e),
        }

        if attempt < retries {
            tokio::time::sleep(Duration::from_millis(1000 * (attempt + 1) as u64)).await;
        }
    }
    Err(last_error.unwrap())
}
