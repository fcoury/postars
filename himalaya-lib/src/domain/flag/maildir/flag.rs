use crate::Flag;

impl From<char> for Flag {
    fn from(c: char) -> Self {
        match c {
            'r' | 'R' => Flag::Answered,
            's' | 'S' => Flag::Seen,
            't' | 'T' => Flag::Deleted,
            'd' | 'D' => Flag::Draft,
            'f' | 'F' => Flag::Flagged,
            'p' | 'P' => Flag::Custom(String::from("Passed")),
            flag => Flag::Custom(flag.to_string()),
        }
    }
}

impl Into<Option<char>> for &Flag {
    fn into(self) -> Option<char> {
        match self {
            Flag::Answered => Some('R'),
            Flag::Seen => Some('S'),
            Flag::Deleted => Some('T'),
            Flag::Draft => Some('D'),
            Flag::Flagged => Some('F'),
            _ => None,
        }
    }
}
