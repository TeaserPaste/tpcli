use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Snippet {
    pub id: String,
    pub title: String,
    pub content: String,
    pub language: String,
    pub visibility: String,
    #[serde(rename = "creatorName")]
    pub creator_name: Option<String>,
    pub tags: Option<Vec<String>>,
    pub views: Option<u32>,
    pub stars: Option<u32>,
    #[serde(rename = "isVerified")]
    pub is_verified: Option<bool>,
    #[serde(rename = "passwordBypassed")]
    pub password_bypassed: Option<bool>,
    pub created_at: Option<String>, // Or DateTime if we parse it
    pub expires_at: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct User {
    #[serde(rename = "userId")]
    pub user_id: String,
    #[serde(rename = "displayName")]
    pub display_name: String,
    #[serde(rename = "photoURL")]
    pub photo_url: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CreateSnippetRequest {
    pub title: String,
    pub content: String,
    pub language: String,
    pub visibility: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CreateSnippetResponse {
    pub id: String,
    pub message: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ListSnippetsRequest {
    pub limit: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub visibility: Option<String>,
    #[serde(rename = "includeDeleted")]
    pub include_deleted: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct UpdateSnippetRequest {
    #[serde(rename = "snippetId")]
    pub snippet_id: String,
    pub updates: serde_json::Value,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SearchSnippetsRequest {
    pub term: String,
    pub size: u32,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SearchSnippetsResponse {
    pub total: u32,
    pub hits: Vec<Snippet>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct StarSnippetResponse {
    pub status: String,
    #[serde(rename = "starCount")]
    pub star_count: u32,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CopySnippetResponse {
    pub message: String,
    #[serde(rename = "newSnippetId")]
    pub new_snippet_id: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RestoreSnippetResponse {
    pub message: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DeleteSnippetResponse {
    pub message: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ErrorResponse {
    pub error: Option<String>,
    pub message: Option<String>,
    #[serde(rename = "requiresPassword")]
    pub requires_password: Option<bool>,
}
