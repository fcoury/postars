//! Backend module.
//!
//! This module exposes the backend trait, which can be used to create
//! custom backend implementations.

use log::info;
use proc_lock::{lock, LockPath};
use std::{any::Any, borrow::Cow, fmt, io, result};
use thiserror::Error;

use crate::{
    account, backend, email, envelope, folder, id_mapper, AccountConfig, BackendConfig, Emails,
    Envelope, Envelopes, Flag, Flags, Folders, ImapBackendBuilder, MaildirBackend, MaildirConfig,
};

#[cfg(feature = "notmuch-backend")]
use crate::NotmuchBackend;

#[derive(Debug, Error)]
pub enum Error {
    #[error("cannot build backend with an empty config")]
    BuildBackendError,
    #[error("cannot lock synchronization for account {1}")]
    SyncAccountLockError(io::Error, String),
    #[error("synchronization not enabled for account {0}")]
    SyncNotEnabled(String),
    #[error(transparent)]
    EmailError(#[from] email::Error),
    #[error(transparent)]
    IdMapper(#[from] id_mapper::Error),
    #[error(transparent)]
    ConfigError(#[from] account::config::Error),
    #[error(transparent)]
    SyncFoldersError(#[from] folder::sync::Error),
    #[error(transparent)]
    SyncEnvelopesError(#[from] envelope::sync::Error),
    #[error(transparent)]
    SqliteError(#[from] rusqlite::Error),

    #[cfg(feature = "imap-backend")]
    #[error(transparent)]
    ImapBackendError(#[from] backend::imap::Error),
    #[error(transparent)]
    MaildirBackendError(#[from] backend::maildir::Error),
    #[cfg(feature = "notmuch-backend")]
    #[error(transparent)]
    NotmuchBackendError(#[from] backend::notmuch::Error),
}

pub type Result<T> = result::Result<T, Error>;

pub trait Backend: Sync + Send {
    fn name(&self) -> String;

    fn add_folder(&self, folder: &str) -> Result<()>;
    fn list_folders(&self) -> Result<Folders>;
    fn expunge_folder(&self, folder: &str) -> Result<()>;
    fn purge_folder(&self, folder: &str) -> Result<()>;
    fn delete_folder(&self, folder: &str) -> Result<()>;

    fn get_envelope(&self, folder: &str, id: &str) -> Result<Envelope>;
    fn get_envelope_internal(&self, folder: &str, internal_id: &str) -> Result<Envelope> {
        self.get_envelope(folder, internal_id)
    }

    fn list_envelopes(&self, folder: &str, page_size: usize, page: usize) -> Result<Envelopes>;
    fn search_envelopes(
        &self,
        folder: &str,
        query: &str,
        sort: &str,
        page_size: usize,
        page: usize,
    ) -> Result<Envelopes>;

    fn add_email(&self, folder: &str, email: &[u8], flags: &Flags) -> Result<String>;
    fn add_email_internal(&self, folder: &str, email: &[u8], flags: &Flags) -> Result<String> {
        self.add_email(folder, email, flags)
    }

    fn preview_emails(&self, folder: &str, ids: Vec<&str>) -> Result<Emails>;
    fn preview_emails_internal(&self, folder: &str, internal_ids: Vec<&str>) -> Result<Emails> {
        self.preview_emails(folder, internal_ids)
    }

    fn get_emails(&self, folder: &str, ids: Vec<&str>) -> Result<Emails>;
    fn get_emails_internal(&self, folder: &str, internal_ids: Vec<&str>) -> Result<Emails> {
        self.get_emails(folder, internal_ids)
    }

    fn copy_emails(&self, from_folder: &str, to_folder: &str, ids: Vec<&str>) -> Result<()>;
    fn copy_emails_internal(
        &self,
        from_folder: &str,
        to_folder: &str,
        internal_ids: Vec<&str>,
    ) -> Result<()> {
        self.copy_emails(from_folder, to_folder, internal_ids)
    }

    fn move_emails(&self, from_folder: &str, to_folder: &str, ids: Vec<&str>) -> Result<()>;
    fn move_emails_internal(
        &self,
        from_folder: &str,
        to_folder: &str,
        internal_ids: Vec<&str>,
    ) -> Result<()> {
        self.move_emails(from_folder, to_folder, internal_ids)
    }

    fn mark_emails_as_deleted(&self, folder: &str, ids: Vec<&str>) -> backend::Result<()> {
        self.add_flags(folder, ids, &Flags::from_iter([Flag::Deleted]))
    }
    fn mark_emails_as_deleted_internal(
        &self,
        folder: &str,
        internal_ids: Vec<&str>,
    ) -> backend::Result<()> {
        self.add_flags_internal(folder, internal_ids, &Flags::from_iter([Flag::Deleted]))
    }

    fn delete_emails(&self, folder: &str, ids: Vec<&str>) -> Result<()>;
    fn delete_emails_internal(&self, folder: &str, internal_ids: Vec<&str>) -> Result<()> {
        self.delete_emails(folder, internal_ids)
    }

    fn add_flags(&self, folder: &str, ids: Vec<&str>, flags: &Flags) -> Result<()>;
    fn add_flags_internal(
        &self,
        folder: &str,
        internal_ids: Vec<&str>,
        flags: &Flags,
    ) -> Result<()> {
        self.add_flags(folder, internal_ids, flags)
    }

    fn set_flags(&self, folder: &str, ids: Vec<&str>, flags: &Flags) -> Result<()>;
    fn set_flags_internal(
        &self,
        folder: &str,
        internal_ids: Vec<&str>,
        flags: &Flags,
    ) -> Result<()> {
        self.set_flags(folder, internal_ids, flags)
    }

    fn remove_flags(&self, folder: &str, ids: Vec<&str>, flags: &Flags) -> Result<()>;
    fn remove_flags_internal(
        &self,
        folder: &str,
        internal_ids: Vec<&str>,
        flags: &Flags,
    ) -> Result<()> {
        self.remove_flags(folder, internal_ids, flags)
    }

    fn close(&self) -> Result<()> {
        Ok(())
    }

    // INFO: for downcasting purpose
    fn as_any(&'static self) -> &(dyn Any);
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BackendSyncProgressEvent {
    GetLocalCachedFolders,
    GetLocalFolders,
    GetRemoteCachedFolders,
    GetRemoteFolders,
    BuildFoldersPatch,
    ProcessFoldersPatch(usize),
    ProcessFolderHunk(String),

    StartEnvelopesSync(String, usize, usize),
    GetLocalCachedEnvelopes,
    GetLocalEnvelopes,
    GetRemoteCachedEnvelopes,
    GetRemoteEnvelopes,
    BuildEnvelopesPatch,
    ProcessEnvelopesPatch(usize),
    ProcessEnvelopeHunk(String),
}

impl fmt::Display for BackendSyncProgressEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::GetLocalCachedFolders => write!(f, "Getting local cached folders"),
            Self::GetLocalFolders => write!(f, "Getting local folders"),
            Self::GetRemoteCachedFolders => write!(f, "Getting remote cached folders"),
            Self::GetRemoteFolders => write!(f, "Getting remote folders"),
            Self::BuildFoldersPatch => write!(f, "Building folders patch"),
            Self::ProcessFoldersPatch(n) => write!(f, "Processing {n} hunks of folders patch"),
            Self::ProcessFolderHunk(s) => write!(f, "Processing folder hunk: {s}"),

            Self::StartEnvelopesSync(_, _, _) => write!(f, "Starting envelopes synchronization"),
            Self::GetLocalCachedEnvelopes => write!(f, "Getting local cached envelopes"),
            Self::GetLocalEnvelopes => write!(f, "Getting local envelopes"),
            Self::GetRemoteCachedEnvelopes => write!(f, "Getting remote cached envelopes"),
            Self::GetRemoteEnvelopes => write!(f, "Getting remote envelopes"),
            Self::BuildEnvelopesPatch => write!(f, "Building envelopes patch"),
            Self::ProcessEnvelopesPatch(n) => write!(f, "Processing {n} hunks of envelopes patch"),
            Self::ProcessEnvelopeHunk(s) => write!(f, "Processing envelope hunk: {s}"),
        }
    }
}

#[derive(Debug, Default)]
pub struct BackendSyncReport {
    pub folders: folder::sync::FoldersName,
    pub folders_patch: Vec<(folder::sync::Hunk, Option<folder::sync::Error>)>,
    pub folders_cache_patch: (Vec<folder::sync::CacheHunk>, Option<folder::sync::Error>),
    pub envelopes_patch: Vec<(envelope::sync::BackendHunk, Option<envelope::sync::Error>)>,
    pub envelopes_cache_patch: (Vec<envelope::sync::CacheHunk>, Vec<envelope::sync::Error>),
}

pub struct BackendSyncBuilder<'a> {
    account_config: &'a AccountConfig,
    on_progress: Box<dyn Fn(BackendSyncProgressEvent) -> Result<()> + Sync + Send + 'a>,
    folders: Option<Vec<String>>,
    dry_run: bool,
}

impl<'a> BackendSyncBuilder<'a> {
    pub fn new(account_config: &'a AccountConfig) -> Self {
        Self {
            account_config,
            on_progress: Box::new(|_| Ok(())),
            folders: None,
            dry_run: false,
        }
    }

    pub fn on_progress<F>(mut self, f: F) -> Self
    where
        F: Fn(BackendSyncProgressEvent) -> Result<()> + Sync + Send + 'a,
    {
        self.on_progress = Box::new(f);
        self
    }

    pub fn all_folders(mut self) -> Self {
        self.folders = None;
        self
    }

    pub fn only_folder<F>(mut self, folder: F) -> Self
    where
        F: ToString,
    {
        self.folders = match self.folders {
            None => Some(vec![folder.to_string()]),
            Some(mut folders) => {
                folders.push(folder.to_string());
                Some(folders)
            }
        };
        self
    }

    pub fn only_folders<F, I>(mut self, folders: I) -> Self
    where
        F: ToString,
        I: IntoIterator<Item = F>,
    {
        self.folders = Some(
            folders
                .into_iter()
                .map(|folder| folder.to_string())
                .collect::<Vec<_>>(),
        );
        self
    }

    pub fn dry_run(mut self, dry_run: bool) -> Self {
        self.dry_run = dry_run;
        self
    }

    pub fn sync(&self, remote: &dyn Backend) -> Result<BackendSyncReport> {
        let account = &self.account_config.name;
        if !self.account_config.sync {
            return Err(Error::SyncNotEnabled(account.clone()));
        }

        info!("starting synchronization");
        let progress = &self.on_progress;
        let sync_dir = self.account_config.sync_dir()?;
        let lock_path = LockPath::Tmp(format!("himalaya-sync-{}.lock", account));
        let guard =
            lock(&lock_path).map_err(|err| Error::SyncAccountLockError(err, account.to_owned()))?;

        // init SQLite cache

        let mut conn = rusqlite::Connection::open(sync_dir.join(".sync.sqlite"))?;

        folder::sync::Cache::init(&mut conn)?;
        envelope::sync::Cache::init(&mut conn)?;

        // init local Maildir

        let local = MaildirBackend::new(
            Cow::Borrowed(self.account_config),
            Cow::Owned(MaildirConfig {
                root_dir: sync_dir.clone(),
            }),
        )?;

        let folders_sync_report = folder::SyncBuilder::new(self.account_config)
            .on_progress(|data| Ok(progress(data).map_err(Box::new)?))
            .folders(self.folders.clone())
            .dry_run(self.dry_run)
            .sync(&mut conn, &local, remote)?;

        let envelopes = envelope::SyncBuilder::new(self.account_config)
            .on_progress(|data| Ok(progress(data).map_err(Box::new)?))
            .dry_run(self.dry_run);

        let mut envelopes_patch = Vec::new();
        let mut envelopes_cache_patch = (Vec::new(), Vec::new());

        for (folder_num, folder) in folders_sync_report.folders.iter().enumerate() {
            progress(BackendSyncProgressEvent::StartEnvelopesSync(
                folder.clone(),
                folder_num + 1,
                folders_sync_report.folders.len(),
            ))?;
            let report = envelopes.sync(folder, &mut conn, &local, remote)?;
            envelopes_patch.extend(report.patch);
            envelopes_cache_patch.0.extend(report.cache_patch.0);
            if let Some(err) = report.cache_patch.1 {
                envelopes_cache_patch.1.push(err);
            }

            local.expunge_folder(folder)?;
            remote.expunge_folder(folder)?;
        }

        drop(guard);

        Ok(BackendSyncReport {
            folders: folders_sync_report.folders,
            folders_patch: folders_sync_report.patch,
            folders_cache_patch: folders_sync_report.cache_patch,
            envelopes_patch,
            envelopes_cache_patch,
        })
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct BackendBuilder {
    sessions_pool_size: usize,
    disable_cache: bool,
}

impl<'a> BackendBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn sessions_pool_size(mut self, pool_size: usize) -> Self {
        self.sessions_pool_size = pool_size;
        self
    }

    pub fn disable_cache(mut self, disable_cache: bool) -> Self {
        self.disable_cache = disable_cache;
        self
    }

    pub fn build(
        &self,
        account_config: &'a AccountConfig,
        backend_config: &'a BackendConfig,
    ) -> Result<Box<dyn Backend + 'a>> {
        match backend_config {
            #[cfg(feature = "imap-backend")]
            BackendConfig::Imap(imap_config) if !account_config.sync || self.disable_cache => {
                Ok(Box::new(
                    ImapBackendBuilder::new()
                        .pool_size(self.sessions_pool_size)
                        .build(Cow::Borrowed(account_config), Cow::Borrowed(imap_config))?,
                ))
            }
            #[cfg(feature = "imap-backend")]
            BackendConfig::Imap(_) => Ok(Box::new(MaildirBackend::new(
                Cow::Borrowed(account_config),
                Cow::Owned(MaildirConfig {
                    root_dir: account_config.sync_dir()?,
                }),
            )?)),
            BackendConfig::Maildir(maildir_config) => Ok(Box::new(MaildirBackend::new(
                Cow::Borrowed(account_config),
                Cow::Borrowed(maildir_config),
            )?)),
            #[cfg(feature = "notmuch-backend")]
            BackendConfig::Notmuch(notmuch_config) => Ok(Box::new(NotmuchBackend::new(
                Cow::Borrowed(account_config),
                Cow::Borrowed(notmuch_config),
            )?)),
            BackendConfig::None => Err(Error::BuildBackendError),
        }
    }
}
