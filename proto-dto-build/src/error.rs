// SPDX-License-Identifier: AGPL-3.0-or-later

//! The error type returned by [`Mirror::generate`](crate::Mirror::generate).

use std::fmt;
use std::path::PathBuf;

/// Everything that can stop the mirrors from being generated.
///
/// Every variant names the protobuf message or field involved, so the message
/// printed by a failing build script points at the `.proto` to change.
#[derive(Debug)]
#[non_exhaustive]
pub enum Error {
    /// Reading the descriptor set or writing an output file failed.
    Io {
        /// File that was being read or written.
        path: PathBuf,
        /// Underlying I/O error.
        source: std::io::Error,
    },
    /// The descriptor set bytes are not a `google.protobuf.FileDescriptorSet`.
    Decode(prost::DecodeError),
    /// The builder was configured inconsistently (no output file, a message
    /// listed twice, two messages with the same Rust name, ...).
    Config(String),
    /// A requested message is not in the descriptor set.
    MessageNotFound {
        /// Fully-qualified protobuf name, e.g. `.rf_grpc.FolderDTO`.
        message: String,
    },
    /// A field refers to a message that is not mirrored itself, so the
    /// bridge could not translate it.
    NotMirrored {
        /// Fully-qualified name of the message that owns the field.
        message: String,
        /// Field name as written in the `.proto` file.
        field: String,
        /// Fully-qualified name of the field's message type.
        type_name: String,
    },
    /// The field uses a protobuf feature the mirror cannot express.
    Unsupported {
        /// Fully-qualified name of the message that owns the field.
        message: String,
        /// Field name as written in the `.proto` file, or empty when the
        /// whole message is unsupported.
        field: String,
        /// What is not supported (`oneof`, `group`, ...).
        what: &'static str,
    },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io { path, source } => write!(f, "{}: {source}", path.display()),
            Self::Decode(error) => write!(f, "invalid descriptor set: {error}"),
            Self::Config(reason) => write!(f, "invalid mirror configuration: {reason}"),
            Self::MessageNotFound { message } => {
                write!(f, "{message} is not in the descriptor set")
            }
            Self::NotMirrored {
                message,
                field,
                type_name,
            } => write!(
                f,
                "{message}.{field} uses {type_name}, which has no mirror; add it to a mirror file"
            ),
            Self::Unsupported {
                message,
                field,
                what,
            } if field.is_empty() => write!(f, "{message}: {what} is not supported"),
            Self::Unsupported {
                message,
                field,
                what,
            } => write!(f, "{message}.{field}: {what} is not supported"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            Self::Decode(error) => Some(error),
            _ => None,
        }
    }
}

impl From<prost::DecodeError> for Error {
    fn from(error: prost::DecodeError) -> Self {
        Self::Decode(error)
    }
}
