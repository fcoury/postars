use reqwest::Client;
use serde_json::Value;
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
}

use serde::{Deserialize, Serialize};

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
}

impl GraphClient {
    pub fn new(access_token: String) -> Self {
        let client = Client::new();
        Self {
            client,
            access_token,
        }
    }

    pub async fn get_user_folders(&self) -> Result<Vec<(String, String)>, GraphClientError> {
        let url = format!("{}/me/mailFolders", GRAPH_API_BASE_URL);
        let response = self
            .client
            .get(&url)
            .bearer_auth(&self.access_token)
            .send()
            .await?;

        if response.status().is_success() {
            let json: Value = response.json().await?;
            let folders = json["value"]
                .as_array()
                .ok_or(GraphClientError::Parse("folders", json.clone()))?
                .iter()
                .filter_map(|folder| {
                    let id = folder["id"].as_str()?.to_string();
                    let display_name = folder["displayName"].as_str()?.to_string();
                    Some((id, display_name))
                })
                .collect();
            Ok(folders)
        } else {
            Err(GraphClientError::Request(response.status()))
        }
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

            match emails {
                Ok(emails) => Ok(emails),
                Err(_) => Err(GraphClientError::Parse("emails", json.clone())),
            }
        } else {
            Err(GraphClientError::Request(response.status()))
        }
    }

    pub async fn get_user_emails_from_folder(
        &self,
        folder_id: &str,
    ) -> Result<Vec<String>, GraphClientError> {
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
            let emails = json["value"]
                .as_array()
                .ok_or(GraphClientError::Parse("emails", json.clone()))?
                .iter()
                .filter_map(|email| email["subject"].as_str().map(|s| s.to_string()))
                .collect();
            Ok(emails)
        } else {
            Err(GraphClientError::Request(response.status()))
        }
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
