// SPDX-License-Identifier: AGPL-3.0-or-later

//! JSON shape of an envelope: `attributes` is the entity object, not a raw
//! `Any` with `type_url` and base64 bytes.

use prost::Message;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::AttributeError;

/// JSON form of an envelope.
///
/// ```json
/// { "id": 7, "type": 3, "attributes": { "name": "desk", "is_current": true } }
/// ```
///
/// [`envelope!`](crate::envelope) implements `From<Envelope> for DtoDataJson`
/// and `TryFrom<DtoDataJson> for Envelope`, so a prost type can opt in with
/// `#[serde(try_from = "DtoDataJson", into = "DtoDataJson")]`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DtoDataJson {
    /// Optional database id of the entity.
    #[serde(default)]
    pub id: Option<i32>,
    /// Raw protobuf discriminant.
    pub r#type: i32,
    /// The payload as a JSON object, or `None` when absent.
    #[serde(default)]
    pub attributes: Option<Value>,
}

/// Decodes protobuf bytes as `M` and renders them as JSON.
///
/// Returns `None` when the bytes do not decode or the message does not
/// serialize; the `From` conversion that calls this cannot fail.
#[inline]
#[must_use]
pub fn encode_json<M>(bytes: &[u8]) -> Option<Value>
where
    M: Message + Default + Serialize,
{
    M::decode(bytes)
        .ok()
        .and_then(|message| serde_json::to_value(message).ok())
}

/// Deserializes a JSON object as `M` and packs it into an `Any`.
///
/// # Errors
///
/// [`AttributeError::Json`] with `label` when `value` is not an `M`.
#[inline]
pub fn decode_json<M>(
    value: Value,
    type_url: &'static str,
    label: &'static str,
) -> Result<prost_types::Any, AttributeError>
where
    M: Message + DeserializeOwned,
{
    let message: M =
        serde_json::from_value(value).map_err(|source| AttributeError::Json { label, source })?;
    Ok(crate::__private::pack(&message, type_url))
}
