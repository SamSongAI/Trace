use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum WriteMode {
    #[default]
    Dimension,
    Thread,
    File,
}

impl WriteMode {
    pub fn next(self) -> Self {
        match self {
            Self::Dimension => Self::Thread,
            Self::Thread => Self::File,
            Self::File => Self::Dimension,
        }
    }

    pub fn previous(self) -> Self {
        match self {
            Self::Dimension => Self::File,
            Self::Thread => Self::Dimension,
            Self::File => Self::Thread,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_matches_mac() {
        assert_eq!(WriteMode::default(), WriteMode::Dimension);
    }

    #[test]
    fn serializes_as_camel_case_raw_values() {
        assert_eq!(
            serde_json::to_string(&WriteMode::Dimension).unwrap(),
            "\"dimension\""
        );
        assert_eq!(
            serde_json::to_string(&WriteMode::Thread).unwrap(),
            "\"thread\""
        );
        assert_eq!(serde_json::to_string(&WriteMode::File).unwrap(), "\"file\"");
    }

    #[test]
    fn round_trip_through_json() {
        for mode in [WriteMode::Dimension, WriteMode::Thread, WriteMode::File] {
            let json = serde_json::to_string(&mode).unwrap();
            let decoded: WriteMode = serde_json::from_str(&json).unwrap();
            assert_eq!(decoded, mode);
        }
    }

    #[test]
    fn next_cycles_dimension_thread_file() {
        assert_eq!(WriteMode::Dimension.next(), WriteMode::Thread);
        assert_eq!(WriteMode::Thread.next(), WriteMode::File);
        assert_eq!(WriteMode::File.next(), WriteMode::Dimension);
    }

    #[test]
    fn previous_is_inverse_of_next() {
        for mode in [WriteMode::Dimension, WriteMode::Thread, WriteMode::File] {
            assert_eq!(mode.next().previous(), mode);
        }
    }
}
