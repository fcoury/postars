use crate::{Flag, Flags};

impl From<&maildir::MailEntry> for Flags {
    fn from(entry: &maildir::MailEntry) -> Self {
        entry.flags().chars().map(Flag::from).collect()
    }
}

impl Flags {
    pub fn to_normalized_string(&self) -> String {
        String::from_iter(self.iter().filter_map(<&Flag as Into<Option<char>>>::into))
    }
}
