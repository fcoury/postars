use std::borrow::Cow;

use chrono::{DateTime, Local};
use himalaya_lib::{AccountConfig, Backend, Envelope, ImapBackend, ImapBackendBuilder, ImapConfig};
use serde::{Deserialize, Serialize};

pub struct Server {
    access_code: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct Email {
    pub folder: String,
    pub internal_id: String,
    pub date: DateTime<Local>,
    pub from_name: Option<String>,
    pub from_addr: String,
    pub subject: String,
    pub body: Option<String>,
    pub selected: bool,
}

impl Email {
    fn from(folder: &str, envelope: Envelope) -> Self {
        Email {
            folder: folder.to_string(),
            internal_id: envelope.internal_id,
            date: envelope.date,
            from_name: envelope.from.name.clone(),
            from_addr: envelope.from.addr.clone(),
            subject: envelope.subject,
            ..Default::default()
        }
    }
}

impl Server {
    pub fn new(access_code: String) -> Self {
        Self { access_code }
    }

    fn backend<'a>(&self) -> eyre::Result<ImapBackend<'a>> {
        let account = AccountConfig {
            name: "Felipe Coury".to_string(),
            email: "felipe.coury@methodiq.com".to_string(),
            ..Default::default()
        };

        let config = ImapConfig {
            host: "outlook.office365.com".to_string(),
            port: 993,
            ssl: Some(true),
            login: "felipe.coury@methodiq.com".to_string(),
            access_token: Some(self.access_code.clone()),
            ..Default::default()
        };

        Ok(ImapBackendBuilder::new().build(Cow::Owned(account), Cow::Owned(config))?)
    }

    pub fn get_emails(&self) -> eyre::Result<Vec<Email>> {
        let backend = self.backend()?;
        let envelopes = backend.list_envelopes("INBOX", 0, 10)?;
        let emails = envelopes
            .iter()
            .map(|e| Email::from("INBOX", e.clone()))
            .collect();

        Ok(emails)
    }
}
