# proto-dto

[![CI](https://github.com/Just-Replicant/proto-dto/actions/workflows/ci.yml/badge.svg)](https://github.com/Just-Replicant/proto-dto/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/proto-dto.svg)](https://crates.io/crates/proto-dto)
[![docs.rs](https://docs.rs/proto-dto/badge.svg)](https://docs.rs/proto-dto)
![MSRV 1.85](https://img.shields.io/badge/MSRV-1.85-blue)
![License: AGPL-3.0-or-later](https://img.shields.io/badge/license-AGPL--3.0--or--later-blue)

`macro_rules!` generators for protobuf DTOs carried in a
`google.protobuf.Any` envelope, and for the `flutter_rust_bridge` CRUD
wrappers that hand them to Dart.

```proto
message DTOData {
  enum DataType { UNSPECIFIED = 0; FILE = 1; FOLDER = 2; DEVICE = 3; HARD_DRIVE = 4; }
  optional int32 id = 1;
  DataType type = 2;
  google.protobuf.Any attributes = 3;
}
```

* `envelope!` turns one table of `(message, discriminant, type URL, label)`
  rows into `from_*` / `try_as_*` methods and a JSON shape where
  `attributes` is the entity object instead of a raw `Any`.
* `frb_crud_service!` emits a `#[frb(opaque)]` wrapper with `new`,
  `create_by_dto`, `update_dto_by_id` and `get_paginate_dto`.
* `AttributeError` and (feature `serde`) `DtoDataJson`.

Both generators are `macro_rules!`: a proc macro cannot see the prost structs a
build script writes into `OUT_DIR`, while a `macro_rules!` invocation placed
after the `include!` can. `proto-dto` depends on neither the crate holding the
protos nor `flutter_rust_bridge`; every path the macros call is a token
resolved at the call site.

| | |
|---|---|
| Edition / MSRV | 2024 / Rust 1.85 |
| Features | `serde`: `DtoDataJson` and the JSON half of `envelope!` |
| Dependencies | `prost`, `prost-types`, `pastey`; `serde` + `serde_json` with `serde` |

## `envelope!`

```rust,ignore
proto_dto::envelope! {
    dto: DtoData,
    id: id,
    kind: r#type,
    attributes: attributes,
    data_type: DataType,
    DeviceDto, Device, "type.googleapis.com/rf_grpc.DeviceDTO", "DeviceDTO";
    FolderDto, Folder, "type.googleapis.com/rf_grpc.FolderDTO", "FolderDTO";
    FileDto, File, "type.googleapis.com/rf_grpc.FileDTO", "FileDTO";
    HardDriveDto, HardDrive, "type.googleapis.com/rf_grpc.HardDriveDTO", "HardDriveDTO";
}
```

For every row:

```rust,ignore
impl DtoData {
    #[inline] pub fn from_folder(id: Option<i32>, attribute: FolderDto) -> Self;
    #[inline] pub fn try_as_folder(&self) -> Result<FolderDto, proto_dto::AttributeError>;
}
impl FolderDto {
    pub const TYPE_URL: &'static str = "type.googleapis.com/rf_grpc.FolderDTO";
}
```

The method suffix is the `snake_case` of the variant (`HardDrive` →
`from_hard_drive`). With the `serde` feature it also emits
`pub type DtoDataJson = proto_dto::DtoDataJson;`,
`From<DtoData> for DtoDataJson` and `TryFrom<DtoDataJson> for DtoData`:

```json
{ "id": 7, "type": 3, "attributes": { "name": "desk", "is_current": true } }
```

### Runtime

* Encode and decode are one `match` on the prost enumeration, one arm per row.
  With contiguous discriminants (`FILE = 1 … HARD_DRIVE = 4`) that is a jump
  table: no registry, `Vec`, `OnceLock` or function-pointer call.
* `prost_types::Any` owns its fields, so each pack makes two allocations:
  `type_url` copied from a `&'static str` and `value` from `encode_to_vec()`.
* Only the JSON path builds a `serde_json::Value`; the protobuf/gRPC path
  never does.
* `AttributeError` (`Missing`, `TypeMismatch`, `UnsupportedType`, `Decode`,
  `Json`) is a small stack enum, `#[non_exhaustive]`. A JSON failure keeps the
  `serde_json::Error` and formats it in `Display`, so returning an error never
  builds a `String`.
* Serializing an envelope whose discriminant has no row, or whose bytes do not
  decode, writes `"attributes": null` (`From` cannot fail). Deserializing
  attributes for an unknown discriminant is an error.

## `frb_crud_service!`

```rust,ignore
proto_dto::frb_crud_service! {
    wrapper: FolderServiceWrapper,
    app: rf_cgp::FolderApp,
    error: crate::api::database::DbError,
    label: "folder",
}
```

expands to `#[flutter_rust_bridge::frb(opaque)] pub struct
FolderServiceWrapper { app: rf_cgp::FolderApp }` and

| Method | Calls |
|---|---|
| `async fn new() -> Result<Self, DbError>` | `RF::load()`, `rf_entity::init(&config).await`, `FolderApp::new(connection)` |
| `async fn create_by_dto(&self, dto) -> Result<DtoData, DbError>` | `CanCreateByDto::create_by_dto` (the `CreateKind` is dropped) |
| `async fn update_dto_by_id(&self, dto) -> Result<Option<DtoData>, DbError>` | `CanUpdateDtoById::update_dto_by_id` |
| `async fn get_paginate_dto(&self, page, size) -> Result<Vec<DtoData>, DbError>` | `CanListDto::list_dto` |

The call site must provide: the `flutter_rust_bridge` and `log` crates; `RF`
in scope with `RF::load() -> Result<Config, E>`; `rf_entity::init(&Config)`
returning a future of `Result<Connection, E>`; `rf_cgp::prelude::{CanCreateByDto,
CanUpdateDtoById, CanListDto}` implemented by `app`; `error: From<E>` for each
error (errors must be `Display`, they are logged with `label`); and
`crate::api::shared::dto::DtoData` (the bridge mirror). Pass a fifth key
`dto: some::Path` to use another envelope type.

`flutter_rust_bridge_codegen generate` expands `macro_rules!` itself through
`cargo expand`. Run it by hand or in CI, never from a `build.rs`.

## Using it from `rf-proto` (guidance)

`rf-proto` keeps `proto/**`, `build.rs`, prost/tonic codegen, the generated
types and the `grpc` feature. It gains one dependency and one invocation.

**`rust/rf-proto/Cargo.toml`**

```toml
[dependencies]
proto-dto = { path = "../../proto-dto", features = ["serde"] }
# or, once published:
# proto-dto = { version = "0.1", features = ["serde"] }
```

**`build.rs`**: keep the serde derive and point `DTOData` at the JSON form
(the alias is emitted by `envelope!` at the crate root):

```rust,ignore
config.type_attribute(".", "#[derive(serde::Serialize, serde::Deserialize)]");
config.type_attribute(
    "rf_grpc.DTOData",
    "#[serde(try_from = \"crate::DtoDataJson\", into = \"crate::DtoDataJson\")]",
);
```

**`src/lib.rs`**: delete `dto_attribute.rs` (`DtoAttribute`,
`AttributeCodec`, the `OnceLock` registry) and `serde_dto_data.rs`, then:

```rust,ignore
pub mod rf_grpc {
    include!(concat!(env!("OUT_DIR"), "/rf_grpc.rs"));
}
pub use proto_dto::AttributeError;
pub use rf_grpc::{DeviceDto, DtoData, FileDto, FolderDto, HardDriveDto, dto_data::DataType /* … */};

proto_dto::envelope! { /* the invocation above */ }
```

Call sites change from `DtoData::from_typed::<FolderDto>(id, dto)` /
`dto.try_as_typed::<DeviceDto>()` to `DtoData::from_folder(id, dto)` /
`dto.try_as_device()`. In `rf-cgp`, each `encode_output` becomes
`Ok(DtoData::from_folder(Some(model.id), dto))` and the local
`*_DTO_TYPE_URL` constants go away; field mapping stays handwritten.

**FRB mirrors.** Instead of handwriting `rust/src/api/file_explorer/dto.rs`
(every `FolderDto` field again), let `rf-proto`'s `build.rs` render the
mirrors from the descriptor set it already compiles:

1. `tonic_prost_build::configure().file_descriptor_set_path(out_dir.join("rf_grpc.bin"))`.
2. Decode it with `prost_types::FileDescriptorSet` (add `prost` and
   `prost-types` to `[build-dependencies]`).
3. For each mirrored message, emit
   `pub use rf_proto::FolderDto; #[frb(mirror(FolderDto), dart_metadata = ("freezed"))] pub struct _FolderDto { … }`
   with prost's field types (`optional`/message → `Option<T>`, `repeated` →
   `Vec<T>`, enum → `i32`, keywords as `r#type`). Write
   `rust/src/api/file_explorer/dto.rs` and `rust/src/api/shared/dto.rs`
   (`Any`, `DtoData`) only when the content changes, and mark them
   `@generated`.
4. Also emit exhaustive `let FolderDto { a, b, … } = x;` conversions into
   `OUT_DIR` and `include!` them in a `#[cfg(test)]` module, so a proto change
   without regenerated mirrors fails to compile.

## Licensing

AGPL-3.0-or-later ([`LICENSE`](LICENSE)). A distributed binary that links a
crate expanding these macros (e.g. `rf-proto`, or the Flutter crate expanding
`frb_crud_service!`) is a combined work under AGPL-3.0-or-later.

## Development

```sh
cargo fmt --all --check
cargo clippy --all-targets -- -D warnings
cargo clippy --all-targets --all-features -- -D warnings
cargo test                    # protobuf path only
cargo test --features serde   # + JSON path
cargo +1.85 test --all-features
RUSTDOCFLAGS="--cfg docsrs -D warnings" cargo +nightly doc --no-deps --all-features
```

`tests/envelope.rs` mirrors the reference `rf_grpc` schema with hand-written
prost messages. `tests/frb_crud_service.rs` drives the service macro against
stand-in `RF`/`rf_entity`/`rf_cgp` modules and a no-op `frb` attribute from
`test-support/frb-shim` (dev-dependency only, not packaged).

## Releasing

Releases are automated with [release-plz](https://release-plz.dev)
(`.github/workflows/release-plz.yml`, `release-plz.toml`): every push to
`main` opens or updates a release PR (version bump + `CHANGELOG.md`); merging
it publishes to crates.io, tags `vX.Y.Z` and creates the GitHub release.

Publishing uses **crates.io trusted publishing** (OIDC); there is no
`CARGO_REGISTRY_TOKEN` secret. One-time setup:

1. **First publish is manual.** Trusted publishing only works for a crate that
   already exists on crates.io. From a clean checkout of `main`:
   `cargo login` (token with `publish-new`), then `cargo publish`. Revoke the
   token afterwards.
2. On crates.io → *proto-dto* → *Settings* → *Trusted Publishing* → *Add*:
   GitHub, owner `Just-Replicant`, repository `proto-dto`, workflow filename
   `release-plz.yml`, environment empty.
3. Optionally enable *Require trusted publishing for all new versions* in the
   same page.
4. In the GitHub repository → *Settings* → *Actions* → *General*, allow
   GitHub Actions to create and approve pull requests (the release PR).

The release job has `id-token: write`; release-plz exchanges the GitHub OIDC
token for a short-lived crates.io token itself, so no
`rust-lang/crates-io-auth-action` step is needed.
