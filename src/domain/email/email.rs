use imap::types::{Fetch, ZeroCopy};
use lettre::{
    address::AddressError,
    message::{Mailbox, Mailboxes},
};
use log::{trace, warn};
use mailparse::{
    addrparse_header, DispositionType, MailAddr, MailHeaderMap, MailParseError, ParsedMail,
};
use mime_msg_builder::TplBuilder;
use ouroboros::self_referencing;
use std::{fmt::Debug, io, path::PathBuf, result};
use thiserror::Error;
use tree_magic;

#[cfg(feature = "maildir-backend")]
use maildir::{MailEntry, MailEntryError};

use crate::{account, process, AccountConfig, Attachment, DEFAULT_SIGNATURE_DELIM};

#[derive(Debug, Error)]
pub enum Error {
    #[error("cannot parse email")]
    #[cfg(feature = "maildir-backend")]
    GetMailEntryError(#[source] MailEntryError),

    #[error("cannot get parsed version of email: {0}")]
    GetParsedEmailError(String),
    #[error("cannot parse email")]
    ParseEmailError(#[source] MailParseError),
    #[error("cannot parse email body")]
    ParseEmailBodyError(#[source] MailParseError),
    #[error("cannot parse email: raw email is empty")]
    ParseEmailEmptyRawError,
    #[error("cannot parse message or address")]
    ParseEmailAddressError(#[from] AddressError),
    #[error("cannot delete local draft at {1}")]
    DeleteLocalDraftError(#[source] io::Error, PathBuf),

    #[cfg(feature = "imap-backend")]
    #[error("cannot parse email from imap fetches: empty fetches")]
    ParseEmailFromImapFetchesEmptyError,

    #[error(transparent)]
    ConfigError(#[from] account::config::Error),
    #[error(transparent)]
    MimeMsgBuilderError(#[from] mime_msg_builder::Error),
    #[error("cannot decrypt encrypted email part")]
    DecryptEmailPartError(#[source] process::Error),
    #[error("cannot verify signed email part")]
    VerifyEmailPartError(#[source] process::Error),

    // TODO: sort me
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
    DecryptPartError(#[source] account::config::Error),
}

#[derive(Debug, Error)]
enum ParsedBuilderError {
    #[error("cannot parse email")]
    MailParseError(#[source] MailParseError),
    #[cfg(feature = "maildir-backend")]
    #[error("cannot parse email")]
    MailEntryError(#[source] MailEntryError),
}

pub type Result<T> = result::Result<T, Error>;

enum RawEmail<'a> {
    Vec(Vec<u8>),
    Slice(&'a [u8]),
    #[cfg(feature = "imap-backend")]
    Fetch(&'a Fetch),
    #[cfg(feature = "maildir-backend")]
    MailEntry(&'a mut MailEntry),
}

#[self_referencing]
pub struct Email<'a> {
    raw: RawEmail<'a>,
    #[borrows(mut raw)]
    #[covariant]
    parsed: result::Result<ParsedMail<'this>, ParsedBuilderError>,
}

impl Email<'_> {
    fn parsed_builder<'a>(
        raw: &'a mut RawEmail,
    ) -> result::Result<ParsedMail<'a>, ParsedBuilderError> {
        match raw {
            RawEmail::Vec(bytes) => {
                mailparse::parse_mail(bytes).map_err(ParsedBuilderError::MailParseError)
            }
            RawEmail::Slice(bytes) => {
                mailparse::parse_mail(bytes).map_err(ParsedBuilderError::MailParseError)
            }
            #[cfg(feature = "imap-backend")]
            RawEmail::Fetch(fetch) => mailparse::parse_mail(fetch.body().unwrap_or_default())
                .map_err(ParsedBuilderError::MailParseError),
            #[cfg(feature = "maildir-backend")]
            RawEmail::MailEntry(entry) => {
                entry.parsed().map_err(ParsedBuilderError::MailEntryError)
            }
        }
    }

    pub fn parsed(&self) -> Result<&ParsedMail> {
        self.borrow_parsed()
            .as_ref()
            .map_err(|err| Error::GetParsedEmailError(err.to_string()))
    }

    pub fn raw(&self) -> Result<&[u8]> {
        self.parsed().map(|parsed| parsed.raw_bytes)
    }

    pub fn attachments(&self) -> Result<Vec<Attachment>> {
        let attachments = self.parsed()?.parts().filter_map(|part| {
            let cdisp = part.get_content_disposition();
            let mime = &part.ctype.mimetype;

            match cdisp.disposition {
                DispositionType::Attachment => {
                    let filename = cdisp.params.get("filename");
                    let body = part
                        .get_body_raw()
                        .map_err(|err| {
                            let filename = filename
                                .map(|f| format!("attachment {}", f))
                                .unwrap_or_else(|| "unknown attachment".into());
                            warn!("skipping {} {}: {}", mime, filename, err);
                            trace!("skipped part: {:#?}", part);
                            err
                        })
                        .ok()?;

                    Some(Attachment {
                        filename: filename.map(String::from),
                        // better to guess the real mime type from the
                        // body instead of using the one given from
                        // the content type
                        mime: tree_magic::from_u8(&body),
                        body,
                    })
                }
                DispositionType::Inline => match cdisp.params.get("filename") {
                    None => {
                        warn!("skipping {} inline attachment without a filename", mime);
                        None
                    }
                    Some(filename) => {
                        let body = part
                            .get_body_raw()
                            .map_err(|err| {
                                let filename = format!("attachment {}", filename);
                                warn!("skipping {} of type {}: {}", filename, mime, err);
                                trace!("skipped part: {:#?}", part);
                                err
                            })
                            .ok()?;

                        Some(Attachment {
                            filename: Some(filename.clone()),
                            // better to guess the real mime type from the
                            // body instead of using the one given from
                            // the content type
                            mime: tree_magic::from_u8(&body),
                            body,
                        })
                    }
                },
                // TODO
                DispositionType::FormData => None,
                // TODO
                DispositionType::Extension(_) => None,
            }
        });

        Ok(attachments.collect())
    }

    fn tpl_builder_from_parsed(config: &AccountConfig, parsed: &ParsedMail) -> Result<TplBuilder> {
        Self::tpl_builder_from_parsed_rec(config, TplBuilder::default(), parsed, true)
    }

    fn tpl_builder_from_parsed_rec(
        config: &AccountConfig,
        mut tpl: TplBuilder,
        parsed: &ParsedMail<'_>,
        take_headers: bool,
    ) -> Result<TplBuilder> {
        let mut in_pgp_signed_part = false;
        let mut in_pgp_encrypted_part = false;

        if take_headers {
            for header in &parsed.headers {
                tpl = tpl.set_header(header.get_key(), header.get_value());
            }
        }

        for part in parsed.parts() {
            match part.ctype.mimetype.as_str() {
                "multipart/signed" => {
                    let protocol = part.ctype.params.get("protocol").map(String::as_str);
                    if protocol == Some("application/pgp-signed") {
                        in_pgp_signed_part = true
                    }
                }
                "application/pgp-signed" => {
                    if in_pgp_signed_part {
                        let signed_body = part.get_body_raw().map_err(Error::ParseEmailError)?;
                        let parsed =
                            mailparse::parse_mail(&signed_body).map_err(Error::ParseEmailError)?;
                        tpl = Self::tpl_builder_from_parsed_rec(config, tpl, &parsed, false)?;
                    }
                }
                "application/pgp-signature" => {
                    if in_pgp_signed_part {
                        if let Some(ref verify_cmd) = config.email_reading_verify_cmd {
                            let signature = part.get_body_raw().map_err(Error::ParseEmailError)?;
                            let (_, exit_code) = process::pipe(verify_cmd, &signature)
                                .map_err(Error::VerifyEmailPartError)?;
                            if exit_code != 0 {
                                warn!("the signature could not be verified");
                            }
                        } else {
                            warn!("no verify command found, cannot verify signature");
                        }
                        in_pgp_signed_part = false
                    }
                }
                "multipart/encrypted" => {
                    let protocol = part.ctype.params.get("protocol").map(String::as_str);
                    if protocol == Some("application/pgp-encrypted") {
                        in_pgp_encrypted_part = true
                    }
                }
                "application/octet-stream" => {
                    if in_pgp_encrypted_part {
                        match config.email_reading_decrypt_cmd {
                            Some(ref decrypt_cmd) => {
                                let encrypted_body =
                                    part.get_body_raw().map_err(Error::ParseEmailError)?;
                                let (decrypted_part, _) =
                                    process::pipe(decrypt_cmd, &encrypted_body)
                                        .map_err(Error::DecryptEmailPartError)?;
                                let parsed = mailparse::parse_mail(&decrypted_part)
                                    .map_err(Error::ParseEmailError)?;
                                tpl =
                                    Self::tpl_builder_from_parsed_rec(config, tpl, &parsed, false)?;
                            }
                            None => {
                                warn!("no decrypt command found, skipping encrypted part");
                            }
                        }
                        in_pgp_encrypted_part = false;
                    } else {
                        tpl = tpl.part(
                            "application/octet-stream",
                            part.get_body_raw().map_err(Error::ParseEmailError)?,
                        );
                    }
                }
                "text/plain" => {
                    tpl = tpl.text_plain_part(part.get_body().map_err(Error::ParseEmailError)?);
                }
                "text/html" => {
                    tpl = tpl.text_html_part(part.get_body().map_err(Error::ParseEmailError)?);
                }
                mime => {
                    tpl = tpl.part(mime, part.get_body_raw().map_err(Error::ParseEmailError)?);
                }
            }
        }

        Ok(tpl)
    }

    /// Preconfigures a template builder for building new emails. It
    /// contains a "From" filled with the user's email address, an
    /// empty "To" and "Subject" and a text/plain part containing the
    /// user's signature (if existing). This function is useful when
    /// you need to compose a new email from scratch.
    pub fn new_tpl_builder(config: &AccountConfig) -> Result<TplBuilder> {
        let tpl = TplBuilder::default()
            .from(config.addr()?)
            .to("")
            .subject("")
            .text_plain_part(if let Some(ref signature) = config.signature()? {
                String::from("\n\n") + signature
            } else {
                String::new()
            });

        Ok(tpl)
    }

    pub fn to_read_tpl_builder(&self, config: &AccountConfig) -> Result<TplBuilder> {
        let parsed = self.parsed()?;
        Ok(Self::tpl_builder_from_parsed(config, &parsed)?)
    }

    pub fn to_reply_tpl_builder(&self, config: &AccountConfig, all: bool) -> Result<TplBuilder> {
        let mut tpl = TplBuilder::default();

        let parsed = self.parsed()?;
        let parsed_headers = parsed.get_headers();
        let sender = config.addr()?;

        // From

        tpl = tpl.from(&sender);

        // To

        tpl = tpl.to({
            let mut all_mboxes = Mailboxes::new();

            let from = parsed_headers.get_all_headers("From");
            let to = parsed_headers.get_all_headers("To");
            let reply_to = parsed_headers.get_all_headers("Reply-To");

            let reply_to_iter = if reply_to.is_empty() {
                from.into_iter()
            } else {
                reply_to.into_iter()
            };

            for reply_to in reply_to_iter.chain(to.into_iter()) {
                match addrparse_header(reply_to) {
                    Err(err) => warn!("skipping invalid addresses {:?}: {}", reply_to, err),
                    Ok(addrs) => {
                        for addr in addrs.iter() {
                            match addr {
                                MailAddr::Group(group) => match group.addrs.first() {
                                    None => (),
                                    Some(single) => {
                                        if single.addr != sender.email.as_ref() {
                                            all_mboxes.push(Mailbox::new(
                                                single.display_name.clone(),
                                                single.addr.parse().unwrap(),
                                            ))
                                        }
                                    }
                                },
                                MailAddr::Single(single) => {
                                    if single.addr != sender.email.as_ref() {
                                        all_mboxes.push(Mailbox::new(
                                            single.display_name.clone(),
                                            single.addr.parse().unwrap(),
                                        ))
                                    }
                                }
                            }
                        }
                    }
                }
            }

            if all {
                all_mboxes
            } else {
                all_mboxes
                    .into_single()
                    .map(|mbox| Mailboxes::from_iter([mbox]))
                    .unwrap_or_default()
            }
        });

        // In-Reply-To

        if let Some(ref message_id) = parsed_headers.get_first_value("Message-Id") {
            tpl = tpl.in_reply_to(message_id);
        }

        // Cc

        if all {
            tpl = tpl.cc({
                let mut cc = Mailboxes::new();

                for mboxes in parsed_headers.get_all_values("Cc") {
                    let mboxes: Mailboxes = mboxes.parse()?;
                    cc.extend(mboxes.into_iter().filter(|mbox| mbox.email != sender.email))
                }

                cc
            });
        }

        // Subject

        if let Some(subject) = parsed_headers.get_first_value("Subject") {
            tpl = tpl.subject(if subject.to_lowercase().starts_with("re:") {
                subject
            } else {
                String::from("Re: ") + &subject
            });
        }

        // Body

        tpl = tpl.text_plain_part({
            let mut lines = String::default();

            for part in parsed.parts() {
                if part.ctype.mimetype != "text/plain" {
                    continue;
                }

                lines.push_str("\n\n");

                let body = Self::tpl_builder_from_parsed(config, &parsed)?
                    .show_headers([] as [&str; 0])
                    .show_text_parts_only(true)
                    .sanitize_text_parts(true)
                    .build();

                for line in body.lines() {
                    // removes existing signature from the original body
                    if line[..] == DEFAULT_SIGNATURE_DELIM[0..3] {
                        break;
                    }

                    lines.push('>');
                    if !line.starts_with('>') {
                        lines.push_str(" ")
                    }
                    lines.push_str(line);
                    lines.push_str("\n");
                }
            }

            if let Some(ref signature) = config.signature()? {
                lines.push_str("\n");
                lines.push_str(signature);
            }

            lines
        });

        Ok(tpl)
    }

    pub fn to_forward_tpl_builder(&self, config: &AccountConfig) -> Result<TplBuilder> {
        let mut tpl = TplBuilder::default();

        let parsed = self.parsed()?;
        let parsed_headers = parsed.get_headers();
        let sender = config.addr()?;

        // From

        tpl = tpl.from(&sender);

        // To

        tpl = tpl.to("");

        // Subject

        let subject = parsed_headers
            .get_first_value("Subject")
            .unwrap_or_default();

        tpl = tpl.subject(if subject.to_lowercase().starts_with("fwd:") {
            subject
        } else {
            String::from("Fwd: ") + &subject
        });

        // Body

        tpl = tpl.text_plain_part({
            let mut lines = String::from("\n");

            if let Some(ref signature) = config.signature()? {
                lines.push_str("\n");
                lines.push_str(signature);
            }

            lines.push_str("\n-------- Forwarded Message --------\n");

            lines.push_str(
                &Self::tpl_builder_from_parsed(config, &parsed)?
                    .show_headers(["Date", "From", "To", "Cc", "Subject"])
                    .show_text_parts_only(true)
                    .sanitize_text_parts(true)
                    .build(),
            );

            lines
        });

        Ok(tpl)
    }
}

impl<'a> From<Vec<u8>> for Email<'a> {
    fn from(bytes: Vec<u8>) -> Self {
        EmailBuilder {
            raw: RawEmail::Vec(bytes),
            parsed_builder: Email::parsed_builder,
        }
        .build()
    }
}

impl<'a> From<&'a [u8]> for Email<'a> {
    fn from(bytes: &'a [u8]) -> Self {
        EmailBuilder {
            raw: RawEmail::Slice(bytes),
            parsed_builder: Email::parsed_builder,
        }
        .build()
    }
}

impl<'a> From<ParsedMail<'a>> for Email<'a> {
    fn from(parsed: ParsedMail<'a>) -> Self {
        EmailBuilder {
            raw: RawEmail::Slice(parsed.raw_bytes),
            parsed_builder: Email::parsed_builder,
        }
        .build()
    }
}

#[cfg(feature = "imap-backend")]
impl<'a> From<&'a Fetch> for Email<'a> {
    fn from(fetch: &'a Fetch) -> Self {
        EmailBuilder {
            raw: RawEmail::Fetch(fetch),
            parsed_builder: Email::parsed_builder,
        }
        .build()
    }
}

#[cfg(feature = "maildir-backend")]
impl<'a> From<&'a mut MailEntry> for Email<'a> {
    fn from(entry: &'a mut MailEntry) -> Self {
        EmailBuilder {
            raw: RawEmail::MailEntry(entry),
            parsed_builder: Email::parsed_builder,
        }
        .build()
    }
}

impl<'a> From<&'a str> for Email<'a> {
    fn from(str: &'a str) -> Self {
        str.as_bytes().into()
    }
}

enum RawEmails {
    #[cfg(feature = "imap-backend")]
    Fetches(ZeroCopy<Vec<Fetch>>),
    #[cfg(feature = "maildir-backend")]
    MailEntries(Vec<MailEntry>),
}

#[self_referencing]
pub struct Emails {
    raw: RawEmails,
    #[borrows(mut raw)]
    #[covariant]
    emails: Vec<Email<'this>>,
}

impl Emails {
    fn emails_builder<'a>(raw: &'a mut RawEmails) -> Vec<Email> {
        match raw {
            #[cfg(feature = "imap-backend")]
            RawEmails::Fetches(fetches) => fetches.iter().map(Email::from).collect(),
            #[cfg(feature = "maildir-backend")]
            RawEmails::MailEntries(entries) => entries.iter_mut().map(Email::from).collect(),
        }
    }

    pub fn to_vec(&self) -> Vec<&Email> {
        self.borrow_emails().iter().collect()
    }
}

#[cfg(feature = "imap-backend")]
impl TryFrom<ZeroCopy<Vec<Fetch>>> for Emails {
    type Error = Error;

    fn try_from(fetches: ZeroCopy<Vec<Fetch>>) -> Result<Self> {
        if fetches.is_empty() {
            Err(Error::ParseEmailFromImapFetchesEmptyError)
        } else {
            Ok(EmailsBuilder {
                raw: RawEmails::Fetches(fetches),
                emails_builder: Emails::emails_builder,
            }
            .build())
        }
    }
}

#[cfg(feature = "maildir-backend")]
impl TryFrom<Vec<MailEntry>> for Emails {
    type Error = Error;

    fn try_from(entries: Vec<MailEntry>) -> Result<Self> {
        if entries.is_empty() {
            Err(Error::ParseEmailFromImapFetchesEmptyError)
        } else {
            Ok(EmailsBuilder {
                raw: RawEmails::MailEntries(entries),
                emails_builder: Emails::emails_builder,
            }
            .build())
        }
    }
}

#[cfg(test)]
mod email {
    use concat_with::concat_line;

    use crate::{AccountConfig, Email};

    #[test]
    fn new_tpl_builder() {
        let config = AccountConfig {
            email: "from@localhost".into(),
            ..AccountConfig::default()
        };

        let tpl = Email::new_tpl_builder(&config).unwrap().build();

        let expected_tpl = concat_line!("From: from@localhost", "To: ", "Subject: ", "", "");

        assert_eq!(expected_tpl, *tpl);
    }

    #[test]
    fn new_tpl_builder_with_signature() {
        let config = AccountConfig {
            email: "from@localhost".into(),
            signature: Some("Regards,".into()),
            ..AccountConfig::default()
        };

        let tpl = Email::new_tpl_builder(&config).unwrap().build();

        let expected_tpl = concat_line!(
            "From: from@localhost",
            "To: ",
            "Subject: ",
            "",
            "",
            "",
            "-- ",
            "Regards,"
        );

        assert_eq!(expected_tpl, *tpl);
    }

    #[test]
    fn to_read_tpl_builder() {
        let config = AccountConfig::default();
        let email = Email::from(concat_line!(
            "From: from@localhost",
            "To: to@localhost",
            "Subject: subject",
            "",
            "Hello!",
            "",
            "-- ",
            "Regards,"
        ));

        let tpl = email
            .to_read_tpl_builder(&config)
            .unwrap()
            .show_headers([] as [String; 0])
            .build();

        let expected_tpl = concat_line!("Hello!", "", "-- ", "Regards,");

        assert_eq!(expected_tpl, *tpl);
    }

    #[test]
    fn to_read_tpl_builder_with_email_reading_headers_config() {
        let config = AccountConfig::default();
        let email = Email::from(concat_line!(
            "From: from@localhost",
            "To: to@localhost",
            "Subject: subject",
            "",
            "Hello!",
            "",
            "-- ",
            "Regards,"
        ));

        let tpl = email
            .to_read_tpl_builder(&config)
            .unwrap()
            .show_headers([
                "From", "Subject", // existing headers
                "Cc", "Bcc", // nonexisting headers
            ])
            .build();

        let expected_tpl = concat_line!(
            "From: from@localhost",
            "Subject: subject",
            "",
            "Hello!",
            "",
            "-- ",
            "Regards,"
        );

        assert_eq!(expected_tpl, *tpl);
    }

    #[test]
    fn to_read_tpl_builder_with_show_all_headers_option() {
        let config = AccountConfig::default();
        let email = Email::from(concat_line!(
            "From: from@localhost",
            "To: to@localhost",
            "Subject: subject",
            "",
            "Hello!",
            "",
            "-- ",
            "Regards,"
        ));

        let tpl = email.to_read_tpl_builder(&config).unwrap().build();

        let expected_tpl = concat_line!(
            "From: from@localhost",
            "To: to@localhost",
            "Subject: subject",
            "",
            "Hello!",
            "",
            "-- ",
            "Regards,"
        );

        assert_eq!(expected_tpl, *tpl);
    }

    #[test]
    fn to_read_tpl_builder_with_show_only_headers_option() {
        let config = AccountConfig::default();
        let email = Email::from(concat_line!(
            "From: from@localhost",
            "To: to@localhost",
            "Subject: subject",
            "",
            "Hello!",
            "",
            "-- ",
            "Regards,"
        ));

        let tpl = email
            .to_read_tpl_builder(&config)
            .unwrap()
            .show_headers([
                // existing headers
                "Subject",
                "To",
                // nonexisting header
                "Content-Type",
            ])
            .build();

        let expected_tpl = concat_line!(
            "Subject: subject",
            "To: to@localhost",
            "",
            "Hello!",
            "",
            "-- ",
            "Regards,"
        );

        assert_eq!(expected_tpl, *tpl);
    }

    #[test]
    fn to_reply_tpl_builder() {
        let config = AccountConfig {
            email: "to@localhost".into(),
            ..AccountConfig::default()
        };

        let email = Email::from(concat_line!(
            "From: from@localhost",
            "To: to@localhost, to2@localhost",
            "Cc: cc@localhost, cc2@localhost",
            "Bcc: bcc@localhost",
            "Subject: subject",
            "",
            "Hello!",
            "",
            "-- ",
            "Regards,"
        ));

        let tpl = email.to_reply_tpl_builder(&config, false).unwrap().build();

        let expected_tpl = concat_line!(
            "From: to@localhost",
            "To: from@localhost",
            "Subject: Re: subject",
            "",
            "",
            "",
            "> Hello!",
            "> ",
            ""
        );

        assert_eq!(expected_tpl, *tpl);
    }

    #[test]
    fn to_reply_all_tpl_builder() {
        let config = AccountConfig {
            email: "to@localhost".into(),
            ..AccountConfig::default()
        };

        let email = Email::from(concat_line!(
            "From: from@localhost",
            "To: to@localhost, to2@localhost",
            "Cc: to@localhost, cc@localhost, cc2@localhost",
            "Bcc: bcc@localhost",
            "Subject: subject",
            "",
            "Hello!",
            "",
            "-- ",
            "Regards,"
        ));

        let tpl = email.to_reply_tpl_builder(&config, true).unwrap().build();

        let expected_tpl = concat_line!(
            "From: to@localhost",
            "To: from@localhost, to2@localhost",
            "Cc: cc@localhost, cc2@localhost",
            "Subject: Re: subject",
            "",
            "",
            "",
            "> Hello!",
            "> ",
            ""
        );

        assert_eq!(expected_tpl, *tpl);
    }

    #[test]
    fn to_reply_tpl_builder_with_signature() {
        let config = AccountConfig {
            email: "to@localhost".into(),
            signature: Some("Cordialement,".into()),
            ..AccountConfig::default()
        };

        let email = Email::from(concat_line!(
            "From: from@localhost",
            "To: to@localhost",
            "Subject: subject",
            "",
            "Hello!",
            "",
            "-- ",
            "Regards,"
        ));

        let tpl = email.to_reply_tpl_builder(&config, false).unwrap().build();

        let expected_tpl = concat_line!(
            "From: to@localhost",
            "To: from@localhost",
            "Subject: Re: subject",
            "",
            "",
            "",
            "> Hello!",
            "> ",
            "",
            "-- ",
            "Cordialement,"
        );

        assert_eq!(expected_tpl, *tpl);
    }

    #[test]
    fn to_forward_tpl_builder() {
        let config = AccountConfig {
            email: "to@localhost".into(),
            ..AccountConfig::default()
        };

        let email = Email::from(concat_line!(
            "From: from@localhost",
            "To: to@localhost, to2@localhost",
            "Cc: cc@localhost, cc2@localhost",
            "Bcc: bcc@localhost",
            "Subject: subject",
            "",
            "Hello!",
            "",
            "-- ",
            "Regards,"
        ));

        let tpl = email.to_forward_tpl_builder(&config).unwrap().build();

        let expected_tpl = concat_line!(
            "From: to@localhost",
            "To: ",
            "Subject: Fwd: subject",
            "",
            "",
            "",
            "-------- Forwarded Message --------",
            "From: from@localhost",
            "To: to@localhost, to2@localhost",
            "Cc: cc@localhost, cc2@localhost",
            "Subject: subject",
            "",
            "Hello!",
            "",
            "-- ",
            "Regards,"
        );

        assert_eq!(expected_tpl, *tpl);
    }

    #[test]
    fn to_forward_tpl_builder_with_date_and_signature() {
        let config = AccountConfig {
            email: "to@localhost".into(),
            signature: Some("Cordialement,".into()),
            ..AccountConfig::default()
        };

        let email = Email::from(concat_line!(
            "Date: Thu, 10 Nov 2022 14:26:33 +0000",
            "From: from@localhost",
            "To: to@localhost, to2@localhost",
            "Cc: cc@localhost, cc2@localhost",
            "Bcc: bcc@localhost",
            "Subject: subject",
            "",
            "Hello!",
            "",
            "-- ",
            "Regards,"
        ));

        let tpl = email.to_forward_tpl_builder(&config).unwrap().build();

        let expected_tpl = concat_line!(
            "From: to@localhost",
            "To: ",
            "Subject: Fwd: subject",
            "",
            "",
            "",
            "-- ",
            "Cordialement,",
            "-------- Forwarded Message --------",
            "Date: Thu, 10 Nov 2022 14:26:33 +0000",
            "From: from@localhost",
            "To: to@localhost, to2@localhost",
            "Cc: cc@localhost, cc2@localhost",
            "Subject: subject",
            "",
            "Hello!",
            "",
            "-- ",
            "Regards,"
        );

        assert_eq!(expected_tpl, *tpl);
    }
}
