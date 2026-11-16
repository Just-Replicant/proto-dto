// SPDX-License-Identifier: AGPL-3.0-or-later

//! `envelope!`: pack, unpack and JSON for a typed message inside an `Any`.

/// Generates typed accessors and JSON conversions for an `Any` envelope.
///
/// Invoke once, in the crate that owns the prost types, after the module that
/// includes the generated code:
///
/// ```ignore
/// proto_dto::envelope! {
///     dto: DtoData,
///     id: id,
///     kind: r#type,
///     attributes: attributes,
///     data_type: DataType,
///     DeviceDto, Device, "type.googleapis.com/rf_grpc.DeviceDTO", "DeviceDTO";
///     FolderDto, Folder, "type.googleapis.com/rf_grpc.FolderDTO", "FolderDTO";
/// }
/// ```
///
/// * `dto`: the envelope struct. It must have exactly the three fields named
///   by `id` (`Option<i32>`), `kind` (`i32`) and `attributes`
///   (`Option<prost_types::Any>`).
/// * `data_type`: the prost enumeration whose variants are the
///   discriminants. It must be in scope at the call site.
/// * Each row is `Message, Variant, "type URL", "protobuf message name"`.
///
/// For every row this emits, on the envelope:
///
/// * `from_<variant>(id, message) -> Self`
/// * `try_as_<variant>(&self) -> Result<Message, AttributeError>`
///
/// where `<variant>` is the `snake_case` form of `Variant`
/// (`HardDrive` → `from_hard_drive`), and on each message a
/// `TYPE_URL: &'static str` constant.
///
/// With the `serde` feature of `proto-dto` it also emits
/// `From<Envelope> for DtoDataJson`, `TryFrom<DtoDataJson> for Envelope` and a
/// `DtoDataJson` alias at the call site. Both directions are one `match` on
/// the discriminant with one arm per row. Point the prost type at the JSON
/// form from `build.rs`:
///
/// ```ignore
/// config.type_attribute(
///     "pkg.DTOData",
///     "#[serde(try_from = \"crate::DtoDataJson\", into = \"crate::DtoDataJson\")]",
/// );
/// ```
#[macro_export]
macro_rules! envelope {
    (
        dto: $dto:ident,
        id: $id:ident,
        kind: $kind:ident,
        attributes: $attributes:ident,
        data_type: $data_type:ident,
        $( $message:ident, $variant:ident, $type_url:literal, $label:literal );+ $(;)?
    ) => {
        $crate::__private::pastey::paste! {
            impl $dto {
                $(
                    #[doc = concat!(
                        " Packs a [`", stringify!($message), "`] into `", stringify!($attributes),
                        "` with discriminant `", stringify!($data_type), "::", stringify!($variant), "`."
                    )]
                    #[inline]
                    #[must_use]
                    pub fn [<from_ $variant:snake>](id: ::core::option::Option<i32>, attribute: $message) -> Self {
                        Self {
                            $id: id,
                            $kind: $data_type::$variant as i32,
                            $attributes: ::core::option::Option::Some(
                                $crate::__private::pack(&attribute, $type_url),
                            ),
                        }
                    }

                    #[doc = concat!(
                        " Decodes `", stringify!($attributes), "` as a [`", stringify!($message),
                        "`] after checking the discriminant is `", stringify!($data_type), "::",
                        stringify!($variant), "`."
                    )]
                    #[doc = ""]
                    #[doc = " # Errors"]
                    #[doc = ""]
                    #[doc = " `TypeMismatch` for another discriminant, `Missing` without attributes, `Decode` for bad bytes."]
                    #[inline]
                    pub fn [<try_as_ $variant:snake>](
                        &self,
                    ) -> ::core::result::Result<$message, $crate::AttributeError> {
                        $crate::__private::unpack::<$message>(
                            self.$kind,
                            $data_type::$variant as i32,
                            $data_type::$variant.as_str_name(),
                            self.$attributes.as_ref(),
                        )
                    }
                )+
            }
        }

        $(
            impl $message {
                #[doc = concat!(" `google.protobuf.Any` type URL of `", $label, "`.")]
                pub const TYPE_URL: &'static str = $type_url;
            }
        )+

        $crate::__envelope_json! {
            dto: $dto,
            id: $id,
            kind: $kind,
            attributes: $attributes,
            data_type: $data_type,
            $( $message, $variant, $type_url, $label );+
        }
    };
}

/// JSON half of [`envelope!`], present only with the `serde` feature.
///
/// `#[cfg(feature = "serde")]` inside a `macro_rules!` body would be evaluated
/// against the caller's features, so the switch lives here instead.
#[cfg(feature = "serde")]
#[doc(hidden)]
#[macro_export]
macro_rules! __envelope_json {
    (
        dto: $dto:ident,
        id: $id:ident,
        kind: $kind:ident,
        attributes: $attributes:ident,
        data_type: $data_type:ident,
        $( $message:ident, $variant:ident, $type_url:literal, $label:literal );+
    ) => {
        /// JSON form of the envelope; see [`proto_dto::DtoDataJson`].
        pub type DtoDataJson = $crate::DtoDataJson;

        impl ::core::convert::From<$dto> for $crate::DtoDataJson {
            /// Renders `attributes` as the entity object. Attributes whose
            /// discriminant has no row, or whose bytes do not decode, become
            /// `None`.
            fn from(dto: $dto) -> Self {
                let attributes = match dto.$attributes.as_ref() {
                    ::core::option::Option::None => ::core::option::Option::None,
                    ::core::option::Option::Some(any) => {
                        match <$data_type as ::core::convert::TryFrom<i32>>::try_from(dto.$kind) {
                            $(
                                ::core::result::Result::Ok($data_type::$variant) => {
                                    $crate::__private::encode_json::<$message>(any.value.as_slice())
                                }
                            )+
                            #[allow(unreachable_patterns)]
                            _ => ::core::option::Option::None,
                        }
                    }
                };
                Self {
                    id: dto.$id,
                    r#type: dto.$kind,
                    attributes,
                }
            }
        }

        impl ::core::convert::TryFrom<$crate::DtoDataJson> for $dto {
            type Error = $crate::AttributeError;

            fn try_from(json: $crate::DtoDataJson) -> ::core::result::Result<Self, Self::Error> {
                let attributes = match json.attributes {
                    ::core::option::Option::None => ::core::option::Option::None,
                    ::core::option::Option::Some(value) => ::core::option::Option::Some(
                        match <$data_type as ::core::convert::TryFrom<i32>>::try_from(json.r#type) {
                            $(
                                ::core::result::Result::Ok($data_type::$variant) => {
                                    $crate::__private::decode_json::<$message>(value, $type_url, $label)?
                                }
                            )+
                            #[allow(unreachable_patterns)]
                            _ => {
                                return ::core::result::Result::Err(
                                    $crate::AttributeError::UnsupportedType(json.r#type),
                                );
                            }
                        },
                    ),
                };
                ::core::result::Result::Ok(Self {
                    $id: json.id,
                    $kind: json.r#type,
                    $attributes: attributes,
                })
            }
        }
    };
}

#[cfg(not(feature = "serde"))]
#[doc(hidden)]
#[macro_export]
macro_rules! __envelope_json {
    ($($tokens:tt)*) => {};
}
