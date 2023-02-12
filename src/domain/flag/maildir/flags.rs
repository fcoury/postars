use crate::{Flag, Flags};

impl From<&maildir::MailEntry> for Flags {
    fn from(entry: &maildir::MailEntry) -> Self {
        entry.flags().chars().map(Flag::from).collect()
    }
}

pub fn to_normalized_string(flags: &Flags) -> String {
    String::from_iter(flags.iter().filter_map(<&Flag as Into<Option<char>>>::into))
}
