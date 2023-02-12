use lettre::address::AddressError;
use log::{info, trace};
use std::{any::Any, borrow::Cow, fs, io, path::PathBuf, result};
use thiserror::Error;

use crate::{
    account, backend, email,
    envelope::notmuch::{envelope, envelopes},
    id_mapper, AccountConfig, Backend, Emails, Envelope, Envelopes, Flag, Flags, Folder, Folders,
    IdMapper, NotmuchConfig,
};

#[derive(Debug, Error)]
pub enum Error {
    #[error("cannot get default notmuch database path")]
    GetDefaultDatabasePathError(#[source] notmuch::Error),
    #[error("cannot store notmuch email")]
    StoreWithFlagsError(maildir::MaildirError),
    #[error("cannot find notmuch email")]
    FindMaildirEmailById,
    #[error("cannot parse notmuch envelope date {1}")]
    ParseTimestampFromEnvelopeError(#[source] mailparse::MailParseError, String),
    #[error("cannot parse notmuch sender {1}")]
    ParseSenderError(#[source] mailparse::MailParseError, String),
    #[error("cannot open notmuch database at {1}")]
    OpenDatabaseError(#[source] rusqlite::Error, PathBuf),
    #[error("cannot find notmuch email")]
    FindEmailError(#[source] notmuch::Error),
    #[error("cannot remove tags from notmuch email {1}")]
    RemoveAllTagsError(#[source] notmuch::Error, String),

    #[error("cannot get notmuch backend from config")]
    GetBackendFromConfigError,
    #[error("cannot get notmuch inner maildir backend")]
    GetMaildirBackendError,
    #[error("cannot parse notmuch message header {1}")]
    ParseMsgHeaderError(#[source] notmuch::Error, String),
    #[error("cannot parse notmuch message date {1}")]
    ParseMsgDateError(#[source] chrono::ParseError, String),
    #[error("cannot find notmuch message header {0}")]
    FindMsgHeaderError(String),
    #[error("cannot find notmuch message sender")]
    FindSenderError,
    #[error("cannot parse notmuch message senders {1}")]
    ParseSendersError(#[source] AddressError, String),
    #[error("cannot open default notmuch database")]
    OpenDefaultNotmuchDatabaseError(#[source] notmuch::Error),
    #[error("cannot open notmuch database at {1}")]
    OpenNotmuchDatabaseError(#[source] notmuch::Error, PathBuf),
    #[error("cannot close notmuch database")]
    CloseDatabaseError(#[source] notmuch::Error),
    #[error("cannot build notmuch query")]
    BuildQueryError(#[source] notmuch::Error),
    #[error("cannot search notmuch envelopes")]
    SearchEnvelopesError(#[source] notmuch::Error),
    #[error("cannot get notmuch envelopes at page {0}")]
    GetEnvelopesOutOfBoundsError(usize),
    #[error("cannot add notmuch mailbox: feature not implemented")]
    AddMboxUnimplementedError,
    #[error("cannot purge notmuch folder: feature not implemented")]
    PurgeFolderUnimplementedError,
    #[error("cannot expunge notmuch folder: feature not implemented")]
    ExpungeFolderUnimplementedError,
    #[error("cannot delete notmuch mailbox: feature not implemented")]
    DeleteFolderUnimplementedError,
    #[error("cannot copy notmuch message: feature not implemented")]
    CopyMsgUnimplementedError,
    #[error("cannot move notmuch message: feature not implemented")]
    MoveMsgUnimplementedError,
    #[error("cannot index notmuch message")]
    IndexFileError(#[source] notmuch::Error),
    #[error("cannot find notmuch message")]
    FindMsgEmptyError,
    #[error("cannot read notmuch raw message from file")]
    ReadMsgError(#[source] io::Error),
    #[error("cannot parse notmuch raw message")]
    ParseMsgError(#[source] mailparse::MailParseError),
    #[error("cannot delete notmuch message")]
    DelMsgError(#[source] notmuch::Error),
    #[error("cannot add notmuch tag")]
    AddTagError(#[source] notmuch::Error),
    #[error("cannot delete notmuch tag")]
    RemoveTagError(#[source] notmuch::Error),

    #[error(transparent)]
    ConfigError(#[from] account::config::Error),
    #[error(transparent)]
    IdMapperError(#[from] id_mapper::Error),
    #[error(transparent)]
    EmailError(#[from] email::Error),
    #[error(transparent)]
    MaildirError(#[from] backend::maildir::Error),
}

pub type Result<T> = result::Result<T, Error>;

/// Represents the Notmuch backend.
pub struct NotmuchBackend<'a> {
    account_config: Cow<'a, AccountConfig>,
    backend_config: Cow<'a, NotmuchConfig>,
    db_path: PathBuf,
    mdir: maildir::Maildir,
}

impl<'a> NotmuchBackend<'a> {
    pub fn new(
        account_config: Cow<'a, AccountConfig>,
        backend_config: Cow<'a, NotmuchConfig>,
    ) -> Result<Self> {
        NotmuchBackendBuilder::new().build(account_config, backend_config)
    }

    pub fn get_default_db_path() -> Result<PathBuf> {
        Ok(notmuch::Database::open_with_config(
            None as Option<PathBuf>,
            notmuch::DatabaseMode::ReadWrite,
            None as Option<PathBuf>,
            None,
        )
        .map_err(Error::OpenDefaultNotmuchDatabaseError)?
        .path()
        .to_owned())
    }

    pub fn with_db<T, F>(&self, f: F) -> Result<T>
    where
        F: Fn(&notmuch::Database) -> Result<T>,
    {
        let db = notmuch::Database::open_with_config(
            Some(&self.backend_config.db_path),
            notmuch::DatabaseMode::ReadWrite,
            None as Option<PathBuf>,
            None,
        )
        .map_err(|err| Error::OpenNotmuchDatabaseError(err, self.backend_config.db_path.clone()))?;
        let res = f(&db)?;
        db.close().map_err(Error::CloseDatabaseError)?;
        Ok(res)
    }

    pub fn id_mapper(&self) -> Result<IdMapper> {
        let db = rusqlite::Connection::open(&self.db_path)
            .map_err(|err| Error::OpenDatabaseError(err, self.db_path.clone()))?;

        let id_mapper = IdMapper::new(db, &self.account_config.name, "all")?;

        Ok(id_mapper)
    }

    fn _search_envelopes(&self, query: &str, page_size: usize, page: usize) -> Result<Envelopes> {
        let id_mapper = self.id_mapper()?;
        let mut envelopes = self.with_db(|db| {
            let query_builder = db.create_query(query).map_err(Error::BuildQueryError)?;
            envelopes::from_raws(
                query_builder
                    .search_messages()
                    .map_err(Error::SearchEnvelopesError)?,
            )
        })?;
        trace!("envelopes: {envelopes:#?}");

        // Calculates pagination boundaries.
        let page_begin = page * page_size;
        trace!("page begin: {:?}", page_begin);
        if page_begin > envelopes.len() {
            return Err(Error::GetEnvelopesOutOfBoundsError(page_begin + 1))?;
        }
        let page_end = envelopes.len().min(page_begin + page_size);
        trace!("page end: {:?}", page_end);

        envelopes.sort_by(|a, b| b.date.partial_cmp(&a.date).unwrap());
        *envelopes = envelopes[page_begin..page_end]
            .iter()
            .map(|envelope| {
                Ok(Envelope {
                    id: id_mapper.get_id(&envelope.internal_id)?,
                    ..envelope.clone()
                })
            })
            .collect::<Result<Vec<_>>>()?;

        Ok(envelopes)
    }
}

impl<'a> Backend for NotmuchBackend<'a> {
    fn name(&self) -> String {
        self.account_config.name.clone()
    }

    fn add_folder(&self, _folder: &str) -> backend::Result<()> {
        Err(Error::AddMboxUnimplementedError)?
    }

    fn list_folders(&self) -> backend::Result<Folders> {
        let mut mboxes = Folders::default();
        for (name, desc) in &self.account_config.folder_aliases {
            mboxes.push(Folder {
                name: name.into(),
                desc: desc.into(),
                ..Folder::default()
            })
        }
        mboxes.sort_by(|a, b| b.name.partial_cmp(&a.name).unwrap());

        trace!("notmuch virtual folders: {:?}", mboxes);
        Ok(mboxes)
    }

    fn expunge_folder(&self, _folder: &str) -> backend::Result<()> {
        Err(Error::PurgeFolderUnimplementedError)?
    }

    fn purge_folder(&self, _folder: &str) -> backend::Result<()> {
        Err(Error::ExpungeFolderUnimplementedError)?
    }

    fn delete_folder(&self, _folder: &str) -> backend::Result<()> {
        Err(Error::DeleteFolderUnimplementedError)?
    }

    fn get_envelope(&self, _folder: &str, id: &str) -> backend::Result<Envelope> {
        info!("getting notmuch envelope by id {id}");

        let internal_id = self.id_mapper()?.get_internal_id(id)?;
        trace!("internal id: {internal_id}");

        let envelope = self.with_db(|db| {
            envelope::from_raw(
                db.find_message(&internal_id)
                    .map_err(Error::FindEmailError)?
                    .ok_or_else(|| Error::FindMsgEmptyError)?,
            )
        })?;
        trace!("envelope: {envelope:#?}");

        Ok(envelope)
    }

    fn get_envelope_internal(&self, _folder: &str, internal_id: &str) -> backend::Result<Envelope> {
        info!("getting notmuch envelope by internal id {internal_id}");

        let envelope = self.with_db(|db| {
            envelope::from_raw(
                db.find_message(&internal_id)
                    .map_err(Error::FindEmailError)?
                    .ok_or_else(|| Error::FindMsgEmptyError)?,
            )
        })?;
        trace!("envelope: {envelope:#?}");

        Ok(envelope)
    }

    fn list_envelopes(
        &self,
        virtual_folder: &str,
        page_size: usize,
        page: usize,
    ) -> backend::Result<Envelopes> {
        info!("listing notmuch envelopes from virtual folder {virtual_folder}");

        let query = self
            .account_config
            .folder_alias(virtual_folder)
            .unwrap_or_else(|_| String::from("all"));
        trace!("query: {query}");

        let envelopes = self._search_envelopes(&query, page_size, page)?;
        trace!("envelopes: {envelopes:#?}");

        Ok(envelopes)
    }

    fn search_envelopes(
        &self,
        virtual_folder: &str,
        query: &str,
        _sort: &str,
        page_size: usize,
        page: usize,
    ) -> backend::Result<Envelopes> {
        info!("searching notmuch envelopes from virtual folder {virtual_folder}");

        let query = if query.is_empty() {
            self.account_config
                .folder_alias(virtual_folder)
                .unwrap_or_else(|_| String::from("all"))
        } else {
            query.to_owned()
        };
        trace!("query: {query}");

        let envelopes = self._search_envelopes(&query, page_size, page)?;
        trace!("envelopes: {envelopes:#?}");

        Ok(envelopes)
    }

    fn add_email(&self, _folder: &str, email: &[u8], flags: &Flags) -> backend::Result<String> {
        info!(
            "adding notmuch email with flags {flags}",
            flags = flags.to_string()
        );

        let mdir_internal_id = self
            .mdir
            .store_cur_with_flags(email, "")
            .map_err(Error::StoreWithFlagsError)?;
        trace!("added email internal maildir id: {mdir_internal_id}");

        let entry = self
            .mdir
            .find(&mdir_internal_id)
            .ok_or(Error::FindMaildirEmailById)?;
        let path = entry.path();
        trace!("path: {path:?}");

        let email = self.with_db(|db| db.index_file(path, None).map_err(Error::IndexFileError))?;
        let internal_id = email.id();
        let id = self.id_mapper()?.insert(&internal_id)?;
        self.add_flags("INBOX", vec![&id], &flags)?;

        Ok(id)
    }

    fn add_email_internal(
        &self,
        _folder: &str,
        email: &[u8],
        flags: &Flags,
    ) -> backend::Result<String> {
        info!(
            "adding notmuch email with flags {flags}",
            flags = flags.to_string()
        );

        let mdir_internal_id = self
            .mdir
            .store_cur_with_flags(email, "")
            .map_err(Error::StoreWithFlagsError)?;
        trace!("added email internal maildir id: {mdir_internal_id}");

        let entry = self
            .mdir
            .find(&mdir_internal_id)
            .ok_or(Error::FindMaildirEmailById)?;
        let path = entry.path();
        trace!("path: {path:?}");

        let email = self.with_db(|db| db.index_file(path, None).map_err(Error::IndexFileError))?;
        let internal_id = email.id();
        self.id_mapper()?.insert(&internal_id)?;
        self.add_flags_internal("INBOX", vec![&internal_id], &flags)?;

        Ok(internal_id.to_string())
    }

    fn preview_emails(&self, _folder: &str, ids: Vec<&str>) -> backend::Result<Emails> {
        info!(
            "previewing notmuch emails by ids {ids}",
            ids = ids.join(", ")
        );

        let id_mapper = self.id_mapper()?;
        let internal_ids: Vec<String> = ids
            .into_iter()
            .map(|id| Ok(id_mapper.get_internal_id(id)?))
            .collect::<Result<_>>()?;
        trace!("internal ids: {internal_ids:?}");

        let emails: Emails = self
            .with_db(|db| {
                internal_ids
                    .iter()
                    .map(|internal_id| {
                        let email_filepath = db
                            .find_message(&internal_id)
                            .map_err(Error::FindEmailError)?
                            .ok_or_else(|| Error::FindMsgEmptyError)?
                            .filename()
                            .to_owned();
                        fs::read(&email_filepath).map_err(Error::ReadMsgError)
                    })
                    .collect::<Result<Vec<_>>>()
            })?
            .into();

        Ok(emails)
    }

    fn preview_emails_internal(
        &self,
        _folder: &str,
        internal_ids: Vec<&str>,
    ) -> backend::Result<Emails> {
        info!(
            "previewing notmuch emails by internal ids {ids}",
            ids = internal_ids.join(", ")
        );

        let emails: Emails = self
            .with_db(|db| {
                internal_ids
                    .iter()
                    .map(|internal_id| {
                        let email_filepath = db
                            .find_message(&internal_id)
                            .map_err(Error::FindEmailError)?
                            .ok_or_else(|| Error::FindMsgEmptyError)?
                            .filename()
                            .to_owned();
                        fs::read(&email_filepath).map_err(Error::ReadMsgError)
                    })
                    .collect::<Result<Vec<_>>>()
            })?
            .into();

        Ok(emails)
    }

    fn get_emails(&self, folder: &str, ids: Vec<&str>) -> backend::Result<Emails> {
        info!("getting notmuch emails by ids {ids}", ids = ids.join(", "));
        let emails = self.preview_emails(folder, ids.clone())?;
        self.add_flags("INBOX", ids, &Flags::from_iter([Flag::Seen]))?;
        Ok(emails)
    }

    fn get_emails_internal(
        &self,
        folder: &str,
        internal_ids: Vec<&str>,
    ) -> backend::Result<Emails> {
        info!(
            "getting notmuch emails by internal ids {ids}",
            ids = internal_ids.join(", ")
        );
        let emails = self.preview_emails_internal(folder, internal_ids.clone())?;
        self.add_flags_internal("INBOX", internal_ids, &Flags::from_iter([Flag::Seen]))?;
        Ok(emails)
    }

    fn copy_emails(
        &self,
        _from_dir: &str,
        _to_dir: &str,
        _short_hashes: Vec<&str>,
    ) -> backend::Result<()> {
        // How to deal with duplicate Message-ID?
        Err(Error::CopyMsgUnimplementedError)?
    }

    fn copy_emails_internal(
        &self,
        _from_dir: &str,
        _to_dir: &str,
        _internal_ids: Vec<&str>,
    ) -> backend::Result<()> {
        // How to deal with duplicate Message-ID?
        Err(Error::CopyMsgUnimplementedError)?
    }

    fn move_emails(
        &self,
        _from_dir: &str,
        _to_dir: &str,
        _short_hashes: Vec<&str>,
    ) -> backend::Result<()> {
        Err(Error::MoveMsgUnimplementedError)?
    }

    fn move_emails_internal(
        &self,
        _from_dir: &str,
        _to_dir: &str,
        _internal_ids: Vec<&str>,
    ) -> backend::Result<()> {
        Err(Error::MoveMsgUnimplementedError)?
    }

    fn delete_emails(&self, _folder: &str, ids: Vec<&str>) -> backend::Result<()> {
        info!("deleting notmuch emails by ids {ids}", ids = ids.join(", "));

        let id_mapper = self.id_mapper()?;
        let internal_ids: Vec<String> = ids
            .into_iter()
            .map(|id| Ok(id_mapper.get_internal_id(id)?))
            .collect::<Result<_>>()?;
        trace!("internal ids: {internal_ids:?}");

        self.with_db(|db| {
            internal_ids.iter().try_for_each(|internal_id| {
                let path = db
                    .find_message(&internal_id)
                    .map_err(Error::FindEmailError)?
                    .ok_or_else(|| Error::FindMsgEmptyError)?
                    .filename()
                    .to_owned();
                db.remove_message(path).map_err(Error::DelMsgError)
            })
        })?;

        Ok(())
    }

    fn delete_emails_internal(
        &self,
        _folder: &str,
        internal_ids: Vec<&str>,
    ) -> backend::Result<()> {
        info!(
            "deleting notmuch emails by internal ids {ids}",
            ids = internal_ids.join(", ")
        );

        self.with_db(|db| {
            internal_ids.iter().try_for_each(|internal_id| {
                let path = db
                    .find_message(&internal_id)
                    .map_err(Error::FindEmailError)?
                    .ok_or_else(|| Error::FindMsgEmptyError)?
                    .filename()
                    .to_owned();
                db.remove_message(path).map_err(Error::DelMsgError)
            })
        })?;

        Ok(())
    }

    fn add_flags(
        &self,
        _virtual_folder: &str,
        ids: Vec<&str>,
        flags: &Flags,
    ) -> backend::Result<()> {
        info!(
            "adding notmuch flags {flags} by ids {ids}",
            flags = flags.to_string(),
            ids = ids.join(", "),
        );

        let id_mapper = self.id_mapper()?;
        let internal_ids: Vec<String> = ids
            .into_iter()
            .map(|id| Ok(id_mapper.get_internal_id(id)?))
            .collect::<Result<_>>()?;
        trace!("internal ids: {internal_ids:?}");

        let query = format!("mid:\"/^({})$/\"", internal_ids.join("|"));
        trace!("query: {query}");

        self.with_db(|db| {
            let query_builder = db.create_query(&query).map_err(Error::BuildQueryError)?;
            let emails = query_builder
                .search_messages()
                .map_err(Error::SearchEnvelopesError)?;

            for email in emails {
                for flag in flags.iter() {
                    email
                        .add_tag(&flag.to_string())
                        .map_err(Error::AddTagError)?;
                }
            }

            Ok(())
        })?;

        Ok(())
    }

    fn add_flags_internal(
        &self,
        _folder: &str,
        internal_ids: Vec<&str>,
        flags: &Flags,
    ) -> backend::Result<()> {
        info!(
            "adding notmuch flags {flags} by internal_ids {ids}",
            flags = flags.to_string(),
            ids = internal_ids.join(", "),
        );

        let query = format!("mid:\"/^({})$/\"", internal_ids.join("|"));
        trace!("query: {query}");

        self.with_db(|db| {
            let query_builder = db.create_query(&query).map_err(Error::BuildQueryError)?;
            let emails = query_builder
                .search_messages()
                .map_err(Error::SearchEnvelopesError)?;

            for email in emails {
                for flag in flags.iter() {
                    email
                        .add_tag(&flag.to_string())
                        .map_err(Error::AddTagError)?;
                }
            }

            Ok(())
        })?;

        Ok(())
    }

    fn set_flags(&self, _folder: &str, ids: Vec<&str>, flags: &Flags) -> backend::Result<()> {
        info!(
            "setting notmuch flags {flags} by ids {ids}",
            flags = flags.to_string(),
            ids = ids.join(", "),
        );

        let id_mapper = self.id_mapper()?;
        let internal_ids: Vec<String> = ids
            .into_iter()
            .map(|id| Ok(id_mapper.get_internal_id(id)?))
            .collect::<Result<_>>()?;
        trace!("internal ids: {internal_ids:?}");

        let query = format!("mid:\"/^({})$/\"", internal_ids.join("|"));
        trace!("query: {query}");

        self.with_db(|db| {
            let query_builder = db.create_query(&query).map_err(Error::BuildQueryError)?;
            let emails = query_builder
                .search_messages()
                .map_err(Error::SearchEnvelopesError)?;

            for email in emails {
                email
                    .remove_all_tags()
                    .map_err(|err| Error::RemoveAllTagsError(err, email.id().to_string()))?;

                for flag in flags.iter() {
                    email
                        .add_tag(&flag.to_string())
                        .map_err(Error::AddTagError)?;
                }
            }

            Ok(())
        })?;

        Ok(())
    }

    fn set_flags_internal(
        &self,
        _folder: &str,
        internal_ids: Vec<&str>,
        flags: &Flags,
    ) -> backend::Result<()> {
        info!(
            "setting notmuch flags {flags} by internal_ids {ids}",
            flags = flags.to_string(),
            ids = internal_ids.join(", "),
        );

        let query = format!("mid:\"/^({})$/\"", internal_ids.join("|"));
        trace!("query: {query}");

        self.with_db(|db| {
            let query_builder = db.create_query(&query).map_err(Error::BuildQueryError)?;
            let emails = query_builder
                .search_messages()
                .map_err(Error::SearchEnvelopesError)?;

            for email in emails {
                email
                    .remove_all_tags()
                    .map_err(|err| Error::RemoveAllTagsError(err, email.id().to_string()))?;

                for flag in flags.iter() {
                    email
                        .add_tag(&flag.to_string())
                        .map_err(Error::AddTagError)?;
                }
            }

            Ok(())
        })?;

        Ok(())
    }

    fn remove_flags(
        &self,
        _virtual_folder: &str,
        ids: Vec<&str>,
        flags: &Flags,
    ) -> backend::Result<()> {
        info!(
            "removing notmuch flags {flags} by ids {ids}",
            flags = flags.to_string(),
            ids = ids.join(", "),
        );

        let id_mapper = self.id_mapper()?;
        let internal_ids: Vec<String> = ids
            .into_iter()
            .map(|id| Ok(id_mapper.get_internal_id(id)?))
            .collect::<Result<_>>()?;
        trace!("internal ids: {internal_ids:?}");

        let query = format!("mid:\"/^({})$/\"", internal_ids.join("|"));
        trace!("query: {query}");

        self.with_db(|db| {
            let query_builder = db.create_query(&query).map_err(Error::BuildQueryError)?;
            let emails = query_builder
                .search_messages()
                .map_err(Error::SearchEnvelopesError)?;

            for email in emails {
                for flag in flags.iter() {
                    email
                        .remove_tag(&flag.to_string())
                        .map_err(Error::RemoveTagError)?;
                }
            }

            Ok(())
        })?;

        Ok(())
    }

    fn remove_flags_internal(
        &self,
        _folder: &str,
        internal_ids: Vec<&str>,
        flags: &Flags,
    ) -> backend::Result<()> {
        info!(
            "removing notmuch flags {flags} by internal_ids {ids}",
            flags = flags.to_string(),
            ids = internal_ids.join(", "),
        );

        let query = format!("mid:\"/^({})$/\"", internal_ids.join("|"));
        trace!("query: {query}");

        self.with_db(|db| {
            let query_builder = db.create_query(&query).map_err(Error::BuildQueryError)?;
            let emails = query_builder
                .search_messages()
                .map_err(Error::SearchEnvelopesError)?;

            for email in emails {
                for flag in flags.iter() {
                    email
                        .remove_tag(&flag.to_string())
                        .map_err(Error::RemoveTagError)?;
                }
            }

            Ok(())
        })?;

        Ok(())
    }

    fn as_any(&self) -> &(dyn Any + 'a) {
        self
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct NotmuchBackendBuilder {
    db_path: Option<PathBuf>,
}

impl NotmuchBackendBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn db_path<P>(mut self, path: P) -> Self
    where
        P: Into<PathBuf>,
    {
        self.db_path = Some(path.into());
        self
    }

    pub fn build<'a>(
        self,
        account_config: Cow<'a, AccountConfig>,
        backend_config: Cow<'a, NotmuchConfig>,
    ) -> Result<NotmuchBackend<'a>> {
        let mdir = maildir::Maildir::from(backend_config.db_path.clone());

        let db_path = self
            .db_path
            .unwrap_or_else(|| backend_config.db_path.join(".database.sqlite"));

        Ok(NotmuchBackend {
            account_config,
            backend_config,
            db_path,
            mdir,
        })
    }
}
