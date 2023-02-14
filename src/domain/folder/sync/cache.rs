pub use rusqlite::Error;

use super::{FoldersName, Result};

const CREATE_FOLDERS_TABLE: &str = "
    CREATE TABLE IF NOT EXISTS folders (
        account TEXT NOT NULL,
        name    TEXT NOT NULL,
        UNIQUE(name, account)
    )
";

const INSERT_FOLDER: &str = "
    INSERT INTO folders
    VALUES (?, ?)
";

const DELETE_FOLDER: &str = "
    DELETE FROM folders
    WHERE account = ?
    AND name = ?
";

const SELECT_ALL_FOLDERS: &str = "
    SELECT name
    FROM folders
    WHERE account = ?
";

const SELECT_FOLDERS: &str = "
    SELECT name
    FROM folders
    WHERE account = ?
    AND name IN (?)
";

pub struct Cache;

impl Cache {
    const LOCAL_SUFFIX: &str = ":cache";

    pub fn init(conn: &mut rusqlite::Connection) -> Result<()> {
        conn.execute(CREATE_FOLDERS_TABLE, ())?;
        Ok(())
    }

    fn list_all_folders<A>(conn: &mut rusqlite::Connection, account: A) -> Result<FoldersName>
    where
        A: AsRef<str>,
    {
        let mut stmt = conn.prepare(SELECT_ALL_FOLDERS)?;
        let folders: Vec<String> = stmt
            .query_map([account.as_ref()], |row| row.get(0))?
            .collect::<rusqlite::Result<_>>()?;

        Ok(FoldersName::from_iter(folders))
    }

    fn list_folders<A, F>(
        conn: &mut rusqlite::Connection,
        account: A,
        folders: F,
    ) -> Result<FoldersName>
    where
        A: AsRef<str>,
        F: AsRef<Vec<String>>,
    {
        // let folders = format!("({})", folders.as_ref().join(","));
        let mut stmt = conn.prepare(SELECT_FOLDERS)?;
        let folders: Vec<String> = stmt
            .query_map([account.as_ref(), &folders.as_ref().join(",")], |row| {
                row.get(0)
            })?
            .collect::<rusqlite::Result<_>>()?;

        Ok(FoldersName::from_iter(folders))
    }

    pub fn list_local_folders<A, F>(
        conn: &mut rusqlite::Connection,
        account: A,
        folders: Option<F>,
    ) -> Result<FoldersName>
    where
        A: ToString,
        F: AsRef<Vec<String>>,
    {
        match folders {
            None => Self::list_all_folders(conn, account.to_string() + Self::LOCAL_SUFFIX),
            Some(folders) => {
                Self::list_folders(conn, account.to_string() + Self::LOCAL_SUFFIX, folders)
            }
        }
    }

    pub fn list_remote_folders<A, F>(
        conn: &mut rusqlite::Connection,
        account: A,
        folders: Option<F>,
    ) -> Result<FoldersName>
    where
        A: AsRef<str>,
        F: AsRef<Vec<String>>,
    {
        match folders {
            None => Self::list_all_folders(conn, account),
            Some(folders) => Self::list_folders(conn, account, folders),
        }
    }

    fn insert_folder<A, F>(tx: &rusqlite::Transaction, account: A, folder: F) -> Result<()>
    where
        A: AsRef<str>,
        F: AsRef<str>,
    {
        tx.execute(INSERT_FOLDER, [account.as_ref(), folder.as_ref()])?;
        Ok(())
    }

    pub fn insert_local_folder<A, F>(
        tx: &rusqlite::Transaction,
        account: A,
        folder: F,
    ) -> Result<()>
    where
        A: ToString,
        F: AsRef<str>,
    {
        Self::insert_folder(tx, account.to_string() + Self::LOCAL_SUFFIX, folder)
    }

    pub fn insert_remote_folder<A, F>(
        tx: &rusqlite::Transaction,
        account: A,
        folder: F,
    ) -> Result<()>
    where
        A: AsRef<str>,
        F: AsRef<str>,
    {
        Self::insert_folder(tx, account, folder)
    }

    fn delete_folder<A, F>(tx: &rusqlite::Transaction, account: A, folder: F) -> Result<()>
    where
        A: AsRef<str>,
        F: AsRef<str>,
    {
        tx.execute(DELETE_FOLDER, [account.as_ref(), folder.as_ref()])?;
        Ok(())
    }

    pub fn delete_local_folder<A, F>(
        tx: &rusqlite::Transaction,
        account: A,
        folder: F,
    ) -> Result<()>
    where
        A: ToString,
        F: AsRef<str>,
    {
        Self::delete_folder(tx, account.to_string() + Self::LOCAL_SUFFIX, folder)
    }

    pub fn delete_remote_folder<A, F>(
        tx: &rusqlite::Transaction,
        account: A,
        folder: F,
    ) -> Result<()>
    where
        A: AsRef<str>,
        F: AsRef<str>,
    {
        Self::delete_folder(tx, account, folder)
    }
}
