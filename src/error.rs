//! Error types for the ChatDelta AI client library

use std::fmt;

/// Errors that can occur when using AI clients
#[derive(Debug)]
pub enum ClientError {
    /// Network-related errors (timeouts, connection failures, etc.)
    Network(NetworkError),
    /// API-specific errors (invalid responses, rate limits, etc.)
    Api(ApiError),
    /// Authentication errors (invalid API keys, etc.)
    Authentication(AuthError),
    /// Configuration errors (invalid parameters, etc.)
    Configuration(ConfigError),
    /// Response parsing errors
    Parse(ParseError),
    /// Streaming-related errors
    Stream(StreamError),
}

/// Network-related error details
#[derive(Debug)]
pub struct NetworkError {
    pub message: String,
    pub error_type: NetworkErrorType,
}

#[derive(Debug)]
pub enum NetworkErrorType {
    Timeout,
    ConnectionFailed,
    DnsResolution,
    Other,
}

/// API-related error details
#[derive(Debug)]
pub struct ApiError {
    pub message: String,
    pub status_code: Option<u16>,
    pub error_type: ApiErrorType,
}

#[derive(Debug)]
pub enum ApiErrorType {
    RateLimit,
    QuotaExceeded,
    InvalidModel,
    ContentFilter,
    ServerError,
    BadRequest,
    Other,
}

/// Authentication error details
#[derive(Debug)]
pub struct AuthError {
    pub message: String,
    pub error_type: AuthErrorType,
}

#[derive(Debug)]
pub enum AuthErrorType {
    InvalidApiKey,
    MissingApiKey,
    ExpiredToken,
    InsufficientPermissions,
    Other,
}

/// Configuration error details
#[derive(Debug)]
pub struct ConfigError {
    pub message: String,
    pub parameter: Option<String>,
}

/// Parse error details
#[derive(Debug)]
pub struct ParseError {
    pub message: String,
    pub error_type: ParseErrorType,
}

#[derive(Debug)]
pub enum ParseErrorType {
    JsonParsing,
    MissingField,
    InvalidFormat,
    Other,
}

/// Streaming error details
#[derive(Debug)]
pub struct StreamError {
    pub message: String,
    pub error_type: StreamErrorType,
}

#[derive(Debug)]
pub enum StreamErrorType {
    ConnectionLost,
    InvalidChunk,
    StreamClosed,
    Other,
}

impl ClientError {
    /// Create a timeout network error
    pub fn timeout(message: impl Into<String>) -> Self {
        Self::Network(NetworkError {
            message: message.into(),
            error_type: NetworkErrorType::Timeout,
        })
    }

    /// Create a rate limit API error
    pub fn rate_limit(message: impl Into<String>) -> Self {
        Self::Api(ApiError {
            message: message.into(),
            status_code: Some(429),
            error_type: ApiErrorType::RateLimit,
        })
    }

    /// Create an invalid API key error
    pub fn invalid_api_key(message: impl Into<String>) -> Self {
        Self::Authentication(AuthError {
            message: message.into(),
            error_type: AuthErrorType::InvalidApiKey,
        })
    }

    /// Create a configuration error
    pub fn config(message: impl Into<String>, parameter: Option<String>) -> Self {
        Self::Configuration(ConfigError {
            message: message.into(),
            parameter,
        })
    }

    /// Create a JSON parsing error
    pub fn json_parse(message: impl Into<String>) -> Self {
        Self::Parse(ParseError {
            message: message.into(),
            error_type: ParseErrorType::JsonParsing,
        })
    }
}

impl fmt::Display for ClientError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ClientError::Network(err) => write!(f, "Network error: {}", err.message),
            ClientError::Api(err) => {
                if let Some(status) = err.status_code {
                    write!(f, "API error ({}): {}", status, err.message)
                } else {
                    write!(f, "API error: {}", err.message)
                }
            }
            ClientError::Authentication(err) => write!(f, "Authentication error: {}", err.message),
            ClientError::Configuration(err) => {
                if let Some(param) = &err.parameter {
                    write!(f, "Configuration error ({}): {}", param, err.message)
                } else {
                    write!(f, "Configuration error: {}", err.message)
                }
            }
            ClientError::Parse(err) => write!(f, "Parse error: {}", err.message),
            ClientError::Stream(err) => write!(f, "Stream error: {}", err.message),
        }
    }
}

impl std::error::Error for ClientError {}

impl From<reqwest::Error> for ClientError {
    fn from(err: reqwest::Error) -> Self {
        if err.is_timeout() {
            let url = err.url().map(|u| u.as_str()).unwrap_or("unknown");
            ClientError::Network(NetworkError {
                message: format!("Request timed out after attempting to reach {}. Consider increasing timeout or checking network connectivity.", url),
                error_type: NetworkErrorType::Timeout,
            })
        } else if err.is_connect() {
            let host = err.url()
                .and_then(|u| u.host_str())
                .unwrap_or("unknown host");
            ClientError::Network(NetworkError {
                message: format!("Failed to connect to {}. Check internet connectivity and DNS resolution.", host),
                error_type: NetworkErrorType::ConnectionFailed,
            })
        } else if err.status().is_some() {
            let status = err.status().unwrap();
            let status_code = status.as_u16();

            if status_code == 401 {
                ClientError::Authentication(AuthError {
                    message: "Invalid API key".to_string(),
                    error_type: AuthErrorType::InvalidApiKey,
                })
            } else if status_code == 429 {
                ClientError::Api(ApiError {
                    message: "Rate limit exceeded".to_string(),
                    status_code: Some(status_code),
                    error_type: ApiErrorType::RateLimit,
                })
            } else if status_code >= 500 {
                ClientError::Api(ApiError {
                    message: format!("Server error: {err}"),
                    status_code: Some(status_code),
                    error_type: ApiErrorType::ServerError,
                })
            } else if status_code >= 400 {
                ClientError::Api(ApiError {
                    message: format!("Bad request: {err}"),
                    status_code: Some(status_code),
                    error_type: ApiErrorType::BadRequest,
                })
            } else {
                ClientError::Api(ApiError {
                    message: format!("HTTP {status}: {err}"),
                    status_code: Some(status_code),
                    error_type: ApiErrorType::Other,
                })
            }
        } else {
            ClientError::Network(NetworkError {
                message: err.to_string(),
                error_type: NetworkErrorType::Other,
            })
        }
    }
}

impl From<serde_json::Error> for ClientError {
    fn from(err: serde_json::Error) -> Self {
        ClientError::Parse(ParseError {
            message: format!("JSON parsing failed: {err}"),
            error_type: ParseErrorType::JsonParsing,
        })
    }
}
