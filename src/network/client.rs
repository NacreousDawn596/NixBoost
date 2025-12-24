// NixBoost - High-performance NixOS package manager frontend
// Copyright (C) 2025 nacreousdawn596, compiledkernel-idk and NixBoost contributors
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

//! HTTP client with retry logic for NixBoost.

use crate::core::config::Config;
use crate::core::error::{NetworkError, Result};
use reqwest::{Client, Response};
use std::time::Duration;
use tracing::{debug, warn};

/// HTTP client with retry logic
pub struct HttpClient {
    client: Client,
    max_retries: u32,
    retry_delay: Duration,
}

impl HttpClient {
    /// Create a new HTTP client with default settings
    pub fn new() -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .connect_timeout(Duration::from_secs(10))
            .user_agent(format!("nixboost/{}", env!("CARGO_PKG_VERSION")))
            .gzip(true)
            .brotli(true)
            .build()
            .unwrap_or_default();

        Self {
            client,
            max_retries: 3,
            retry_delay: Duration::from_secs(1),
        }
    }

    /// Create from configuration
    pub fn from_config(config: &Config) -> Self {
        let mut builder = Client::builder()
            .timeout(Duration::from_secs(config.network.timeout_secs))
            .connect_timeout(Duration::from_secs(config.network.connect_timeout_secs))
            .user_agent(&config.network.user_agent)
            .gzip(true)
            .brotli(true);

        if let Some(ref proxy) = config.network.proxy {
            if let Ok(proxy) = reqwest::Proxy::all(proxy) {
                builder = builder.proxy(proxy);
            }
        }

        let client = builder.build().unwrap_or_default();

        Self {
            client,
            max_retries: config.network.max_retries,
            retry_delay: Duration::from_millis(config.network.retry_delay_ms),
        }
    }

    /// Set maximum retries
    pub fn max_retries(mut self, retries: u32) -> Self {
        self.max_retries = retries;
        self
    }

    /// Set retry delay
    pub fn retry_delay(mut self, delay: Duration) -> Self {
        self.retry_delay = delay;
        self
    }

    /// GET request with retry
    pub async fn get(&self, url: &str) -> Result<Response> {
        self.request_with_retry(|| self.client.get(url).send()).await
    }

    /// GET request returning body as string with retry
    pub async fn get_string(&self, url: &str) -> Result<String> {
        let response = self.get(url).await?;
        let text = response.text().await
            .map_err(|e| NetworkError::DownloadFailed(e.to_string()))?;
        Ok(text)
    }

    /// GET request returning body as bytes with retry
    pub async fn get_bytes(&self, url: &str) -> Result<Vec<u8>> {
        let response = self.get(url).await?;
        let bytes = response.bytes().await
            .map_err(|e| NetworkError::DownloadFailed(e.to_string()))?;
        Ok(bytes.to_vec())
    }

    /// GET request returning JSON with retry
    pub async fn get_json<T: serde::de::DeserializeOwned>(&self, url: &str) -> Result<T> {
        let response = self.get(url).await?;
        let json = response.json().await
            .map_err(|e| NetworkError::DownloadFailed(e.to_string()))?;
        Ok(json)
    }

    /// Execute a request with retry logic
    async fn request_with_retry<F, Fut>(&self, make_request: F) -> Result<Response>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = reqwest::Result<Response>>,
    {
        let mut last_error = None;
        let mut delay = self.retry_delay;

        for attempt in 0..=self.max_retries {
            if attempt > 0 {
                debug!("Retry attempt {} after {:?}", attempt, delay);
                tokio::time::sleep(delay).await;
                delay *= 2; // Exponential backoff
            }

            match make_request().await {
                Ok(response) => {
                    if response.status().is_success() {
                        return Ok(response);
                    }

                    let status = response.status();
                    
                    // Don't retry client errors (4xx) except 429 (rate limit)
                    if status.is_client_error() && status.as_u16() != 429 {
                        return Err(NetworkError::HttpError {
                            status: status.as_u16(),
                            message: status.to_string(),
                        }.into());
                    }

                    // Check for rate limiting
                    if status.as_u16() == 429 {
                        if let Some(retry_after) = response.headers()
                            .get("retry-after")
                            .and_then(|v| v.to_str().ok())
                            .and_then(|s| s.parse::<u64>().ok())
                        {
                            warn!("Rate limited, waiting {}s", retry_after);
                            delay = Duration::from_secs(retry_after);
                        }
                    }

                    last_error = Some(NetworkError::HttpError {
                        status: status.as_u16(),
                        message: status.to_string(),
                    });
                }
                Err(e) => {
                    if e.is_timeout() {
                        last_error = Some(NetworkError::Timeout { timeout_secs: 30 });
                    } else if e.is_connect() {
                        last_error = Some(NetworkError::ConnectionFailed(e.to_string()));
                    } else {
                        last_error = Some(NetworkError::DownloadFailed(e.to_string()));
                    }
                    warn!("Request failed: {}", e);
                }
            }
        }

        Err(last_error.unwrap_or(NetworkError::AllMirrorsFailed).into())
    }

    /// Get the underlying reqwest client
    pub fn inner(&self) -> &Client {
        &self.client
    }
}

impl Default for HttpClient {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let client = HttpClient::new();
        assert_eq!(client.max_retries, 3);
    }

    #[test]
    fn test_client_builder() {
        let client = HttpClient::new()
            .max_retries(5)
            .retry_delay(Duration::from_secs(2));
        
        assert_eq!(client.max_retries, 5);
        assert_eq!(client.retry_delay, Duration::from_secs(2));
    }
}
