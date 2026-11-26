use thiserror::Error;

/// Everything that can go wrong while reading a `.pub` file.
#[derive(Debug, Error)]
pub enum PubError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("required stream `{0}` not found in the compound file")]
    MissingStream(String),
    #[error("unsupported format generation {0} (only generation 1000, Publisher 2003-2010, is supported)")]
    UnsupportedGeneration(u16),
    #[error("malformed data at offset 0x{offset:x}: {message}")]
    Format { offset: usize, message: String },
}

impl PubError {
    pub(crate) fn format(offset: usize, message: impl Into<String>) -> Self {
        PubError::Format {
            offset,
            message: message.into(),
        }
    }
}
