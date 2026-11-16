// SPDX-License-Identifier: AGPL-3.0-or-later

//! Typed envelopes for protobuf DTOs carried in a `google.protobuf.Any`.
//!
//! Many RPC schemas wrap a domain message in a generic envelope:
//!
//! ```proto
//! message DTOData {
//!   enum DataType { UNSPECIFIED = 0; FILE = 1; FOLDER = 2; DEVICE = 3; HARD_DRIVE = 4; }
//!   optional int32 id = 1;
//!   DataType type = 2;
//!   google.protobuf.Any attributes = 3;
//! }
//! ```
//!
//! This crate turns one table of `(message, discriminant, type URL, label)`
//! rows into inherent pack/unpack methods and a JSON shape where `attributes`
//! is the entity object instead of a raw `Any`. Everything is generated with
//! `macro_rules!` at the call site, because a procedural macro cannot see the
//! prost structs that a build script writes into `OUT_DIR`.
//!
//! * [`envelope!`] emits `from_*` / `try_as_*` methods on the envelope, a
//!   `TYPE_URL` constant on each payload message and, with the `serde`
//!   feature, the `DtoDataJson` conversions. Dispatch is a single `match` on
//!   the protobuf discriminant: no registry, no `OnceLock`, no function
//!   pointers.
//! * [`frb_crud_service!`] emits a `flutter_rust_bridge` opaque wrapper around
//!   a CRUD application type. The paths it calls are plain tokens, so this
//!   crate does not depend on the bridge or on the application crates.
//!
//! The companion build-only crate
//! [`proto-dto-build`](https://docs.rs/proto-dto-build) renders the
//! `flutter_rust_bridge` mirror structs for these DTOs from a protobuf
//! descriptor set. It is a separate crate so that depending on `proto-dto`
//! never compiles its code generator.
//!
//! # Features
//!
//! * `serde`: [`DtoDataJson`] and the JSON half of [`envelope!`].
//!
//! # License
//!
//! AGPL-3.0-or-later. A binary that links a crate expanding these macros is a
//! combined work under the same license.

#![cfg_attr(docsrs, feature(doc_cfg))]

mod envelope;
mod error;
#[cfg(feature = "serde")]
mod json;
mod service;

pub use error::AttributeError;
#[cfg(feature = "serde")]
pub use json::DtoDataJson;

/// Support code for the exported macros. Not public API.
#[doc(hidden)]
pub mod __private {
    pub use ::pastey;
    pub use ::prost;
    pub use ::prost_types;

    #[cfg(feature = "serde")]
    pub use crate::json::{decode_json, encode_json};

    use crate::AttributeError;

    /// Packs `message` into an `Any`.
    ///
    /// `Any` owns its fields, so this is two allocations: `type_url` copied
    /// from the `'static` literal and `value` from `encode_to_vec`.
    #[inline]
    #[must_use]
    pub fn pack<M: ::prost::Message>(message: &M, type_url: &'static str) -> ::prost_types::Any {
        ::prost_types::Any {
            type_url: type_url.to_owned(),
            value: message.encode_to_vec(),
        }
    }

    /// Checks the discriminant, then decodes `attributes` as `M`.
    ///
    /// # Errors
    ///
    /// [`AttributeError::TypeMismatch`] when `actual != expected`,
    /// [`AttributeError::Missing`] when there is no `Any`, and
    /// [`AttributeError::Decode`] when the bytes are not an `M`.
    #[inline]
    pub fn unpack<M: ::prost::Message + Default>(
        actual: i32,
        expected: i32,
        expected_name: &'static str,
        attributes: Option<&::prost_types::Any>,
    ) -> Result<M, AttributeError> {
        if actual != expected {
            return Err(AttributeError::TypeMismatch {
                expected: expected_name,
                actual,
            });
        }
        let any = attributes.ok_or(AttributeError::Missing)?;
        M::decode(any.value.as_slice()).map_err(AttributeError::Decode)
    }
}
