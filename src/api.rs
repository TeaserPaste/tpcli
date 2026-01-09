use crate::types::ErrorResponse;
use anyhow::{Result, anyhow};
use log::{debug, error, info, trace};
use serde::Serialize;
use serde::de::DeserializeOwned;

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
        T: Serialize + std::fmt::Debug,
        R: DeserializeOwned,
    {
        let url = format!("{}{}", BASE_API_URL, endpoint);
        info!("API Request: {} {}", method, url);

        let mut request = ureq::request(method, &url);

        request = request.set("Content-Type", "application/json");

        if let Some(token) = &self.token {
            request = request.set("Authorization", &format!("Bearer {}", token));
        }

        let response = if let Some(body) = body {
            debug!("Request Payload: {:?}", body);
            // Also log as JSON string for clearer view if debug is high enough or payload complex
            if log::log_enabled!(log::Level::Trace) {
                 if let Ok(json_str) = serde_json::to_string(body) {
                     trace!("Request JSON: {}", json_str);
                 }
            }
            request.send_json(body)
        } else {
            request.call()
        };

        match response {
            Ok(resp) => {
                let status = resp.status();
                info!("API Response Status: {}", status);
                let text = resp.into_string()?;
                
                debug!("Response Body: {}", text);

                // Try parsing as JSON first
                match serde_json::from_str::<R>(&text) {
                    Ok(data) => Ok(data),
                    Err(e) => {
                        error!("Failed to parse JSON response: {}", e);
                        trace!("Raw response was: {}", text);
                        Err(anyhow!("Failed to parse response: {}", text))
                    }
                }
            }
            Err(ureq::Error::Status(code, response)) => {
                let text = response.into_string()?;
                error!("API Error Response ({}): {}", code, text);
                
                // Try to parse error details
                if let Ok(err_data) = serde_json::from_str::<ErrorResponse>(&text) {
                    if let Some(true) = err_data.requires_password {
                        return Err(anyhow!("Snippet requires a password."));
                    }
                    let details = err_data
                        .error
                        .or(err_data.message)
                        .unwrap_or_else(|| text.clone());
                    return Err(anyhow!("API Error ({}): {}", code, details));
                }
                Err(anyhow!("API Error ({}): {}", code, text))
            }
            Err(e) => {
                error!("Request transport failed: {}", e);
                Err(anyhow!("Request failed: {}", e))
            },
        }
    }
}
