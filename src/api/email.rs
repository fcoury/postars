use std::{borrow::Cow, env};

use bitflags::bitflags;
use chrono::{DateTime, Local};
use himalaya_lib::{
    AccountConfig, Backend, Envelope, ImapBackend, ImapBackendBuilder, ImapConfig,
    ShowTextPartsStrategy, Tpl,
};
use serde::{Deserialize, Serialize};

pub struct Server<'a> {
    backend: ImapBackend<'a>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub enum LoadState {
    /// Email wasn't loaded
    #[default]
    None,

    /// Email envelope was loaded
    Partial,

    /// Email body was loaded
    Complete,
}

bitflags! {
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
    pub struct Flags: u32 {
        const SEEN = 1 << 0;
        const ANSWERED = 1 << 1;
        const FLAGGED = 1 << 2;
        const DELETED = 1 << 3;
        const DRAFT = 1 << 4;
        const RECENT = 1 << 5;
        const CUSTOM = 1 << 6; // FIXME: if we need to use custom, we need to make this a struct
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct Email {
    pub state: LoadState,
    pub folder: String,
    pub internal_id: String,
    pub flags: Flags,
    pub date: DateTime<Local>,
    pub from_name: Option<String>,
    pub from_addr: String,
    pub subject: String,
    pub body: Option<String>,
    pub selected: bool,
}

impl From<&Envelope> for Email {
    fn from(envelope: &Envelope) -> Self {
        let envelope = envelope.clone();
        Email {
            state: LoadState::Partial,
            internal_id: envelope.internal_id.clone(),
            date: envelope.date,
            from_name: envelope.from.name.clone(),
            from_addr: envelope.from.addr.clone(),
            subject: envelope.subject.clone(),
            flags: envelope.flags.into(),
            ..Default::default()
        }
    }
}

impl From<himalaya_lib::Flags> for Flags {
    fn from(flags: himalaya_lib::Flags) -> Self {
        let mut result = Flags::empty();

        for flag in flags.0 {
            let bit = match flag {
                himalaya_lib::Flag::Seen => Self::SEEN,
                himalaya_lib::Flag::Answered => Self::ANSWERED,
                himalaya_lib::Flag::Flagged => Self::FLAGGED,
                himalaya_lib::Flag::Deleted => Self::DELETED,
                himalaya_lib::Flag::Draft => Self::DRAFT,
                himalaya_lib::Flag::Recent => Self::RECENT,
                himalaya_lib::Flag::Custom(_) => Self::CUSTOM,
            };
            result |= bit;
        }

        result
    }
}

impl Email {
    #[allow(unused)]
    pub fn hidrate_body(&mut self, server: &Server) -> eyre::Result<()> {
        let body = server.fetch_body(&self.folder, &self.internal_id)?;
        self.body = Some(body);
        self.state = LoadState::Complete;

        Ok(())
    }
}

impl<'a> Server<'a> {
    pub fn new(access_code: String) -> eyre::Result<Self> {
        let account = AccountConfig {
            name: env::var("ACCOUNT_NAME")?,
            email: env::var("ACCOUNT_EMAIL")?,
            ..Default::default()
        };

        let config = ImapConfig {
            host: "outlook.office365.com".to_string(),
            port: 993,
            ssl: Some(true),
            login: env::var("ACCOUNT_EMAIL")?,
            access_token: Some(access_code),
            ..Default::default()
        };

        let imap_backend =
            ImapBackendBuilder::new().build(Cow::Owned(account), Cow::Owned(config))?;

        Ok(Self {
            backend: imap_backend,
        })
    }

    pub fn folders(&self) -> eyre::Result<Vec<String>> {
        let folders = self.backend.list_folders()?;
        Ok(folders.to_vec().iter().map(|f| f.name.clone()).collect())
    }

    pub fn fetch(&self, folder: &str) -> eyre::Result<Vec<Email>> {
        let envelopes = self.backend.list_envelopes(folder, 0, 10)?;
        let emails = envelopes
            .iter()
            .map(|e| {
                let mut email = Email::from(e);
                email.folder = folder.to_string();
                email
            })
            .collect();

        Ok(emails)
    }

    pub fn fetch_body(&self, folder: &str, internal_id: &str) -> eyre::Result<String> {
        let config = AccountConfig {
            email: env::var("ACCOUNT_EMAIL")?,
            ..AccountConfig::default()
        };

        let emails = self.backend.get_emails(folder, vec![internal_id])?;
        let email = emails
            .to_vec()
            .pop()
            .ok_or(eyre::eyre!("Email not found"))?;
        let tpl = email
            .to_read_tpl_builder(&config)?
            .show_headers(config.email_reading_headers())
            .show_text_parts_only(false)
            .sanitize_text_parts(false)
            .use_show_text_parts_strategy(ShowTextPartsStrategy::HtmlOtherwisePlain)
            .build();

        Ok(<Tpl as Into<String>>::into(tpl))
    }

    pub fn move_emails(
        &self,
        from_folder: &str,
        to_folder: &str,
        internal_ids: Vec<&str>,
    ) -> eyre::Result<()> {
        self.backend
            .move_emails(from_folder, to_folder, internal_ids)?;
        Ok(())
    }
}
