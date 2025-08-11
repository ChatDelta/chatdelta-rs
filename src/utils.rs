use crate::ClientError;
use std::future::Future;
use std::time::Duration;

/// Strategy for retrying failed requests
#[derive(Debug, Clone, Copy)]
pub enum RetryStrategy {
    /// Fixed delay between retries
    Fixed(Duration),
    /// Linear increase in delay (base * attempt)
    Linear(Duration),
    /// Exponential backoff (base * 2^attempt)
    Exponential(Duration),
    /// Exponential backoff with jitter (randomized delay)
    ExponentialWithJitter(Duration),
}

impl Default for RetryStrategy {
    fn default() -> Self {
        RetryStrategy::Exponential(Duration::from_secs(1))
    }
}

impl RetryStrategy {
    /// Calculate the delay for the given attempt number (0-based)
    pub fn delay(&self, attempt: u32) -> Duration {
        match self {
            RetryStrategy::Fixed(base) => *base,
            RetryStrategy::Linear(base) => *base * (attempt + 1),
            RetryStrategy::Exponential(base) => {
                let multiplier = 2_u32.saturating_pow(attempt);
                *base * multiplier
            }
            RetryStrategy::ExponentialWithJitter(base) => {
                let base_delay = 2_u32.saturating_pow(attempt);
                let jitter = rand::random::<f64>() * 0.3; // 0-30% jitter
                let multiplier = base_delay as f64 * (1.0 + jitter);
                base.mul_f64(multiplier)
            }
        }
    }
}

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
    Err(last_error.unwrap_or_else(|| {
        ClientError::config("No retry attempts were made", None)
    }))
}

/// Execute an async operation with a retry strategy.
///
/// The provided closure is executed up to `retries + 1` times, with delays
/// determined by the retry strategy.
pub async fn execute_with_retry_strategy<F, Fut, T>(
    retries: u32,
    strategy: RetryStrategy,
    mut op: F,
) -> Result<T, ClientError>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T, ClientError>>,
{
    let mut last_error = None;
    for attempt in 0..=retries {
        match op().await {
            Ok(value) => return Ok(value),
            Err(e) => {
                // Check if error is retryable
                if !is_retryable_error(&e) {
                    return Err(e);
                }
                last_error = Some(e);
            }
        }

        if attempt < retries {
            let delay = strategy.delay(attempt);
            tokio::time::sleep(delay).await;
        }
    }
    Err(last_error.unwrap_or_else(|| {
        ClientError::config("No retry attempts were made", None)
    }))
}

/// Check if an error should trigger a retry
fn is_retryable_error(error: &ClientError) -> bool {
    match error {
        ClientError::Network(_) => true,
        ClientError::Api(api_error) => {
            matches!(
                api_error.error_type,
                crate::ApiErrorType::RateLimit | crate::ApiErrorType::ServerError
            )
        }
        ClientError::Stream(stream_error) => {
            matches!(
                stream_error.error_type,
                crate::StreamErrorType::ConnectionLost
            )
        }
        _ => false,
    }
}
