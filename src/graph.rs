use std::collections::HashMap;

use reqwest::Client;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::{json, Value};
use thiserror::Error;

const GRAPH_API_BASE_URL: &str = "https://graph.microsoft.com/v1.0";

#[derive(Error, Debug)]
pub enum GraphClientError {
    #[error("HTTP Request Error: {0}")]
    HttpRequest(#[from] reqwest::Error),

    #[error("Request failed with status: {0}")]
    Request(reqwest::StatusCode),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Failed to parse {0}: {1}")]
    Parse(&'static str, Value),

    #[error("Folder not found: {0}")]
    FolderNotFound(String),
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Folder {
    pub child_folder_count: u32,
    pub display_name: String,
    pub id: String,
    pub is_hidden: bool,
    pub parent_folder_id: String,
    pub size_in_bytes: u64,
    pub total_item_count: u32,
    pub unread_item_count: u32,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Email {
    pub id: String,
    pub created_date_time: String,
    pub last_modified_date_time: String,
    pub received_date_time: String,
    pub sent_date_time: String,
    pub has_attachments: bool,
    pub internet_message_id: String,
    pub subject: String,
    pub body_preview: String,
    pub importance: String,
    pub parent_folder_id: String,
    pub conversation_id: String,
    pub conversation_index: String,
    pub is_delivery_receipt_requested: Option<bool>,
    pub is_read_receipt_requested: bool,
    pub is_read: bool,
    pub is_draft: bool,
    pub web_link: String,
    pub inference_classification: String,
    pub body: Body,
    pub sender: EmailAddressWrapper,
    pub from: EmailAddressWrapper,
    pub to_recipients: Vec<EmailAddressWrapper>,
    pub cc_recipients: Vec<EmailAddressWrapper>,
    pub bcc_recipients: Vec<EmailAddressWrapper>,
    pub reply_to: Vec<EmailAddressWrapper>,
    pub flag: Flag,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Body {
    pub content_type: String,
    pub content: String,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct EmailAddressWrapper {
    pub email_address: EmailAddress,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct EmailAddress {
    pub name: String,
    pub address: String,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Flag {
    pub flag_status: String,
}

pub struct GraphClient {
    client: Client,
    access_token: String,
    folder_cache: HashMap<String, String>,
}

impl GraphClient {
    pub fn new(access_token: String) -> Self {
        let client = Client::new();
        Self {
            client,
            access_token,
            folder_cache: HashMap::new(),
        }
    }

    pub async fn get_user_folders(&self) -> Result<Vec<Folder>, GraphClientError> {
        let url = format!("{}/me/mailFolders", GRAPH_API_BASE_URL);
        self.fetch_all_items::<Folder>(&url).await
    }

    pub async fn get_user_emails(&self) -> Result<Vec<Email>, GraphClientError> {
        let url = format!("{}/me/messages", GRAPH_API_BASE_URL);
        let response = self
            .client
            .get(&url)
            .bearer_auth(&self.access_token)
            .send()
            .await?;

        if response.status().is_success() {
            let json: Value = response.json().await?;
            let emails_value = json["value"]
                .as_array()
                .ok_or_else(|| GraphClientError::Parse("emails", json.clone()))?;

            let emails: Result<Vec<Email>, serde_json::Error> = emails_value
                .iter()
                .map(|email_value| serde_json::from_value(email_value.clone()))
                .collect();

            Ok(emails?)
        } else {
            Err(GraphClientError::Request(response.status()))
        }
    }

    pub async fn get_user_emails_from_folder(
        &self,
        folder_id: &str,
    ) -> Result<Vec<Email>, GraphClientError> {
        let url = format!(
            "{}/me/mailFolders/{}/messages",
            GRAPH_API_BASE_URL, folder_id
        );
        let response = self
            .client
            .get(&url)
            .bearer_auth(&self.access_token)
            .send()
            .await?;

        if response.status().is_success() {
            let json: Value = response.json().await?;
            let emails_value = json["value"]
                .as_array()
                .ok_or_else(|| GraphClientError::Parse("emails", json.clone()))?;

            let emails: Result<Vec<Email>, serde_json::Error> = emails_value
                .iter()
                .map(|email_value| serde_json::from_value(email_value.clone()))
                .collect();

            Ok(emails?)
        } else {
            Err(GraphClientError::Request(response.status()))
        }
    }

    pub async fn get_email_by_id(&self, email_id: &str) -> Result<Email, GraphClientError> {
        let url = format!("{}/me/messages/{}", GRAPH_API_BASE_URL, email_id);
        let response = self
            .client
            .get(&url)
            .bearer_auth(&self.access_token)
            .send()
            .await?;

        if response.status().is_success() {
            let email: Email = response.json().await?;

            Ok(email)
        } else {
            Err(GraphClientError::Request(response.status()))
        }
    }

    pub async fn move_email_to_folder(
        &self,
        email_id: &str,
        folder_id: &str,
    ) -> Result<Email, GraphClientError> {
        let url = format!("{}/me/messages/{}/move", GRAPH_API_BASE_URL, email_id);
        let payload = json!({ "destinationId": folder_id });

        let response = self
            .client
            .post(&url)
            .bearer_auth(&self.access_token)
            .json(&payload)
            .send()
            .await?;

        if response.status().is_success() {
            let email: Email = response.json().await?;
            Ok(email)
        } else {
            Err(GraphClientError::Request(response.status()))
        }
    }

    pub async fn move_email_to_folder_by_name(
        &mut self,
        email_id: &str,
        folder_name: &str,
    ) -> Result<Email, GraphClientError> {
        let folder_id = match self.folder_cache.get(folder_name) {
            Some(folder_id) => folder_id.to_string(),
            None => {
                let folders = self.get_user_folders().await?;
                if let Some(folder) = folders
                    .into_iter()
                    .find(|f| f.display_name.to_lowercase() == folder_name.to_lowercase())
                {
                    let folder_id = folder.id;
                    self.folder_cache
                        .insert(folder_name.to_string(), folder_id.clone());
                    folder_id
                } else {
                    return Err(GraphClientError::FolderNotFound(folder_name.to_string()));
                }
            }
        };

        self.move_email_to_folder(email_id, &folder_id).await
    }

    pub async fn get_user_profile(&self) -> Result<Value, GraphClientError> {
        let url = format!("{}/me", GRAPH_API_BASE_URL);
        let response = self
            .client
            .get(&url)
            .bearer_auth(&self.access_token)
            .send()
            .await?;

        if response.status().is_success() {
            let json: Value = response.json().await?;
            Ok(json)
        } else {
            Err(GraphClientError::Request(response.status()))
        }
    }

    async fn fetch_all_items<T: DeserializeOwned>(
        &self,
        base_url: &str,
    ) -> Result<Vec<T>, GraphClientError> {
        let mut items = Vec::new();
        let mut next_link: Option<String> = Some(base_url.to_string());

        while let Some(url) = next_link {
            let response = self
                .client
                .get(&url)
                .bearer_auth(&self.access_token)
                .send()
                .await?;

            if response.status().is_success() {
                let json: Value = response.json().await?;
                let item_values = json["value"]
                    .as_array()
                    .ok_or_else(|| GraphClientError::Parse("items", json.clone()))?;

                let deserialized_items: Vec<T> = item_values
                    .iter()
                    .map(|item_value| serde_json::from_value(item_value.clone()))
                    .collect::<Result<Vec<T>, _>>()?;

                items.extend(deserialized_items);

                next_link = json["@odata.nextLink"]
                    .as_str()
                    .map(|link| link.to_string());
            } else {
                return Err(GraphClientError::Request(response.status()));
            }
        }

        Ok(items)
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    #[test]
    fn test_parsing() {
        let json = fs::read_to_string("src/fixtures/broken-sender.json").unwrap();
        let json = serde_json::from_str::<Value>(&json).unwrap();
        let emails_value = json["value"].as_array();
        let email: Email = serde_json::from_value(emails_value.unwrap()[0].clone()).unwrap();
        assert_eq!(email.sender.email_address.name, "Giuliana Reggi");
    }

    #[test]
    fn test_body() {
        let body = r#"
           {
                "contentType": "html",
                "content": "<html><head>"
            }
        "#;
        let body: Body = serde_json::from_str(body).unwrap();
        assert_eq!(body.content_type, "html");
    }
}
