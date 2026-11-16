# proto-dto-build

Build-script helper for [`proto-dto`](https://github.com/Just-Replicant/proto-dto): renders
`flutter_rust_bridge` mirror structs, and an exhaustive conversion check,
from the protobuf descriptor set that `prost-build` / `tonic-prost-build`
already write.

`flutter_rust_bridge` only generates Dart classes for types under the Flutter
crate's `crate::api`. A prost type from another crate needs a
`#[frb(mirror(T))] pub struct _T { .. }` that repeats every field. Written by
hand, those drift from the `.proto` files; this crate renders them instead.

It is a separate crate so that only build scripts compile `heck` and the file
writer: add it under `[build-dependencies]`, never `[dependencies]`.

| | |
|---|---|
| Edition / MSRV | 2024 / Rust 1.85 |
| Dependencies | `heck`, `prost`, `prost-types` |

## What it does

* **Descriptor walk.** Every field gets the type prost generates for it:

  | `.proto` | Rust |
  |---|---|
  | `double`, `float` | `f64`, `f32` |
  | `int64`, `sint64`, `sfixed64` / `uint64`, `fixed64` | `i64` / `u64` |
  | `int32`, `sint32`, `sfixed32`, enums / `uint32`, `fixed32` | `i32` / `u32` |
  | `bool`, `string`, `bytes` | `bool`, `String`, `Vec<u8>` |
  | message field, proto3 `optional`, proto2 `optional` | `Option<T>` |
  | `repeated T` | `Vec<T>` |
  | `map<K, V>` | `::std::collections::HashMap<K, V>` |
  | singular field whose type contains its owner | `Option<Box<T>>` |

  A message field must point at another mirrored message; otherwise the
  build fails with the field and type to add.
* **Names** use `heck` exactly like prost: `FolderDTO` → `FolderDto`,
  `createdAt` → `created_at`, nested `Outer.Inner` → `outer::Inner`.
  Keywords are escaped like prost: `type` → `r#type`, `self` → `self_`.
* **Write-if-unchanged.** Files are rewritten only when their content
  differs, so their mtime stays put and file watchers, the bridge codegen
  and the IDE are not retriggered on every build.
* **Exhaustiveness check.** An optional file in `OUT_DIR` with, for every
  mirrored message, a private copy of the mirror and `From` conversions both
  ways written as `let FolderDto { parent_id, name, .. } = source;` without
  `..`. `include!` it in a `#[cfg(test)]` module of the crate that owns the
  prost types: adding, removing or retyping a field without regenerating the
  mirrors is a compile error.
* `.proto` leading comments become rustdoc on the mirrors (except for
  `google.protobuf` well-known types, which keep a one-line summary).

## `rf-proto/build.rs`

Everything specific to the app is an argument: the descriptor path, the
protobuf package, the message names, the `rf_proto` import and the output
files.

```toml
# rust/rf-proto/Cargo.toml
[build-dependencies]
proto-dto-build = { path = "../../proto-dto/proto-dto-build" }
# or, once published: proto-dto-build = "0.1"
protoc-bin-vendored = "3"
tonic-prost-build = "0.14"
```

```rust,ignore
// rust/rf-proto/build.rs
use std::error::Error;
use std::path::{Path, PathBuf};

use proto_dto_build::{Message, Mirror};

const PROTOS: &[&str] = &[
    "proto/device/device.proto",
    "proto/device/hard_drive.proto",
    "proto/explorer/file.proto",
    "proto/explorer/folder.proto",
];
const INCLUDES: &[&str] = &["proto", "proto/device", "proto/explorer"];
/// `src/api` of the Flutter crate, relative to rf-proto.
const API_DIR: &str = "../src/api";

fn main() -> Result<(), Box<dyn Error>> {
    println!("cargo:rerun-if-changed=proto");
    println!("cargo:rerun-if-env-changed=PROTOC");

    let out_dir = PathBuf::from(std::env::var_os("OUT_DIR").ok_or("OUT_DIR is not set")?);
    let descriptor_path = out_dir.join("rf_grpc_descriptor.bin");

    let mut config = tonic_prost_build::Config::new();
    config.protoc_executable(protoc_bin_vendored::protoc_bin_path()?);
    config.type_attribute(".", "#[derive(serde::Serialize, serde::Deserialize)]");
    config.type_attribute(
        "rf_grpc.DTOData",
        "#[serde(try_from = \"crate::DtoDataJson\", into = \"crate::DtoDataJson\")]",
    );
    tonic_prost_build::configure()
        .build_client(false)
        .build_server(std::env::var_os("CARGO_FEATURE_GRPC").is_some())
        .file_descriptor_set_path(&descriptor_path)
        .compile_with_config(config, PROTOS, INCLUDES)?;

    // When rf-proto is built on its own there is no Flutter crate next to
    // it: still render (and check) the mirrors, into OUT_DIR only.
    let api_dir = if Path::new(API_DIR).is_dir() {
        PathBuf::from(API_DIR)
    } else {
        println!("cargo:warning=rf-proto: {API_DIR} not found; FRB mirrors only written to OUT_DIR");
        out_dir.join("frb_mirror")
    };

    Mirror::new(&descriptor_path)
        .package("rf_grpc")
        .import("rf_proto")
        .generated_by("rust/rf-proto/build.rs")
        .file(
            api_dir.join("shared/dto.rs"),
            [Message::new(".google.protobuf.Any"), Message::new("DTOData")],
        )
        .file(
            api_dir.join("file_explorer/dto.rs"),
            [Message::new("FolderDTO").freezed()],
        )
        .check_file(out_dir.join("frb_mirror_check.rs"))
        .generate()?;
    Ok(())
}
```

```rust,ignore
// rust/rf-proto/src/lib.rs
pub use prost_types::Any; // mirrored types must all be reachable at `rf_proto::`
pub use rf_grpc::{DtoData, FolderDto /* … */};

#[cfg(test)]
mod mirror_check {
    include!(concat!(env!("OUT_DIR"), "/frb_mirror_check.rs"));
}
```

`../src/api/file_explorer/dto.rs` then reads:

```rust,ignore
// @generated by rust/rf-proto/build.rs from the protobuf descriptor set.
// Do not edit: change the .proto file and rebuild.
// …
use flutter_rust_bridge::frb;

pub use rf_proto::FolderDto;

/// Mirror of [`FolderDto`] (`rf_grpc.FolderDTO`).
///
/// Source: `explorer/folder.proto`.
#[frb(mirror(FolderDto), dart_metadata = ("freezed"))]
pub struct _FolderDto {
    pub parent_id: Option<i32>,
    pub name: String,
    pub path: String,
    pub created_at: String,
    pub updated_at: String,
}
```

Run `flutter_rust_bridge_codegen generate` by hand or in CI after a build has
refreshed the mirrors, never from `build.rs`: it expands the bridge crate with
`cargo expand`, which would re-enter the build.

## API

| | |
|---|---|
| `Mirror::new(path)` / `Mirror::from_bytes(bytes)` | descriptor set to read |
| `.package("rf_grpc")` | package of relative message names; `.google.protobuf.Any` style names are absolute |
| `.import("rf_proto")` | path the mirror files `pub use` the prost types from (required) |
| `.file(path, messages)` | one mirror file; `Message::new("FolderDTO").freezed()` adds `dart_metadata = ("freezed")` |
| `.check_file(path)` | also write the exhaustive conversions |
| `.check_import("crate")` | path the check file names the prost types by (default `crate`) |
| `.generated_by("…")` | generator named in the `@generated` header |
| `.render()` | `Vec<RenderedFile>`, writes nothing |
| `.generate()` | renders, writes changed files, returns `Generated { files, written }` |
| `write_if_changed(path, contents)` | the writer on its own |
| `naming::{field_name, type_name, sanitize_identifier}` | prost's identifier rules |

Errors are a `proto_dto_build::Error` naming the message and field involved.

## Limitations

Real `oneof`s, `group`s and edition-based `.proto` files are rejected. The
mirror assumes prost's defaults: `bytes` as `Vec<u8>`, maps as `HashMap`, no
`Config::boxed` overrides.

Preview the output of a descriptor set without a build:

```sh
cargo run -p proto-dto-build --example render -- rf_grpc.bin rf_grpc rf_proto FolderDTO
```

## License

AGPL-3.0-or-later, like `proto-dto` ([`LICENSE`](https://github.com/Just-Replicant/proto-dto/blob/main/LICENSE)).
