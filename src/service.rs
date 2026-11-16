// SPDX-License-Identifier: AGPL-3.0-or-later

//! `frb_crud_service!`: one `flutter_rust_bridge` opaque wrapper per CRUD app.

/// Generates a `flutter_rust_bridge` opaque CRUD wrapper around an app type.
///
/// ```ignore
/// proto_dto::frb_crud_service! {
///     wrapper: FolderServiceWrapper,
///     app: rf_cgp::FolderApp,
///     error: crate::api::database::DbError,
///     label: "folder",
/// }
/// ```
///
/// expands to
///
/// ```ignore
/// #[flutter_rust_bridge::frb(opaque)]
/// pub struct FolderServiceWrapper { app: rf_cgp::FolderApp }
///
/// impl FolderServiceWrapper {
///     pub async fn new() -> Result<Self, DbError>;
///     pub async fn create_by_dto(&self, dto: DtoData) -> Result<DtoData, DbError>;
///     pub async fn update_dto_by_id(&self, dto: DtoData) -> Result<Option<DtoData>, DbError>;
///     pub async fn get_paginate_dto(&self, page: Option<u64>, size: Option<u64>)
///         -> Result<Vec<DtoData>, DbError>;
/// }
/// ```
///
/// Everything the expansion calls is a token resolved at the call site, so
/// `proto-dto` depends on none of these crates:
///
/// | Token | Expected at the call site |
/// |---|---|
/// | `flutter_rust_bridge::frb` | the bridge attribute |
/// | `RF::load()` | `Result<Config, E>` with `error: From<E>` |
/// | `rf_entity::init(&config).await` | `Result<Connection, E>` with `error: From<E>` |
/// | `<app>::new(connection)` | `app` |
/// | `rf_cgp::prelude::{CanCreateByDto, CanUpdateDtoById, CanListDto}` | CRUD traits implemented by `app`, whose error is `Display` and converts into `error` |
/// | `log::error!` | the `log` facade |
/// | `dto` (optional key) | the envelope type; default `crate::api::shared::dto::DtoData`, the bridge mirror |
///
/// `label` prefixes the log line written when a call fails, e.g.
/// `folder create_by_dto failed: ...`.
// `crate::` below deliberately names the *caller's* crate (the Flutter bridge
// crate), where the mirrored `DtoData` lives.
#[allow(clippy::crate_in_macro_def)]
#[macro_export]
macro_rules! frb_crud_service {
    (
        wrapper: $wrapper:ident,
        app: $app:path,
        error: $error:ty,
        label: $label:literal,
        dto: $dto:ty $(,)?
    ) => {
        #[doc = concat!(
            " Flutter-facing CRUD service for `", $label, "` entities, backed by [`",
            stringify!($app), "`]."
        )]
        #[::flutter_rust_bridge::frb(opaque)]
        pub struct $wrapper {
            app: $app,
        }

        impl $wrapper {
            #[doc = concat!(
                " Loads `RF`, opens the database with `rf_entity::init` and builds the `",
                $label, "` app."
            )]
            #[doc = ""]
            #[doc = " # Errors"]
            #[doc = ""]
            #[doc = " When the configuration cannot be loaded or the database cannot be opened."]
            pub async fn new() -> ::core::result::Result<Self, $error> {
                let config = RF::load().map_err(|error| {
                    ::log::error!("{} new: RF::load failed: {}", $label, error);
                    <$error as ::core::convert::From<_>>::from(error)
                })?;
                let connection = rf_entity::init(&config).await.map_err(|error| {
                    ::log::error!("{} new: rf_entity::init failed: {}", $label, error);
                    <$error as ::core::convert::From<_>>::from(error)
                })?;
                ::core::result::Result::Ok(Self {
                    app: <$app>::new(connection),
                })
            }

            #[doc = concat!(" Creates a `", $label, "` from `dto`, or returns the existing match.")]
            #[doc = ""]
            #[doc = " # Errors"]
            #[doc = ""]
            #[doc = " When the DTO is invalid or the database write fails."]
            pub async fn create_by_dto(&self, dto: $dto) -> ::core::result::Result<$dto, $error> {
                #[allow(unused_imports)]
                use rf_cgp::prelude::CanCreateByDto as _;
                match self.app.create_by_dto(dto).await {
                    ::core::result::Result::Ok((created, _kind)) => ::core::result::Result::Ok(created),
                    ::core::result::Result::Err(error) => {
                        ::log::error!("{} create_by_dto failed: {}", $label, error);
                        ::core::result::Result::Err(<$error as ::core::convert::From<_>>::from(error))
                    }
                }
            }

            #[doc = concat!(" Updates the `", $label, "` whose id is `dto.id`; `None` when it does not exist.")]
            #[doc = ""]
            #[doc = " # Errors"]
            #[doc = ""]
            #[doc = " When the DTO is invalid or the database write fails."]
            pub async fn update_dto_by_id(
                &self,
                dto: $dto,
            ) -> ::core::result::Result<::core::option::Option<$dto>, $error> {
                #[allow(unused_imports)]
                use rf_cgp::prelude::CanUpdateDtoById as _;
                self.app.update_dto_by_id(dto).await.map_err(|error| {
                    ::log::error!("{} update_dto_by_id failed: {}", $label, error);
                    <$error as ::core::convert::From<_>>::from(error)
                })
            }

            #[doc = concat!(" Lists one page of `", $label, "` DTOs.")]
            #[doc = ""]
            #[doc = " # Errors"]
            #[doc = ""]
            #[doc = " When the database read fails."]
            pub async fn get_paginate_dto(
                &self,
                page: ::core::option::Option<u64>,
                size: ::core::option::Option<u64>,
            ) -> ::core::result::Result<::std::vec::Vec<$dto>, $error> {
                #[allow(unused_imports)]
                use rf_cgp::prelude::CanListDto as _;
                self.app.list_dto(page, size).await.map_err(|error| {
                    ::log::error!("{} get_paginate_dto failed: {}", $label, error);
                    <$error as ::core::convert::From<_>>::from(error)
                })
            }
        }
    };
    (
        wrapper: $wrapper:ident,
        app: $app:path,
        error: $error:ty,
        label: $label:literal $(,)?
    ) => {
        $crate::frb_crud_service! {
            wrapper: $wrapper,
            app: $app,
            error: $error,
            label: $label,
            dto: crate::api::shared::dto::DtoData,
        }
    };
}
