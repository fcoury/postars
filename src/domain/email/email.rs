// himalaya-lib, a Rust library for email management.
// Copyright (C) 2022  soywod <clement.douin@posteo.net>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

use ammonia;
use chrono::{DateTime, Local, TimeZone, Utc};
use convert_case::{Case, Casing};
use html_escape;
use lettre::message::{header::ContentType, Attachment, MultiPart, SinglePart};
use log::{info, trace, warn};
use regex::Regex;
use std::{
    collections::{HashMap, HashSet},
    convert::TryInto,
    env::{self, temp_dir},
    fmt::Debug,
    fs, io,
    path::PathBuf,
    result,
};
use thiserror::Error;
use tree_magic;
use uuid::Uuid;

use crate::{
    config, from_addrs_to_sendable_addrs, from_addrs_to_sendable_mbox, from_slice_to_addrs,
    AccountConfig, Addr, Addrs, BinaryPart, Part, Parts, TextPlainPart, TplOverride,
    DEFAULT_SIGNATURE_DELIM,
};

#[derive(Error, Debug)]
pub enum Error {
    #[error("cannot expand attachment path {1}")]
    ExpandAttachmentPathError(#[source] shellexpand::LookupError<env::VarError>, String),
    #[error("cannot read attachment at {1}")]
    ReadAttachmentError(#[source] io::Error, PathBuf),
    #[error("cannot parse template")]
    ParseTplError(#[source] mailparse::MailParseError),
    #[error("cannot parse content type of attachment {1}")]
    ParseAttachmentContentTypeError(#[source] lettre::message::header::ContentTypeErr, String),
    #[error("cannot write temporary multipart on the disk")]
    WriteTmpMultipartError(#[source] io::Error),
    #[error("cannot write temporary multipart on the disk")]
    BuildSendableMsgError(#[source] lettre::error::Error),
    #[error("cannot parse {1} value: {2}")]
    ParseHeaderError(#[source] mailparse::MailParseError, String, String),
    #[error("cannot build envelope")]
    BuildEnvelopeError(#[source] lettre::error::Error),
    #[error("cannot get file name of attachment {0}")]
    GetAttachmentFilenameError(PathBuf),
    #[error("cannot parse recipient")]
    ParseRecipientError,

    #[error("cannot parse message or address")]
    ParseAddressError(#[from] lettre::address::AddressError),

    #[error(transparent)]
    ConfigError(#[from] config::Error),

    #[error("cannot get content type of multipart")]
    GetMultipartContentTypeError,
    #[error("cannot find encrypted part of multipart")]
    GetEncryptedPartMultipartError,
    #[error("cannot parse encrypted part of multipart")]
    ParseEncryptedPartError(#[source] mailparse::MailParseError),
    #[error("cannot get body from encrypted part")]
    GetEncryptedPartBodyError(#[source] mailparse::MailParseError),
    #[error("cannot write encrypted part to temporary file")]
    WriteEncryptedPartBodyError(#[source] io::Error),
    #[error("cannot write encrypted part to temporary file")]
    DecryptPartError(#[source] config::Error),

    #[error("cannot delete local draft: {1}")]
    DeleteLocalDraftError(#[source] io::Error, PathBuf),
}

pub type Result<T> = result::Result<T, Error>;

/// Representation of a message.
#[derive(Debug, Clone, Default)]
pub struct Email {
    pub id: u32,
    pub subject: String,
    pub from: Option<Addrs>,
    pub reply_to: Option<Addrs>,
    pub to: Option<Addrs>,
    pub cc: Option<Addrs>,
    pub bcc: Option<Addrs>,
    pub in_reply_to: Option<String>,
    pub message_id: Option<String>,
    pub headers: HashMap<String, String>,
    pub date: Option<DateTime<Local>>,
    pub parts: Parts,
    pub encrypt: bool,
    pub raw: Vec<u8>,
}

impl Email {
    pub fn attachments(&self) -> Vec<BinaryPart> {
        self.parts
            .iter()
            .filter_map(|part| match part {
                Part::Binary(part) => Some(part.to_owned()),
                _ => None,
            })
            .collect()
    }

    /// Folds string body from all plain text parts into a single
    /// string body. If no plain text parts are found, HTML parts are
    /// used instead. The result is sanitized (all HTML markup is
    /// removed).
    pub fn fold_text_plain_parts(&self) -> String {
        let (plain, html) = self.parts.iter().fold(
            (String::default(), String::default()),
            |(mut plain, mut html), part| {
                match part {
                    Part::TextPlain(part) => {
                        let glue = if plain.is_empty() { "" } else { "\n\n" };
                        plain.push_str(glue);
                        plain.push_str(&part.content);
                    }
                    Part::TextHtml(part) => {
                        let glue = if html.is_empty() { "" } else { "\n\n" };
                        html.push_str(glue);
                        html.push_str(&part.content);
                    }
                    _ => (),
                };
                (plain, html)
            },
        );
        if plain.is_empty() {
            // Remove HTML markup
            let sanitized_html = ammonia::Builder::new()
                .tags(HashSet::default())
                .clean(&html)
                .to_string();
            // Merge new line chars
            let sanitized_html = Regex::new(r"(\r?\n\s*){2,}")
                .unwrap()
                .replace_all(&sanitized_html, "\n\n")
                .to_string();
            // Replace tabulations and &npsp; by spaces
            let sanitized_html = Regex::new(r"(\t|&nbsp;)")
                .unwrap()
                .replace_all(&sanitized_html, " ")
                .to_string();
            // Merge spaces
            let sanitized_html = Regex::new(r" {2,}")
                .unwrap()
                .replace_all(&sanitized_html, "  ")
                .to_string();
            // Decode HTML entities
            let sanitized_html = html_escape::decode_html_entities(&sanitized_html).to_string();

            sanitized_html
        } else {
            // Merge new line chars
            let sanitized_plain = Regex::new(r"(\r?\n\s*){2,}")
                .unwrap()
                .replace_all(&plain, "\n\n")
                .to_string();
            // Replace tabulations by spaces
            let sanitized_plain = Regex::new(r"\t")
                .unwrap()
                .replace_all(&sanitized_plain, " ")
                .to_string();
            // Merge spaces
            let sanitized_plain = Regex::new(r" {2,}")
                .unwrap()
                .replace_all(&sanitized_plain, "  ")
                .to_string();

            sanitized_plain
        }
    }

    /// Fold string body from all HTML parts into a single string
    /// body.
    fn fold_text_html_parts(&self) -> String {
        let text_parts = self
            .parts
            .iter()
            .filter_map(|part| match part {
                Part::TextHtml(part) => Some(part.content.to_owned()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("\n\n");
        let text_parts = Regex::new(r"(\r?\n){2,}")
            .unwrap()
            .replace_all(&text_parts, "\n\n")
            .to_string();
        text_parts
    }

    /// Fold string body from all text parts into a single string
    /// body. The mime allows users to choose between plain text parts
    /// and html text parts.
    pub fn fold_text_parts(&self, text_mime: &str) -> String {
        if text_mime == "html" {
            self.fold_text_html_parts()
        } else {
            self.fold_text_plain_parts()
        }
    }

    pub fn into_reply(mut self, all: bool, config: &AccountConfig) -> Result<Self> {
        let account_addr = config.address()?;

        // In-Reply-To
        self.in_reply_to = self.message_id.to_owned();

        // Message-Id
        self.message_id = None;

        // To
        let addrs = self
            .reply_to
            .as_deref()
            .or_else(|| self.from.as_deref())
            .map(|addrs| {
                addrs.iter().cloned().filter(|addr| match addr {
                    Addr::Group(_) => false,
                    Addr::Single(a) => match &account_addr {
                        Addr::Group(_) => false,
                        Addr::Single(b) => a.addr != b.addr,
                    },
                })
            });
        if all {
            self.to = addrs.map(|addrs| addrs.collect::<Vec<_>>().into());
        } else {
            self.to = addrs
                .and_then(|mut addrs| addrs.next())
                .map(|addr| vec![addr].into());
        }

        // Cc
        self.cc = if all {
            self.cc.as_deref().map(|addrs| {
                addrs
                    .iter()
                    .cloned()
                    .filter(|addr| match addr {
                        Addr::Group(_) => false,
                        Addr::Single(a) => match &account_addr {
                            Addr::Group(_) => false,
                            Addr::Single(b) => a.addr != b.addr,
                        },
                    })
                    .collect::<Vec<_>>()
                    .into()
            })
        } else {
            None
        };

        // Bcc
        self.bcc = None;

        // Body
        let plain_content = {
            let date = self
                .date
                .as_ref()
                .map(|date| date.format("%d %b %Y, at %H:%M (%z)").to_string())
                .unwrap_or_else(|| "unknown date".into());
            let sender = self
                .reply_to
                .as_ref()
                .or_else(|| self.from.as_ref())
                .and_then(|addrs| addrs.clone().extract_single_info())
                .map(|addr| addr.display_name.clone().unwrap_or_else(|| addr.addr))
                .unwrap_or_else(|| "unknown sender".into());
            let mut content = format!("\n\nOn {}, {} wrote:\n", date, sender);

            let mut glue = "";
            for line in self.fold_text_parts("plain").trim().lines() {
                if line == DEFAULT_SIGNATURE_DELIM {
                    break;
                }
                content.push_str(glue);
                content.push('>');
                content.push_str(if line.starts_with('>') { "" } else { " " });
                content.push_str(line);
                glue = "\n";
            }

            content
        };

        self.parts = Parts(vec![Part::new_text_plain(plain_content)]);

        // Subject
        if !self.subject.starts_with("Re:") {
            self.subject = format!("Re: {}", self.subject);
        }

        // From
        self.from = Some(vec![account_addr.clone()].into());

        Ok(self)
    }

    pub fn into_forward(mut self, config: &AccountConfig) -> Result<Self> {
        let account_addr = config.address()?;

        let prev_subject = self.subject.to_owned();
        let prev_date = self.date.to_owned();
        let prev_from = self.reply_to.to_owned().or_else(|| self.from.to_owned());
        let prev_to = self.to.to_owned();

        // Message-Id
        self.message_id = None;

        // In-Reply-To
        self.in_reply_to = None;

        // From
        self.from = Some(vec![account_addr].into());

        // To
        self.to = Some(vec![].into());

        // Cc
        self.cc = None;

        // Bcc
        self.bcc = None;

        // Subject
        if !self.subject.starts_with("Fwd:") {
            self.subject = format!("Fwd: {}", self.subject);
        }

        // Body
        let mut content = String::default();
        content.push_str("\n\n-------- Forwarded Message --------\n");
        content.push_str(&format!("Subject: {}\n", prev_subject));
        if let Some(date) = prev_date {
            content.push_str(&format!("Date: {}\n", date.to_rfc2822()));
        }
        if let Some(addrs) = prev_from.as_ref() {
            content.push_str("From: ");
            content.push_str(&addrs.to_string());
            content.push('\n');
        }
        if let Some(addrs) = prev_to.as_ref() {
            content.push_str("To: ");
            content.push_str(&addrs.to_string());
            content.push('\n');
        }
        content.push('\n');
        content.push_str(&self.fold_text_parts("plain"));
        self.parts
            .replace_text_plain_parts_with(TextPlainPart { content });

        Ok(self)
    }

    pub fn encrypt(mut self, encrypt: bool) -> Self {
        self.encrypt = encrypt;
        self
    }

    pub fn add_attachments(mut self, attachments_paths: Vec<&str>) -> Result<Self> {
        for path in attachments_paths {
            let path = shellexpand::full(path)
                .map_err(|err| Error::ExpandAttachmentPathError(err, path.to_owned()))?;
            let path = PathBuf::from(path.to_string());
            let filename: String = path
                .file_name()
                .ok_or_else(|| Error::GetAttachmentFilenameError(path.to_owned()))?
                .to_string_lossy()
                .into();
            let content =
                fs::read(&path).map_err(|err| Error::ReadAttachmentError(err, path.to_owned()))?;
            let mime = tree_magic::from_u8(&content);

            self.parts.push(Part::Binary(BinaryPart {
                filename,
                mime,
                content,
            }))
        }

        Ok(self)
    }

    pub fn merge_with(&mut self, email: Email) {
        self.from = email.from;
        self.reply_to = email.reply_to;
        self.to = email.to;
        self.cc = email.cc;
        self.bcc = email.bcc;
        self.subject = email.subject;

        if email.message_id.is_some() {
            self.message_id = email.message_id;
        }

        if email.in_reply_to.is_some() {
            self.in_reply_to = email.in_reply_to;
        }

        for part in email.parts.0.into_iter() {
            match part {
                Part::Binary(_) => self.parts.push(part),
                Part::TextPlain(_) => {
                    self.parts.retain(|p| !matches!(p, Part::TextPlain(_)));
                    self.parts.push(part);
                }
                Part::TextHtml(_) => {
                    self.parts.retain(|p| !matches!(p, Part::TextHtml(_)));
                    self.parts.push(part);
                }
            }
        }
    }

    pub fn to_tpl(&self, opts: TplOverride, config: &AccountConfig) -> Result<String> {
        let account_addr: Addrs = vec![config.address()?].into();
        let mut tpl = String::default();

        tpl.push_str("Content-Type: text/plain; charset=utf-8\n");

        if let Some(in_reply_to) = self.in_reply_to.as_ref() {
            tpl.push_str(&format!("In-Reply-To: {}\n", in_reply_to))
        }

        // From
        tpl.push_str(&format!(
            "From: {}\n",
            opts.from
                .map(|addrs| addrs.join(", "))
                .unwrap_or_else(|| account_addr.to_string())
        ));

        // To
        tpl.push_str(&format!(
            "To: {}\n",
            opts.to
                .map(|addrs| addrs.join(", "))
                .or_else(|| self.to.clone().map(|addrs| addrs.to_string()))
                .unwrap_or_default()
        ));

        // Cc
        if let Some(addrs) = opts
            .cc
            .map(|addrs| addrs.join(", "))
            .or_else(|| self.cc.clone().map(|addrs| addrs.to_string()))
        {
            tpl.push_str(&format!("Cc: {}\n", addrs));
        }

        // Bcc
        if let Some(addrs) = opts
            .bcc
            .map(|addrs| addrs.join(", "))
            .or_else(|| self.bcc.clone().map(|addrs| addrs.to_string()))
        {
            tpl.push_str(&format!("Bcc: {}\n", addrs));
        }

        // Subject
        tpl.push_str(&format!(
            "Subject: {}\n",
            opts.subject.unwrap_or(&self.subject)
        ));

        // Headers <=> body separator
        tpl.push('\n');

        // Body
        if let Some(body) = opts.body {
            tpl.push_str(body);
        } else {
            tpl.push_str(&self.fold_text_plain_parts())
        }

        // Signature
        if let Some(sig) = opts.sig {
            tpl.push_str("\n\n");
            tpl.push_str(sig);
        } else if let Some(ref sig) = config.signature()? {
            tpl.push_str("\n\n");
            tpl.push_str(sig);
        }

        tpl.push('\n');

        trace!("template: {:?}", tpl);
        Ok(tpl)
    }

    pub fn from_tpl(tpl: &str) -> Result<Self> {
        info!("begin: building message from template");
        trace!("template: {:?}", tpl);

        let parsed_mail = mailparse::parse_mail(tpl.as_bytes()).map_err(Error::ParseTplError)?;

        info!("end: building message from template");
        Self::from_parsed_mail(parsed_mail, &AccountConfig::default())
    }

    pub fn into_sendable_msg(&self, config: &AccountConfig) -> Result<lettre::Message> {
        let mut msg_builder = lettre::Message::builder()
            .message_id(self.message_id.to_owned())
            .subject(self.subject.to_owned());

        if let Some(id) = self.in_reply_to.as_ref() {
            msg_builder = msg_builder.in_reply_to(id.to_owned());
        };

        if let Some(addrs) = self.from.as_ref() {
            for addr in from_addrs_to_sendable_mbox(addrs)? {
                msg_builder = msg_builder.from(addr)
            }
        };

        if let Some(addrs) = self.to.as_ref() {
            for addr in from_addrs_to_sendable_mbox(addrs)? {
                msg_builder = msg_builder.to(addr)
            }
        };

        if let Some(addrs) = self.reply_to.as_ref() {
            for addr in from_addrs_to_sendable_mbox(addrs)? {
                msg_builder = msg_builder.reply_to(addr)
            }
        };

        if let Some(addrs) = self.cc.as_ref() {
            for addr in from_addrs_to_sendable_mbox(addrs)? {
                msg_builder = msg_builder.cc(addr)
            }
        };

        if let Some(addrs) = self.bcc.as_ref() {
            for addr in from_addrs_to_sendable_mbox(addrs)? {
                msg_builder = msg_builder.bcc(addr)
            }
        };

        let mut multipart = {
            let mut multipart =
                MultiPart::mixed().singlepart(SinglePart::plain(self.fold_text_plain_parts()));
            for part in self.attachments() {
                multipart = multipart.singlepart(Attachment::new(part.filename.clone()).body(
                    part.content,
                    part.mime.parse().map_err(|err| {
                        Error::ParseAttachmentContentTypeError(err, part.filename)
                    })?,
                ))
            }
            multipart
        };

        if self.encrypt {
            let multipart_buffer = temp_dir().join(Uuid::new_v4().to_string());
            fs::write(multipart_buffer.clone(), multipart.formatted())
                .map_err(Error::WriteTmpMultipartError)?;
            let addr = self
                .to
                .as_ref()
                .and_then(|addrs| addrs.clone().extract_single_info())
                .map(|addr| addr.addr)
                .ok_or_else(|| Error::ParseRecipientError)?;
            let encrypted_multipart = config.pgp_encrypt_file(&addr, multipart_buffer.clone())?;
            trace!("encrypted multipart: {:#?}", encrypted_multipart);
            multipart = MultiPart::encrypted(String::from("application/pgp-encrypted"))
                .singlepart(
                    SinglePart::builder()
                        .header(ContentType::parse("application/pgp-encrypted").unwrap())
                        .body(String::from("Version: 1")),
                )
                .singlepart(
                    SinglePart::builder()
                        .header(ContentType::parse("application/octet-stream").unwrap())
                        .body(encrypted_multipart),
                )
        }

        msg_builder
            .multipart(multipart)
            .map_err(Error::BuildSendableMsgError)
    }

    pub fn from_parsed_mail(
        parsed_mail: mailparse::ParsedMail<'_>,
        config: &AccountConfig,
    ) -> Result<Self> {
        trace!(">> build message from parsed mail");
        trace!("parsed mail: {:?}", parsed_mail);

        let mut email = Email::default();
        for header in parsed_mail.get_headers() {
            trace!(">> parse header {:?}", header);

            let key = header.get_key();
            trace!("header key: {:?}", key);

            let val = header.get_value();
            trace!("header value: {:?}", val);

            match key.to_lowercase().as_str() {
                "message-id" => email.message_id = Some(val),
                "in-reply-to" => email.in_reply_to = Some(val),
                "subject" => {
                    email.subject = val;
                }
                "date" => match mailparse::dateparse(&val) {
                    Ok(timestamp) => {
                        email.date = Some(Utc.timestamp(timestamp, 0).with_timezone(&Local))
                    }
                    Err(err) => {
                        warn!("cannot parse message date {:?}, skipping it", val);
                        warn!("{}", err);
                    }
                },
                "from" => {
                    email.from = from_slice_to_addrs(&val)
                        .map_err(|err| Error::ParseHeaderError(err, key, val.to_owned()))?
                }
                "to" => {
                    email.to = from_slice_to_addrs(&val)
                        .map_err(|err| Error::ParseHeaderError(err, key, val.to_owned()))?
                }
                "reply-to" => {
                    email.reply_to = from_slice_to_addrs(&val)
                        .map_err(|err| Error::ParseHeaderError(err, key, val.to_owned()))?
                }
                "cc" => {
                    email.cc = from_slice_to_addrs(&val)
                        .map_err(|err| Error::ParseHeaderError(err, key, val.to_owned()))?
                }
                "bcc" => {
                    email.bcc = from_slice_to_addrs(&val)
                        .map_err(|err| Error::ParseHeaderError(err, key, val.to_owned()))?
                }
                key => {
                    email.headers.insert(key.to_lowercase(), val);
                }
            }
            trace!("<< parse header");
        }

        email.parts = Parts::from_parsed_mail(config, &parsed_mail)?;
        trace!("message: {:?}", email);

        info!("<< build message from parsed mail");
        Ok(email)
    }

    /// Transforms a message into a readable string. A readable
    /// message is like a template, except that:
    ///  - headers part is customizable (can be omitted if empty filter given in argument)
    ///  - body type is customizable (plain or html)
    pub fn to_readable_string(
        &self,
        text_mime: &str,
        headers: Vec<&str>,
        config: &AccountConfig,
    ) -> Result<String> {
        let mut all_headers = vec![];
        for h in config.email_reading_headers().iter() {
            let h = h.to_lowercase();
            if !all_headers.contains(&h) {
                all_headers.push(h)
            }
        }
        for h in headers.iter() {
            let h = h.to_lowercase();
            if !all_headers.contains(&h) {
                all_headers.push(h)
            }
        }

        let mut readable_email = String::new();
        for h in all_headers {
            match h.as_str() {
                "message-id" => match self.message_id {
                    Some(ref message_id) if !message_id.is_empty() => {
                        readable_email.push_str(&format!("Message-Id: {}\n", message_id));
                    }
                    _ => (),
                },
                "in-reply-to" => match self.in_reply_to {
                    Some(ref in_reply_to) if !in_reply_to.is_empty() => {
                        readable_email.push_str(&format!("In-Reply-To: {}\n", in_reply_to));
                    }
                    _ => (),
                },
                "subject" => {
                    readable_email.push_str(&format!("Subject: {}\n", self.subject));
                }
                "date" => {
                    if let Some(ref date) = self.date {
                        readable_email.push_str(&format!("Date: {}\n", date.to_rfc2822()));
                    }
                }
                "from" => match self.from {
                    Some(ref addrs) if !addrs.is_empty() => {
                        readable_email.push_str(&format!("From: {}\n", addrs));
                    }
                    _ => (),
                },
                "to" => match self.to {
                    Some(ref addrs) if !addrs.is_empty() => {
                        readable_email.push_str(&format!("To: {}\n", addrs));
                    }
                    _ => (),
                },
                "reply-to" => match self.reply_to {
                    Some(ref addrs) if !addrs.is_empty() => {
                        readable_email.push_str(&format!("Reply-To: {}\n", addrs));
                    }
                    _ => (),
                },
                "cc" => match self.cc {
                    Some(ref addrs) if !addrs.is_empty() => {
                        readable_email.push_str(&format!("Cc: {}\n", addrs));
                    }
                    _ => (),
                },
                "bcc" => match self.bcc {
                    Some(ref addrs) if !addrs.is_empty() => {
                        readable_email.push_str(&format!("Bcc: {}\n", addrs));
                    }
                    _ => (),
                },
                key => match self.headers.get(key) {
                    Some(ref val) if !val.is_empty() => {
                        readable_email.push_str(&format!(
                            "{}: {}\n",
                            key.to_case(Case::Train),
                            val
                        ));
                    }
                    _ => (),
                },
            };
        }

        if !readable_email.is_empty() {
            readable_email.push_str("\n");
        }

        readable_email.push_str(&self.fold_text_parts(text_mime));
        Ok(readable_email)
    }
}

#[cfg(feature = "internal-sender")]
impl TryInto<lettre::address::Envelope> for Email {
    type Error = Error;

    fn try_into(self) -> Result<lettre::address::Envelope> {
        (&self).try_into()
    }
}

#[cfg(feature = "internal-sender")]
impl TryInto<lettre::address::Envelope> for &Email {
    type Error = Error;

    fn try_into(self) -> Result<lettre::address::Envelope> {
        let from = match self
            .from
            .as_ref()
            .and_then(|addrs| addrs.clone().extract_single_info())
        {
            Some(addr) => addr.addr.parse().map(Some),
            None => Ok(None),
        }?;
        let to = self
            .to
            .as_ref()
            .map(from_addrs_to_sendable_addrs)
            .unwrap_or(Ok(vec![]))?;
        Ok(lettre::address::Envelope::new(from, to).map_err(Error::BuildEnvelopeError)?)
    }
}

#[cfg(test)]
mod tests {
    use mailparse::SingleInfo;
    use std::iter::FromIterator;

    use crate::{config::AccountConfig, email::Addr};

    use super::*;

    #[test]
    fn test_into_reply() {
        let config = AccountConfig {
            display_name: Some("Test".into()),
            email: "test-account@local".into(),
            ..AccountConfig::default()
        };

        // Checks that:
        //  - "message_id" moves to "in_reply_to"
        //  - "subject" starts by "Re: "
        //  - "to" is replaced by "from"
        //  - "from" is replaced by the address from the account config

        let email = Email {
            message_id: Some("email-id".into()),
            subject: "subject".into(),
            from: Some(
                vec![Addr::Single(SingleInfo {
                    addr: "test-sender@local".into(),
                    display_name: None,
                })]
                .into(),
            ),
            ..Email::default()
        }
        .into_reply(false, &config)
        .unwrap();

        assert_eq!(email.message_id, None);
        assert_eq!(email.in_reply_to.unwrap(), "email-id");
        assert_eq!(email.subject, "Re: subject");
        assert_eq!(
            email.from.unwrap().to_string(),
            "\"Test\" <test-account@local>"
        );
        assert_eq!(email.to.unwrap().to_string(), "test-sender@local");

        // Checks that:
        //  - "subject" does not contains additional "Re: "
        //  - "to" is replaced by reply_to
        //  - "to" contains one address when "all" is false
        //  - "cc" are empty when "all" is false

        let email = Email {
            subject: "Re: subject".into(),
            from: Some(
                vec![Addr::Single(SingleInfo {
                    addr: "test-sender@local".into(),
                    display_name: None,
                })]
                .into(),
            ),
            reply_to: Some(
                vec![
                    Addr::Single(SingleInfo {
                        addr: "test-sender-to-reply@local".into(),
                        display_name: Some("Sender".into()),
                    }),
                    Addr::Single(SingleInfo {
                        addr: "test-sender-to-reply-2@local".into(),
                        display_name: Some("Sender 2".into()),
                    }),
                ]
                .into(),
            ),
            cc: Some(
                vec![Addr::Single(SingleInfo {
                    addr: "test-cc@local".into(),
                    display_name: None,
                })]
                .into(),
            ),
            ..Email::default()
        }
        .into_reply(false, &config)
        .unwrap();

        assert_eq!(email.subject, "Re: subject");
        assert_eq!(
            email.to.unwrap().to_string(),
            "\"Sender\" <test-sender-to-reply@local>"
        );
        assert_eq!(email.cc, None);

        // Checks that:
        //  - "to" contains all addresses except for the sender when "all" is true
        //  - "cc" contains all addresses except for the sender when "all" is true

        let email = Email {
            from: Some(
                vec![
                    Addr::Single(SingleInfo {
                        addr: "test-sender-1@local".into(),
                        display_name: Some("Sender 1".into()),
                    }),
                    Addr::Single(SingleInfo {
                        addr: "test-sender-2@local".into(),
                        display_name: Some("Sender 2".into()),
                    }),
                    Addr::Single(SingleInfo {
                        addr: "test-account@local".into(),
                        display_name: Some("Test".into()),
                    }),
                ]
                .into(),
            ),
            cc: Some(
                vec![
                    Addr::Single(SingleInfo {
                        addr: "test-sender-1@local".into(),
                        display_name: Some("Sender 1".into()),
                    }),
                    Addr::Single(SingleInfo {
                        addr: "test-sender-2@local".into(),
                        display_name: Some("Sender 2".into()),
                    }),
                    Addr::Single(SingleInfo {
                        addr: "test-account@local".into(),
                        display_name: None,
                    }),
                ]
                .into(),
            ),
            ..Email::default()
        }
        .into_reply(true, &config)
        .unwrap();

        assert_eq!(
            email.to.unwrap().to_string(),
            "\"Sender 1\" <test-sender-1@local>, \"Sender 2\" <test-sender-2@local>"
        );
        assert_eq!(
            email.cc.unwrap().to_string(),
            "\"Sender 1\" <test-sender-1@local>, \"Sender 2\" <test-sender-2@local>"
        );
    }

    #[test]
    fn test_to_readable() {
        let config = AccountConfig::default();
        let email = Email {
            parts: Parts(vec![Part::TextPlain(TextPlainPart {
                content: String::from("hello, world!"),
            })]),
            ..Email::default()
        };

        // empty email headers, empty headers, empty config
        assert_eq!(
            "hello, world!",
            email.to_readable_string("plain", vec![], &config).unwrap()
        );
        // empty email headers, basic headers
        assert_eq!(
            "hello, world!",
            email
                .to_readable_string("plain", vec!["From", "DATE", "custom-hEader"], &config)
                .unwrap()
        );
        // empty email headers, multiple subject headers
        assert_eq!(
            "Subject: \n\nhello, world!",
            email
                .to_readable_string("plain", vec!["subject", "Subject", "SUBJECT"], &config)
                .unwrap()
        );

        let email = Email {
            headers: HashMap::from_iter([("custom-header".into(), "custom value".into())]),
            message_id: Some("<message-id>".into()),
            from: Some(
                vec![Addr::Single(SingleInfo {
                    addr: "test@local".into(),
                    display_name: Some("Test".into()),
                })]
                .into(),
            ),
            cc: Some(vec![].into()),
            parts: Parts(vec![Part::TextPlain(TextPlainPart {
                content: String::from("hello, world!"),
            })]),
            ..Email::default()
        };

        // header present in email headers, empty config
        assert_eq!(
            "From: \"Test\" <test@local>\n\nhello, world!",
            email
                .to_readable_string("plain", vec!["from"], &config)
                .unwrap()
        );
        // header present but empty in email headers, empty config
        assert_eq!(
            "hello, world!",
            email
                .to_readable_string("plain", vec!["cc"], &config)
                .unwrap()
        );
        // multiple same custom headers present in email headers, empty
        // config
        assert_eq!(
            "Custom-Header: custom value\n\nhello, world!",
            email
                .to_readable_string("plain", vec!["custom-header", "cuSTom-HeaDer"], &config)
                .unwrap()
        );

        let config = AccountConfig {
            email_reading_headers: Some(vec![
                "CusTOM-heaDER".into(),
                "Subject".into(),
                "from".into(),
                "cc".into(),
            ]),
            ..AccountConfig::default()
        };
        // header present but empty in email headers, empty config
        assert_eq!(
            "Custom-Header: custom value\nSubject: \nFrom: \"Test\" <test@local>\nMessage-Id: <message-id>\n\nhello, world!",
            email.to_readable_string("plain", vec!["cc", "message-ID"], &config)
                .unwrap()
        );
    }
}
