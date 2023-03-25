use std::collections::HashMap;

use reqwest::Client;
use serde::{de::DeserializeOwned, Deserialize, Deserializer, Serialize};
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
pub struct Profile {
    pub business_phones: Vec<String>,
    pub display_name: String,
    pub given_name: String,
    pub id: String,
    pub job_title: Option<String>,
    pub mail: String,
    pub mobile_phone: Option<String>,
    pub office_location: Option<String>,
    pub preferred_language: Option<String>,
    pub surname: String,
    pub user_principal_name: String,
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
    #[serde(deserialize_with = "deserialize_null_default")]
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
    pub sender: Option<EmailAddressWrapper>,
    pub from: Option<EmailAddressWrapper>,
    pub to_recipients: Vec<EmailAddressWrapper>,
    pub cc_recipients: Vec<EmailAddressWrapper>,
    pub bcc_recipients: Vec<EmailAddressWrapper>,
    pub reply_to: Vec<EmailAddressWrapper>,
    pub flag: Flag,
}

fn deserialize_null_default<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    T: Default + Deserialize<'de>,
    D: Deserializer<'de>,
{
    let opt = Option::deserialize(deserializer)?;
    Ok(opt.unwrap_or_default())
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
        self.fetch_all_items::<Email>(&url).await
    }

    pub async fn get_user_emails_paginated(
        &self,
        initial_page: usize,
        num_pages: usize,
    ) -> Result<(Vec<Email>, bool), GraphClientError> {
        let url = format!("{}/me/messages", GRAPH_API_BASE_URL);
        self.fetch_pages::<Email>(&url, initial_page, num_pages)
            .await
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

    pub async fn get_user_emails_from_folder_by_name(
        &mut self,
        folder_name: &str,
    ) -> Result<Vec<Email>, GraphClientError> {
        let folder_id = self.get_folder_id_by_name(folder_name).await?;
        self.get_user_emails_from_folder(&folder_id).await
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
        let folder_id = self.get_folder_id_by_name(folder_name).await?;
        self.move_email_to_folder(email_id, &folder_id).await
    }

    pub async fn move_emails_to_folder_by_name(
        &mut self,
        email_ids: Vec<String>,
        folder_name: &str,
    ) -> Result<Vec<Email>, GraphClientError> {
        let mut moved_emails = Vec::new();

        for email_id in email_ids {
            let moved_email = self
                .move_email_to_folder_by_name(&email_id, folder_name)
                .await?;
            moved_emails.push(moved_email);
        }

        Ok(moved_emails)
    }

    pub async fn get_user_profile(&self) -> Result<Profile, GraphClientError> {
        let url = format!("{}/me", GRAPH_API_BASE_URL);
        let response = self
            .client
            .get(&url)
            .bearer_auth(&self.access_token)
            .send()
            .await?;

        if response.status().is_success() {
            let json: Profile = response.json().await?;
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

    async fn fetch_pages<T: DeserializeOwned>(
        &self,
        base_url: &str,
        initial_page: usize,
        num_pages: usize,
    ) -> Result<(Vec<T>, bool), GraphClientError> {
        let mut items = Vec::new();
        let mut next_link: Option<String> =
            Some(format!("{}?$skip={}", base_url, initial_page * num_pages));
        let mut pages_fetched = 0;
        let mut has_more_pages = false;

        while let Some(url) = next_link {
            if pages_fetched >= num_pages {
                has_more_pages = true;
                break;
            }

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

                pages_fetched += 1;
            } else {
                return Err(GraphClientError::Request(response.status()));
            }
        }

        Ok((items, has_more_pages))
    }

    async fn get_folder_id_by_name(
        &mut self,
        folder_name: &str,
    ) -> Result<String, GraphClientError> {
        if let Some(folder_id) = self.folder_cache.get(folder_name) {
            return Ok(folder_id.to_string());
        }

        let folders = self.get_user_folders().await?;
        if let Some(folder) = folders
            .into_iter()
            .find(|f| f.display_name.to_lowercase() == folder_name.to_lowercase())
        {
            let folder_id = folder.id;
            self.folder_cache
                .insert(folder_name.to_string(), folder_id.clone());
            Ok(folder_id)
        } else {
            Err(GraphClientError::FolderNotFound(folder_name.to_string()))
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

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

    #[test]
    fn test_parsing() {
        let json = fs::read_to_string("src/fixtures/broken-sender.json").unwrap();
        let json = serde_json::from_str::<Value>(&json).unwrap();
        let emails_value = json["value"].as_array();
        let email: Email = serde_json::from_value(emails_value.unwrap()[0].clone()).unwrap();
        let sender = email.sender.unwrap();
        assert_eq!(sender.email_address.name, "Giuliana Reggi");
    }

    #[test]
    fn test_empty_subject() {
        let json = fs::read_to_string("src/fixtures/empty-subject.json").unwrap();
        let json = serde_json::from_str::<Value>(&json).unwrap();
        let email: Email = serde_json::from_value(json).unwrap();
        let sender = email.sender.unwrap();
        assert_eq!(sender.email_address.name, "Sarah McFarlin");
        assert_eq!(email.subject, "");
    }

    #[test]
    fn test_no_sender_no_from() {
        let json = fs::read_to_string("src/fixtures/no-sender-no-from.json").unwrap();
        let json = serde_json::from_str::<Value>(&json).unwrap();
        let email: Email = serde_json::from_value(json).unwrap();
        assert!(email.sender.is_none());
        assert!(email.from.is_none());
    }
}
