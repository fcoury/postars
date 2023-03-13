//! IMAP backend module.
//!
//! This module contains the definition of the IMAP backend.

use imap::extensions::idle::{stop_on_any, SetReadTimeout};
use imap_proto::{NameAttribute, UidSetMember};
use log::{debug, info, log_enabled, trace, Level};
use native_tls::{TlsConnector, TlsStream};
use rayon::prelude::*;
use std::{
    any::Any,
    borrow::Cow,
    collections::HashSet,
    convert::TryInto,
    io::{self, Read, Write},
    net::TcpStream,
    result, string,
    sync::{Mutex, MutexGuard},
    thread,
    time::Duration,
};
use thiserror::Error;
use utf7_imap::{decode_utf7_imap as decode_utf7, encode_utf7_imap as encode_utf7};

use crate::{
    account, backend, email, envelope, process, AccountConfig, Backend, Emails, Envelope,
    Envelopes, Flag, Flags, Folder, Folders, ImapConfig,
};

#[derive(Error, Debug)]
pub enum Error {
    // Folders
    #[error("cannot create imap folder {1}")]
    CreateFolderError(#[source] imap::Error, String),
    #[error("cannot select imap folder {1}")]
    SelectFolderError(#[source] imap::Error, String),
    #[error("cannot list imap folders")]
    ListFoldersError(#[source] imap::Error),
    #[error("cannot examine folder {1}")]
    ExamineFolderError(#[source] imap::Error, String),
    #[error("cannot expunge imap folder {1}")]
    ExpungeFolderError(#[source] imap::Error, String),
    #[error("cannot delete imap folder {1}")]
    DeleteFolderError(#[source] imap::Error, String),

    // Envelopes
    #[error("cannot get imap envelope of email {0}")]
    GetEnvelopeError(String),
    #[error("cannot list imap envelopes: page {0} out of bounds")]
    ListEnvelopesOutOfBounds(usize),
    #[error("cannot fetch new imap envelopes")]
    FetchNewEnvelopesError(#[source] imap::Error),
    #[error("cannot search new imap envelopes")]
    SearchNewEnvelopesError(#[source] imap::Error),
    #[error("cannot search imap envelopes in folder {1} with query: {2}")]
    SearchEnvelopesError(#[source] imap::Error, String, String),
    #[error("cannot sort imap envelopes in folder {1} with query: {2}")]
    SortEnvelopesError(#[source] imap::Error, String, String),
    #[error("cannot get next imap envelope uid of folder {0}")]
    GetNextEnvelopeUidError(String),

    // Flags
    #[error("cannot add flags {1} to imap email(s) {2}")]
    AddFlagsError(#[source] imap::Error, String, String),
    #[error("cannot set flags {1} to emails(s) {2}")]
    SetFlagsError(#[source] imap::Error, String, String),
    #[error("cannot remove flags {1} from email(s) {2}")]
    RemoveFlagsError(#[source] imap::Error, String, String),

    // Emails
    #[error("cannot copy imap email(s) {1} from {2} to {3}")]
    CopyEmailError(#[source] imap::Error, String, String, String),
    #[error("cannot move email(s) {1} from {2} to {3}")]
    MoveEmailError(#[source] imap::Error, String, String, String),
    #[error("cannot fetch imap email {1}")]
    FetchEmailsByUidError(#[source] imap::Error, String),
    #[error("cannot fetch imap emails within uid range {1}")]
    FetchEmailsByUidRangeError(#[source] imap::Error, String),
    #[error("cannot get added email uid from range {0}")]
    GetAddedEmailUidFromRangeError(String),
    #[error("cannot get added email uid (extensions UIDPLUS not enabled on the server?)")]
    GetAddedEmailUidError,
    #[error("cannot append email to folder {1}")]
    AppendEmailError(#[source] imap::Error, String),

    // Parsing/decoding
    #[error("cannot parse sender from imap envelope")]
    ParseSenderFromImapEnvelopeError,
    #[error("cannot decode sender name from imap envelope")]
    DecodeSenderNameFromImapEnvelopeError(rfc2047_decoder::Error),
    #[error("cannot decode sender mailbox from imap envelope")]
    DecodeSenderMailboxFromImapEnvelopeError(rfc2047_decoder::Error),
    #[error("cannot decode sender host from imap envelope")]
    DecodeSenderHostFromImapEnvelopeError(rfc2047_decoder::Error),
    #[error("cannot decode date from imap envelope")]
    DecodeDateFromImapEnvelopeError(rfc2047_decoder::Error),
    #[error("cannot parse timestamp from imap envelope: {1}")]
    ParseTimestampFromImapEnvelopeError(mailparse::MailParseError, String),
    #[error("cannot parse imap sort criterion {0}")]
    ParseSortCriterionError(String),
    #[error("cannot decode subject of imap email {1}")]
    DecodeSubjectError(#[source] rfc2047_decoder::Error, String),
    #[error("cannot get imap sender of email {0}")]
    GetSenderError(String),
    #[error("cannot get uid of email sequence {0}")]
    GetUidError(u32),

    // Sessions
    #[error("cannot find session from pool at cursor {0}")]
    FindSessionByCursorError(usize),
    #[error("cannot parse Message-ID of email {0}")]
    ParseMessageIdError(#[source] string::FromUtf8Error, String),
    #[error("cannot lock imap session: {0}")]
    LockSessionError(String),
    #[error("cannot lock imap sessions pool cursor: {0}")]
    LockSessionsPoolCursorError(String),
    #[error("cannot create tls connector")]
    CreateTlsConnectorError(#[source] native_tls::Error),
    #[error("cannot connect to imap server")]
    ConnectImapServerError(#[source] imap::Error),
    #[error("cannot login to imap server")]
    LoginImapServerError(#[source] imap::Error),
    #[error("cannot start the idle mode")]
    StartIdleModeError(#[source] imap::Error),
    #[error("cannot close imap session")]
    CloseImapSessionError(#[source] imap::Error),

    // Other error forwarding
    #[error(transparent)]
    ConfigError(#[from] account::config::Error),
    #[error(transparent)]
    ImapConfigError(#[from] backend::imap::config::Error),
    #[error(transparent)]
    EmailError(#[from] email::Error),
    #[error(transparent)]
    MaildirBackend(#[from] backend::maildir::Error),
}

pub type Result<T> = result::Result<T, Error>;

#[derive(Debug)]
pub enum ImapSessionStream {
    Tls(TlsStream<TcpStream>),
    Tcp(TcpStream),
}

impl SetReadTimeout for ImapSessionStream {
    fn set_read_timeout(&mut self, timeout: Option<Duration>) -> imap::Result<()> {
        match self {
            Self::Tls(stream) => stream.set_read_timeout(timeout),
            Self::Tcp(stream) => stream.set_read_timeout(timeout),
        }
    }
}

impl Read for ImapSessionStream {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        match self {
            Self::Tls(stream) => stream.read(buf),
            Self::Tcp(stream) => stream.read(buf),
        }
    }
}

impl Write for ImapSessionStream {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match self {
            Self::Tls(stream) => stream.write(buf),
            Self::Tcp(stream) => stream.write(buf),
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        match self {
            Self::Tls(stream) => stream.flush(),
            Self::Tcp(stream) => stream.flush(),
        }
    }
}

pub type ImapSession = imap::Session<ImapSessionStream>;

pub struct ImapBackendBuilder {
    sessions_pool_size: usize,
}

impl Default for ImapBackendBuilder {
    fn default() -> Self {
        Self {
            sessions_pool_size: 1,
        }
    }
}

impl<'a> ImapBackendBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn pool_size(mut self, pool_size: usize) -> Self {
        self.sessions_pool_size = pool_size;
        self
    }

    pub fn build(
        &self,
        account_config: Cow<'a, AccountConfig>,
        imap_config: Cow<'a, ImapConfig>,
    ) -> Result<ImapBackend<'a>> {
        let passwd = imap_config.passwd()?;
        let sessions_pool: Vec<_> = (0..=self.sessions_pool_size).collect();
        let backend = ImapBackend {
            account_config,
            imap_config: imap_config.clone(),
            sessions_pool_size: self.sessions_pool_size.max(1),
            sessions_pool_cursor: Mutex::new(0),
            sessions_pool: sessions_pool
                .par_iter()
                .flat_map(|_| ImapBackend::create_session(&imap_config, &passwd).map(Mutex::new))
                .collect(),
        };

        Ok(backend)
    }
}

pub struct ImapBackend<'a> {
    account_config: Cow<'a, AccountConfig>,
    imap_config: Cow<'a, ImapConfig>,
    sessions_pool_size: usize,
    sessions_pool_cursor: Mutex<usize>,
    sessions_pool: Vec<Mutex<ImapSession>>,
}

#[derive(Debug)]
struct OAuth2 {
    user: String,
    access_token: String,
}

impl imap::Authenticator for OAuth2 {
    type Response = String;
    fn process(&self, _: &[u8]) -> Self::Response {
        format!(
            "user={}\x01auth=Bearer {}\x01\x01",
            self.user, self.access_token
        )
    }
}

impl<'a> ImapBackend<'a> {
    pub fn new(
        account_config: Cow<'a, AccountConfig>,
        imap_config: Cow<'a, ImapConfig>,
    ) -> Result<Self> {
        ImapBackendBuilder::default().build(account_config, imap_config)
    }

    fn create_session<P>(config: &'a ImapConfig, passwd: P) -> Result<ImapSession>
    where
        P: AsRef<str>,
    {
        let builder = TlsConnector::builder()
            .danger_accept_invalid_certs(config.insecure())
            .danger_accept_invalid_hostnames(config.insecure())
            .build()
            .map_err(Error::CreateTlsConnectorError)?;

        let mut client_builder = imap::ClientBuilder::new(&config.host, config.port);
        if config.starttls() {
            client_builder.starttls();
        }

        let client = if config.ssl() {
            client_builder.connect(|domain, tcp| {
                let connector = TlsConnector::connect(&builder, domain, tcp)?;
                Ok(ImapSessionStream::Tls(connector))
            })
        } else {
            client_builder.connect(|_, tcp| Ok(ImapSessionStream::Tcp(tcp)))
        }
        .map_err(Error::ConnectImapServerError)?;

        let mut session = if let Some(access_token) = config.clone().access_token {
            let auth = OAuth2 {
                user: config.login.clone(),
                access_token,
            };

            client.authenticate("XOAUTH2", &auth)
        } else {
            client.login(&config.login, passwd.as_ref())
        }
        .map_err(|res| Error::LoginImapServerError(res.0))?;

        session.debug = log_enabled!(Level::Trace);

        Result::Ok(session)
    }

    pub fn session(&self) -> Result<MutexGuard<ImapSession>> {
        let session = {
            let mut cursor = self
                .sessions_pool_cursor
                .lock()
                .map_err(|err| Error::LockSessionsPoolCursorError(err.to_string()))?;
            let session = self
                .sessions_pool
                .get(*cursor)
                .ok_or(Error::FindSessionByCursorError(*cursor))?;
            // TODO: find a way to get the next available connection
            // instead of the next one in the list
            *cursor = (*cursor + 1) % self.sessions_pool_size;
            session
        };

        session
            .lock()
            .map_err(|err| Error::LockSessionError(err.to_string()))
    }

    fn search_new_msgs(&self, session: &mut ImapSession, query: &str) -> Result<Vec<u32>> {
        let uids: Vec<u32> = session
            .uid_search(query)
            .map_err(Error::SearchNewEnvelopesError)?
            .into_iter()
            .collect();
        debug!("found {} new messages", uids.len());
        trace!("uids: {:?}", uids);

        Ok(uids)
    }

    pub fn notify(&self, keepalive: u64, folder: &str) -> Result<()> {
        let mut session = self.session()?;

        session
            .examine(folder)
            .map_err(|err| Error::ExamineFolderError(err, folder.to_owned()))?;

        debug!("init messages hashset");
        let mut msgs_set: HashSet<u32> = self
            .search_new_msgs(&mut session, &self.imap_config.notify_query())?
            .iter()
            .cloned()
            .collect::<HashSet<_>>();
        trace!("messages hashset: {:?}", msgs_set);

        loop {
            debug!("begin loop");
            session
                .idle()
                .timeout(Duration::new(keepalive, 0))
                .wait_while(stop_on_any)
                .map_err(Error::StartIdleModeError)?;

            let uids: Vec<u32> = self
                .search_new_msgs(&mut session, &self.imap_config.notify_query())?
                .into_iter()
                .filter(|uid| msgs_set.get(uid).is_none())
                .collect();
            debug!("found {} new messages not in hashset", uids.len());
            trace!("messages hashet: {:?}", msgs_set);

            if !uids.is_empty() {
                let uids = uids
                    .iter()
                    .map(|uid| uid.to_string())
                    .collect::<Vec<_>>()
                    .join(",");
                let fetches = session
                    .uid_fetch(uids, "(UID ENVELOPE)")
                    .map_err(Error::FetchNewEnvelopesError)?;

                for fetch in fetches.iter() {
                    let msg = envelope::imap::from_raw(fetch)?;
                    let uid = fetch.uid.ok_or_else(|| Error::GetUidError(fetch.message))?;

                    let from = msg.from.addr.clone();
                    self.imap_config.run_notify_cmd(uid, &msg.subject, &from)?;

                    debug!("notify message: {}", uid);
                    trace!("message: {:?}", msg);

                    debug!("insert message {} in hashset", uid);
                    msgs_set.insert(uid);
                    trace!("messages hashset: {:?}", msgs_set);
                }
            }

            debug!("end loop");
        }
    }

    pub fn watch(&self, keepalive: u64, mbox: &str) -> Result<()> {
        debug!("examine folder: {}", mbox);
        let mut session = self.session()?;

        session
            .examine(mbox)
            .map_err(|err| Error::ExamineFolderError(err, mbox.to_owned()))?;

        loop {
            debug!("begin loop");

            let cmds = self.imap_config.watch_cmds().clone();
            thread::spawn(move || {
                debug!("batch execution of {} cmd(s)", cmds.len());
                cmds.iter().for_each(|cmd| match process::run(cmd, &[]) {
                    // TODO: manage errors
                    Err(_) => (),
                    Ok(_) => (),
                })
            });

            session
                .idle()
                .timeout(Duration::new(keepalive, 0))
                .wait_while(stop_on_any)
                .map_err(Error::StartIdleModeError)?;

            debug!("end loop");
        }
    }
}

impl<'a> Backend for ImapBackend<'a> {
    fn name(&self) -> String {
        self.account_config.name.clone()
    }

    fn add_folder(&self, folder: &str) -> backend::Result<()> {
        info!("adding imap folder {folder}");

        let folder_encoded = encode_utf7(folder.to_owned());
        trace!("utf7 encoded folder: {folder_encoded}");

        let mut session = self.session()?;
        session
            .create(folder_encoded)
            .map_err(|err| Error::CreateFolderError(err, folder.to_owned()))?;

        Ok(())
    }

    fn list_folders(&self) -> backend::Result<Folders> {
        info!("listing imap folders");

        let mut session = self.session()?;
        let folders = session
            .list(Some(""), Some("*"))
            .map_err(Error::ListFoldersError)?;
        let folders = Folders::from_iter(folders.iter().filter_map(|folder| {
            if folder.attributes().contains(&NameAttribute::NoSelect) {
                None
            } else {
                Some(Folder {
                    delim: folder.delimiter().unwrap_or_default().into(),
                    name: decode_utf7(folder.name().into()),
                    desc: folder
                        .attributes()
                        .iter()
                        .map(|attr| format!("{attr:?}"))
                        .collect::<Vec<_>>()
                        .join(", "),
                })
            }
        }));
        trace!("imap folders: {:?}", folders);

        Ok(folders)
    }

    fn expunge_folder(&self, folder: &str) -> backend::Result<()> {
        info!("expunging imap folder {folder}");

        let folder_encoded = encode_utf7(folder.to_owned());
        trace!("utf7 encoded folder: {folder_encoded}");

        let mut session = self.session()?;
        session
            .select(folder_encoded)
            .map_err(|err| Error::SelectFolderError(err, folder.to_owned()))?;
        session
            .expunge()
            .map_err(|err| Error::ExpungeFolderError(err, folder.to_owned()))?;

        Ok(())
    }

    fn purge_folder(&self, folder: &str) -> backend::Result<()> {
        info!("purging imap folder {folder}");

        let folder_encoded = encode_utf7(folder.to_owned());
        trace!("utf7 encoded folder: {folder_encoded}");

        let flags = Flags::from_iter([Flag::Deleted]);
        let uids = String::from("1:*");

        let mut session = self.session()?;
        session
            .select(folder_encoded)
            .map_err(|err| Error::SelectFolderError(err, folder.to_owned()))?;
        session
            .uid_store(&uids, format!("+FLAGS ({})", flags.to_imap_query()))
            .map_err(|err| Error::AddFlagsError(err, flags.to_imap_query(), uids))?;
        session
            .expunge()
            .map_err(|err| Error::ExpungeFolderError(err, folder.to_owned()))?;

        Ok(())
    }

    fn delete_folder(&self, folder: &str) -> backend::Result<()> {
        info!("deleting imap folder {folder}");

        let folder_encoded = encode_utf7(folder.to_owned());
        trace!("utf7 encoded folder: {folder_encoded}");

        let mut session = self.session()?;
        session
            .delete(&folder_encoded)
            .map_err(|err| Error::DeleteFolderError(err, folder.to_owned()))?;

        Ok(())
    }

    fn get_envelope(&self, folder: &str, uid: &str) -> backend::Result<Envelope> {
        info!("getting imap envelope {uid} from folder {folder}");

        let folder_encoded = encode_utf7(folder.to_owned());
        trace!("utf7 encoded folder: {folder_encoded}");

        let mut session = self.session()?;
        session
            .select(&folder_encoded)
            .map_err(|err| Error::SelectFolderError(err, folder.to_owned()))?;
        let fetches = session
            .uid_fetch(uid, "(UID FLAGS ENVELOPE)")
            .map_err(|err| Error::FetchEmailsByUidError(err, uid.to_owned()))?;
        let fetch = fetches
            .get(0)
            .ok_or_else(|| Error::GetEnvelopeError(uid.to_owned()))?;

        let envelope = envelope::imap::from_raw(&fetch)?;
        trace!("imap envelope: {envelope:#?}");

        Ok(envelope)
    }

    fn list_envelopes(
        &self,
        folder: &str,
        page_size: usize,
        page: usize,
    ) -> backend::Result<Envelopes> {
        info!("listing imap envelopes from folder {folder}");

        let folder_encoded = encode_utf7(folder.to_owned());
        trace!("utf7 encoded folder: {folder_encoded}");

        let mut session = self.session()?;
        let folder_size = session
            .select(&folder_encoded)
            .map_err(|err| Error::SelectFolderError(err, folder.to_owned()))?
            .exists as usize;
        trace!("folder size: {folder_size}");

        if folder_size == 0 {
            return Ok(Envelopes::default());
        }

        let page_cursor = page * page_size;
        if page_cursor >= folder_size {
            return Err(Error::ListEnvelopesOutOfBounds(page + 1))?;
        }

        let range = if page_size == 0 {
            String::from("1:*")
        } else {
            let page_size = page_size.min(folder_size);
            let mut count = 1;
            let mut cursor = folder_size - (folder_size.min(page_cursor));
            let mut range = cursor.to_string();
            while cursor > 0 && count < page_size {
                count += 1;
                cursor -= 1;
                if count > 1 {
                    range.push(',');
                }
                range.push_str(&cursor.to_string());
            }
            range
        };
        trace!("page: {page}");
        trace!("page size: {page_size}");
        trace!("seq range: {range}");

        let fetches = session
            .fetch(&range, "(UID FLAGS ENVELOPE)")
            .map_err(|err| Error::FetchEmailsByUidRangeError(err, range))?;
        let envelopes = envelope::imap::from_raws(fetches)?;
        trace!("imap envelopes: {envelopes:#?}");

        Ok(envelopes)
    }

    fn search_envelopes(
        &self,
        folder: &str,
        query: &str,
        sort: &str,
        page_size: usize,
        page: usize,
    ) -> backend::Result<Envelopes> {
        info!("searching imap envelopes from folder {folder}");

        let folder_encoded = encode_utf7(folder.to_owned());
        trace!("utf7 encoded folder: {folder_encoded}");

        let mut session = self.session()?;
        let folder_size = session
            .select(&folder_encoded)
            .map_err(|err| Error::SelectFolderError(err, folder.to_owned()))?
            .exists as usize;
        trace!("folder size: {folder_size}");

        if folder_size == 0 {
            return Ok(Envelopes::default());
        }

        let uids: Vec<String> = if sort.is_empty() {
            session
                .uid_search(query)
                .map_err(|err| {
                    Error::SearchEnvelopesError(err, folder.to_owned(), query.to_owned())
                })?
                .iter()
                .map(|seq| seq.to_string())
                .collect()
        } else {
            let sort: envelope::imap::SortCriteria = sort.try_into()?;
            session
                .uid_sort(&sort, imap::extensions::sort::SortCharset::Utf8, query)
                .map_err(|err| Error::SortEnvelopesError(err, folder.to_owned(), query.to_owned()))?
                .iter()
                .map(|uid| uid.to_string())
                .collect()
        };
        trace!("uids: {uids:?}");

        if uids.is_empty() {
            return Ok(Envelopes::default());
        }

        let uid_range = if page_size > 0 {
            let begin = uids.len().min(page * page_size);
            let end = begin + uids.len().min(page_size);
            if end > begin + 1 {
                uids[begin..end].join(",")
            } else {
                uids[0].to_string()
            }
        } else {
            uids.join(",")
        };
        trace!("page: {page}");
        trace!("page size: {page_size}");
        trace!("uid range: {uid_range}");

        let fetches = session
            .uid_fetch(&uid_range, "(UID FLAGS ENVELOPE)")
            .map_err(|err| Error::FetchEmailsByUidRangeError(err, uid_range))?;
        let envelopes = envelope::imap::from_raws(fetches)?;
        trace!("imap envelopes: {envelopes:#?}");

        Ok(envelopes)
    }

    fn add_email(&self, folder: &str, email: &[u8], flags: &Flags) -> backend::Result<String> {
        info!(
            "adding imap email to folder {folder} with flags {flags}",
            flags = flags.to_string(),
        );

        let folder_encoded = encode_utf7(folder.to_owned());
        trace!("utf7 encoded folder: {folder_encoded}");

        let mut session = self.session()?;
        let appended = session
            .append(&folder, email)
            .flags(flags.into_imap_flags_vec())
            .finish()
            .map_err(|err| Error::AppendEmailError(err, folder.to_owned()))?;

        let uid = match appended.uids {
            Some(mut uids) if uids.len() == 1 => match uids.get_mut(0).unwrap() {
                UidSetMember::Uid(uid) => Ok(*uid),
                UidSetMember::UidRange(uids) => Ok(uids.next().ok_or_else(|| {
                    Error::GetAddedEmailUidFromRangeError(uids.fold(String::new(), |range, uid| {
                        if range.is_empty() {
                            uid.to_string()
                        } else {
                            range + ", " + &uid.to_string()
                        }
                    }))
                })?),
            },
            _ => {
                // TODO: find a way to retrieve the UID of the added
                // email (by Message-ID?)
                Err(Error::GetAddedEmailUidError)
            }
        }?;
        trace!("uid: {uid}");

        Ok(uid.to_string())
    }

    fn preview_emails(&self, folder: &str, uids: Vec<&str>) -> backend::Result<Emails> {
        let uids = uids.join(",");
        info!("previewing imap emails {uids} from folder {folder}");

        let folder_encoded = encode_utf7(folder.to_owned());
        trace!("utf7 encoded folder: {folder_encoded}");

        let mut session = self.session()?;
        session
            .select(&folder_encoded)
            .map_err(|err| Error::SelectFolderError(err, folder.to_owned()))?;
        let fetches = session
            .uid_fetch(&uids, "BODY.PEEK[]")
            .map_err(|err| Error::FetchEmailsByUidRangeError(err, uids))?;

        Ok(Emails::try_from(fetches)?)
    }

    fn get_emails(&self, folder: &str, uids: Vec<&str>) -> backend::Result<Emails> {
        let uids = uids.join(",");
        info!("getting imap emails {uids} from folder {folder}");

        let folder_encoded = encode_utf7(folder.to_owned());
        trace!("utf7 encoded folder: {folder_encoded}");

        let mut session = self.session()?;
        session
            .select(&folder_encoded)
            .map_err(|err| Error::SelectFolderError(err, folder.to_owned()))?;
        let fetches = session
            .uid_fetch(&uids, "BODY[]")
            .map_err(|err| Error::FetchEmailsByUidRangeError(err, uids))?;

        Ok(Emails::try_from(fetches)?)
    }

    fn copy_emails(
        &self,
        from_folder: &str,
        to_folder: &str,
        uids: Vec<&str>,
    ) -> backend::Result<()> {
        let uids = uids.join(",");
        info!("copying imap emails {uids} from folder {from_folder} to folder {to_folder}");

        let from_folder_encoded = encode_utf7(from_folder.to_owned());
        let to_folder_encoded = encode_utf7(to_folder.to_owned());
        trace!("utf7 encoded from folder: {}", from_folder_encoded);
        trace!("utf7 encoded to folder: {}", to_folder_encoded);

        let mut session = self.session()?;
        session
            .select(from_folder_encoded)
            .map_err(|err| Error::SelectFolderError(err, from_folder.to_owned()))?;
        session.uid_copy(&uids, to_folder_encoded).map_err(|err| {
            Error::CopyEmailError(err, uids, from_folder.to_owned(), to_folder.to_owned())
        })?;

        Ok(())
    }

    fn move_emails(
        &self,
        from_folder: &str,
        to_folder: &str,
        uids: Vec<&str>,
    ) -> backend::Result<()> {
        let uids = uids.join(",");
        info!("moving imap emails {uids} from folder {from_folder} to folder {to_folder}");

        let from_folder_encoded = encode_utf7(from_folder.to_owned());
        let to_folder_encoded = encode_utf7(to_folder.to_owned());
        trace!("utf7 encoded from folder: {}", from_folder_encoded);
        trace!("utf7 encoded to folder: {}", to_folder_encoded);

        let mut session = self.session()?;
        session
            .select(from_folder_encoded)
            .map_err(|err| Error::SelectFolderError(err, from_folder.to_owned()))?;
        session.uid_mv(&uids, to_folder_encoded).map_err(|err| {
            Error::MoveEmailError(err, uids, from_folder.to_owned(), to_folder.to_owned())
        })?;

        Ok(())
    }

    fn delete_emails(&self, folder: &str, uids: Vec<&str>) -> backend::Result<()> {
        let trash_folder = self.account_config.trash_folder_alias()?;

        if self.account_config.folder_alias(folder)? == trash_folder {
            self.mark_emails_as_deleted(folder, uids)
        } else {
            self.move_emails(folder, &trash_folder, uids)
        }
    }

    fn add_flags(&self, folder: &str, uids: Vec<&str>, flags: &Flags) -> backend::Result<()> {
        let uids = uids.join(",");
        info!(
            "addings flags {flags} to imap emails {uids} from folder {folder}",
            flags = flags.to_string(),
        );

        let folder_encoded = encode_utf7(folder.to_owned());
        debug!("utf7 encoded folder: {}", folder_encoded);

        let mut session = self.session()?;
        session
            .select(&folder_encoded)
            .map_err(|err| Error::SelectFolderError(err, folder.to_owned()))?;
        session
            .uid_store(&uids, format!("+FLAGS ({})", flags.to_imap_query()))
            .map_err(|err| Error::AddFlagsError(err, flags.to_imap_query(), uids))?;

        Ok(())
    }

    fn set_flags(&self, folder: &str, uids: Vec<&str>, flags: &Flags) -> backend::Result<()> {
        let uids = uids.join(",");
        info!(
            "setting flags {flags} to imap emails {uids} from folder {folder}",
            flags = flags.to_string(),
        );

        let folder_encoded = encode_utf7(folder.to_owned());
        debug!("utf7 encoded folder: {}", folder_encoded);

        let mut session = self.session()?;
        session
            .select(&folder_encoded)
            .map_err(|err| Error::SelectFolderError(err, folder.to_owned()))?;
        session
            .uid_store(&uids, format!("FLAGS ({})", flags.to_imap_query()))
            .map_err(|err| Error::SetFlagsError(err, flags.to_imap_query(), uids))?;

        Ok(())
    }

    fn remove_flags(&self, folder: &str, uids: Vec<&str>, flags: &Flags) -> backend::Result<()> {
        let uids = uids.join(",");
        info!(
            "removing flags {flags} to imap emails {uids} from folder {folder}",
            flags = flags.to_string(),
        );

        let folder_encoded = encode_utf7(folder.to_owned());
        debug!("utf7 encoded folder: {}", folder_encoded);

        let mut session = self.session()?;
        session
            .select(&folder_encoded)
            .map_err(|err| Error::SelectFolderError(err, folder.to_owned()))?;
        session
            .uid_store(&uids, format!("-FLAGS ({})", flags.to_imap_query()))
            .map_err(|err| Error::RemoveFlagsError(err, flags.to_imap_query(), uids))?;

        Ok(())
    }

    fn close(&self) -> backend::Result<()> {
        self.sessions_pool.par_iter().try_for_each(|session| {
            let mut session = session
                .lock()
                .map_err(|err| Error::LockSessionError(err.to_string()))?;
            session.logout().map_err(Error::CloseImapSessionError)
        })?;

        Ok(())
    }

    fn as_any(&'static self) -> &(dyn Any) {
        self
    }
}
