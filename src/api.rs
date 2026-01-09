use ureq;
use serde::de::DeserializeOwned;
use serde::Serialize;
use anyhow::{Result, anyhow};
use crate::types::ErrorResponse;

const BASE_API_URL: &str = "https://paste-api.teaserverse.online";

pub struct ApiClient {
    token: Option<String>,
}

impl ApiClient {
    pub fn new(token: Option<String>) -> Self {
        Self { token }
    }

    pub fn request<T, R>(&self, endpoint: &str, method: &str, body: Option<&T>) -> Result<R>
    where
        T: Serialize,
        R: DeserializeOwned,
    {
        let url = format!("{}{}", BASE_API_URL, endpoint);
        let mut request = ureq::request(method, &url);
        
        request = request.set("Content-Type", "application/json");
        
        if let Some(token) = &self.token {
            request = request.set("Authorization", &format!("Bearer {}", token));
        }

        let response = if let Some(body) = body {
            request.send_json(body)
        } else {
            request.call()
        };

        match response {
            Ok(resp) => {
                let text = resp.into_string()?;
                // Try parsing as JSON first
                match serde_json::from_str::<R>(&text) {
                    Ok(data) => Ok(data),
                    Err(_) => {
                         // Sometimes it might return a string that is not valid JSON but expected type is String?
                         // But R is generic. If R is String, we can try to return text.
                         // However, serde_json::from_str should handle strings if they are quoted.
                         // If the API returns raw string not quoted, that's an issue if R expects a struct.
                         // For now assume API always returns JSON.
                         Err(anyhow!("Failed to parse response: {}", text))
                    }
                }
            },
            Err(ureq::Error::Status(code, response)) => {
                let text = response.into_string()?;
                // Try to parse error details
                if let Ok(err_data) = serde_json::from_str::<ErrorResponse>(&text) {
                     if let Some(true) = err_data.requires_password {
                         return Err(anyhow!("Snippet requires a password."));
                     }
                     let details = err_data.error.or(err_data.message).unwrap_or_else(|| text.clone());
                     return Err(anyhow!("API Error ({}): {}", code, details));
                }
                Err(anyhow!("API Error ({}): {}", code, text))
            },
            Err(e) => Err(anyhow!("Request failed: {}", e)),
        }
    }
}
