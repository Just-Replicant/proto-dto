// SPDX-License-Identifier: AGPL-3.0-or-later

//! Error returned when a typed message cannot be read from an envelope.

use core::fmt;

/// Why an envelope's `attributes` could not be packed, unpacked or converted.
///
/// This is a plain stack enum. Variants carry the data needed to explain the
/// failure and [`Display`](fmt::Display) formats it lazily, so returning the
/// error never builds a `String`.
#[derive(Debug)]
#[non_exhaustive]
pub enum AttributeError {
    /// The envelope has no `attributes`.
    Missing,
    /// The envelope's discriminant is not the one the caller asked for.
    TypeMismatch {
        /// Protobuf name of the requested discriminant, e.g. `"DEVICE"`.
        expected: &'static str,
        /// Raw discriminant found in the envelope.
        actual: i32,
    },
    /// The discriminant has no registered payload, so `attributes` cannot be
    /// interpreted (JSON path only).
    UnsupportedType(i32),
    /// The `Any` bytes are not a valid encoding of the requested message.
    Decode(prost::DecodeError),
    /// The JSON `attributes` object does not deserialize into the message.
    #[cfg(feature = "serde")]
    Json {
        /// Protobuf message name of the payload, e.g. `"DeviceDTO"`.
        label: &'static str,
        /// The underlying `serde_json` error.
        source: serde_json::Error,
    },
}

impl fmt::Display for AttributeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Missing => formatter.write_str("DtoData.attributes is missing"),
            Self::TypeMismatch { expected, actual } => {
                write!(formatter, "DtoData.type {actual} does not match {expected}")
            }
            Self::UnsupportedType(actual) => {
                write!(
                    formatter,
                    "unsupported DtoData.type {actual} for attributes"
                )
            }
            Self::Decode(error) => {
                write!(formatter, "failed to decode DtoData.attributes: {error}")
            }
            #[cfg(feature = "serde")]
            Self::Json { label, source } => {
                write!(formatter, "invalid {label} attributes: {source}")
            }
        }
    }
}

impl core::error::Error for AttributeError {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        match self {
            Self::Decode(error) => Some(error),
            #[cfg(feature = "serde")]
            Self::Json { source, .. } => Some(source),
            Self::Missing | Self::TypeMismatch { .. } | Self::UnsupportedType(_) => None,
        }
    }
}

impl From<prost::DecodeError> for AttributeError {
    fn from(error: prost::DecodeError) -> Self {
        Self::Decode(error)
    }
}

#[cfg(test)]
mod tests {
    use super::AttributeError;
    use core::error::Error as _;

    #[test]
    fn display_messages_are_stable() {
        assert_eq!(
            AttributeError::Missing.to_string(),
            "DtoData.attributes is missing"
        );
        assert_eq!(
            AttributeError::TypeMismatch {
                expected: "DEVICE",
                actual: 2
            }
            .to_string(),
            "DtoData.type 2 does not match DEVICE"
        );
        assert_eq!(
            AttributeError::UnsupportedType(99).to_string(),
            "unsupported DtoData.type 99 for attributes"
        );
    }

    #[test]
    fn decode_error_is_the_source() {
        let decode = <prost_types::Any as prost::Message>::decode(&[0xff_u8][..])
            .expect_err("truncated varint");
        let error = AttributeError::from(decode);
        assert!(error.source().is_some());
        assert!(
            error
                .to_string()
                .starts_with("failed to decode DtoData.attributes")
        );
    }

    #[test]
    fn stays_small() {
        // A `&'static str` plus one boxed error and the tag: nothing owned
        // inline, so returning the error is a small copy.
        assert!(core::mem::size_of::<AttributeError>() <= 4 * core::mem::size_of::<usize>());
    }
}
