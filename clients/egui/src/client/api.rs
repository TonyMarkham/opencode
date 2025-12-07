use std::path::PathBuf;
use std::time::Duration;

use reqwest::Url;
use serde::{Deserialize, Serialize};
use crate::error::ApiError;

    #[error("invalid base url: {0}")]
    Url(String),
    #[error("http error: {0}")]
    Http(String),
    #[error("decode error: {0}")]
    Decode(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionInfo {
    pub id: String,
    pub title: String,
    pub directory: String,
    #[serde(default)]
    pub time: Option<SessionTime>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionTime {
    pub created: i64,
    pub updated: i64,
}

#[derive(Clone)]
pub struct OpencodeClient {
    base: Url,
    http: reqwest::Client,
    pub directory: Option<PathBuf>,
}

impl OpencodeClient {
    pub fn new(base_url: &str) -> Result<Self, ApiError> {
        let base = Url::parse(base_url).map_err(|e| ApiError::Url(e.to_string()))?;
        Ok(Self {
            base,
            http: reqwest::Client::builder()
                .timeout(Duration::from_secs(30))
                .build()
                .map_err(|e| ApiError::Http(e.to_string()))?,
            directory: None,
        })
    }

    fn with_dir(&self, mut req: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        if let Some(dir) = &self.directory {
            if let Some(d) = dir.to_str() {
                req = req.header("x-opencode-directory", d);
            }
        }
        req
    }

    pub async fn doc(&self) -> Result<String, ApiError> {
        let url = self.base.join("doc").map_err(|e| ApiError::Url(e.to_string()))?;
        let resp = self.http.get(url).send().await.map_err(|e| ApiError::Http(e.to_string()))?;
        let text = resp.text().await.map_err(|e| ApiError::Decode(e.to_string()))?;
        Ok(text)
    }

    pub async fn list_sessions(&self) -> Result<Vec<SessionInfo>, ApiError> {
        let url = self.base.join("session").map_err(|e| ApiError::Url(e.to_string()))?;
        let resp = self.with_dir(self.http.get(url)).send().await.map_err(|e| ApiError::Http(e.to_string()))?;
        let data = resp.json::<Vec<SessionInfo>>().await.map_err(|e| ApiError::Decode(e.to_string()))?;
        Ok(data)
    }
}
