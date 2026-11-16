// SPDX-License-Identifier: AGPL-3.0-or-later

//! Golden-file and behaviour tests for `Mirror`.
//!
//! The fixtures under `tests/fixtures` are the exact rendered output; they
//! are also compiled by `tests/compile.rs`. After an intended change to the
//! output, regenerate them with
//! `PROTO_DTO_BUILD_BLESS=1 cargo test -p proto-dto-build --test generate`.

mod common;

use std::path::{Path, PathBuf};

use proto_dto_build::{Error, Message, Mirror};

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

/// `.file(...)` paths are fixture names relative to `tests/fixtures`.
fn assert_matches_fixtures(mirror: &Mirror) {
    let bless = std::env::var_os("PROTO_DTO_BUILD_BLESS").is_some();
    for file in mirror.render().expect("render") {
        let fixture = fixtures().join(&file.path);
        if bless {
            proto_dto_build::write_if_changed(&fixture, &file.contents).expect("bless");
            continue;
        }
        let expected = std::fs::read_to_string(&fixture)
            .unwrap_or_else(|error| panic!("{}: {error}", fixture.display()));
        assert_eq!(
            file.contents,
            expected,
            "{} is stale; rerun with PROTO_DTO_BUILD_BLESS=1",
            fixture.display()
        );
    }
}

fn rf_grpc_mirror() -> Mirror {
    Mirror::from_bytes(common::encode(&common::rf_grpc()))
        .package("rf_grpc")
        .import("rf_proto::")
        .generated_by("rust/rf-proto/build.rs")
        .file(
            "rf_grpc/shared_dto.rs",
            [
                Message::new(".google.protobuf.Any"),
                Message::new("DTOData"),
            ],
        )
        .file(
            "rf_grpc/file_explorer_dto.rs",
            [Message::new("FolderDTO").freezed()],
        )
        .check_file("rf_grpc/check.rs")
}

fn kinds_mirror() -> Mirror {
    Mirror::from_bytes(common::encode(&common::kitchen_sink()))
        .package("kinds")
        .import("crate::kinds")
        .check_import("crate::kinds")
        .file("kinds/sink.rs", ["KitchenSink", "Empty"])
        .file("kinds/child.rs", ["KitchenSink.Child", "Legacy"])
        .check_file("kinds/check.rs")
}

#[test]
fn rf_grpc_matches_fixtures() {
    assert_matches_fixtures(&rf_grpc_mirror());
}

#[test]
fn kitchen_sink_matches_fixtures() {
    assert_matches_fixtures(&kinds_mirror());
}

#[test]
fn folder_mirror_has_prost_types_and_docs() {
    let files = rf_grpc_mirror().render().unwrap();
    let folder = &files[1].contents;
    assert!(folder.contains("pub use rf_proto::FolderDto;\n"));
    assert!(folder.contains("#[frb(mirror(FolderDto), dart_metadata = (\"freezed\"))]\n"));
    assert!(folder.contains("pub struct _FolderDto {\n"));
    assert!(
        folder.contains("    /// `None` for a root folder.\n    pub parent_id: Option<i32>,\n")
    );
    assert!(folder.contains("/// A folder on a device.\n///\n/// Paths are absolute.\n"));
    assert!(folder.contains("/// Source: `explorer/folder.proto`.\n"));

    let shared = &files[0].contents;
    assert!(shared.contains("pub use rf_proto::{Any, DtoData};\n"));
    assert!(shared.contains("    pub r#type: i32,\n"));
    assert!(shared.contains("    pub attributes: Option<Any>,\n"));
    // Well-known types keep only the summary line.
    assert!(!shared.contains("Long upstream documentation"));
}

#[test]
fn field_kinds_map_to_prost_types() {
    let files = kinds_mirror().render().unwrap();
    let sink = &files[0].contents;
    for line in [
        "pub double_value: f64,",
        "pub float_value: f32,",
        "pub sint64_value: i64,",
        "pub fixed64_value: u64,",
        "pub sfixed32_value: i32,",
        "pub fixed32_value: u32,",
        "pub bytes_value: Vec<u8>,",
        "pub color: i32,",
        "pub maybe: Option<String>,",
        "pub tags: Vec<String>,",
        "pub colors: Vec<i32>,",
        "pub child: Option<Child>,",
        "pub children: Vec<Child>,",
        "pub counts: ::std::collections::HashMap<String, i64>,",
        "pub by_name: ::std::collections::HashMap<String, Child>,",
        "pub r#type: String,",
        "pub r#match: bool,",
        "pub self_: i32,",
        "pub r#async: i32,",
        "pub created_at: String,",
        "pub next: Option<Box<KitchenSink>>,",
    ] {
        assert!(sink.contains(line), "missing `{line}` in:\n{sink}");
    }
    // The nested message comes from prost's module and is imported, not
    // re-exported, by the file that only uses it.
    assert!(sink.contains("use crate::kinds::kitchen_sink::Child;\n"));
    assert!(!sink.contains("pub use crate::kinds::kitchen_sink::Child;"));
    assert!(sink.contains("pub use crate::kinds::{Empty, KitchenSink};\n"));
    assert!(sink.contains("pub struct _Empty {}\n"));

    let child = &files[1].contents;
    assert!(child.contains("pub use crate::kinds::kitchen_sink::Child;\n"));
    // proto2: `optional` has presence, `required` does not.
    assert!(child.contains("pub optional_name: Option<String>,"));
    assert!(child.contains("pub required_id: i32,"));
    assert!(child.contains("pub values: Vec<i32>,"));
}

#[test]
fn check_file_destructures_every_field() {
    let files = kinds_mirror().render().unwrap();
    let check = &files[2].contents;
    assert!(check.contains(
        "        let crate::kinds::kitchen_sink::Child { id } = source;\n        Self { id }\n"
    ));
    assert!(check.contains("        let crate::kinds::Empty {} = source;\n"));
    assert!(check.contains("    next: Option<Box<crate::kinds::KitchenSink>>,\n"));
    assert!(check.contains("r#type, r#match, self_, r#async, created_at, next }"));
}

#[test]
fn generate_writes_only_changed_files() {
    let dir = common::temp_dir("write-if-changed");
    let mirror = Mirror::from_bytes(common::encode(&common::rf_grpc()))
        .package("rf_grpc")
        .import("rf_proto")
        .file(dir.join("api/file_explorer/dto.rs"), ["FolderDTO"])
        .check_file(dir.join("out/check.rs"));

    let first = mirror.generate().unwrap();
    assert_eq!(first.files.len(), 2);
    assert_eq!(first.written, first.files, "first run writes every file");
    let dto = dir.join("api/file_explorer/dto.rs");
    let rendered = std::fs::read_to_string(&dto).unwrap();
    assert!(rendered.starts_with("// @generated by build.rs"));

    let second = mirror.generate().unwrap();
    assert_eq!(second.files, first.files);
    assert!(
        second.written.is_empty(),
        "unchanged files are not rewritten"
    );

    std::fs::write(&dto, "stale").unwrap();
    let third = mirror.generate().unwrap();
    assert_eq!(third.written, vec![dto.clone()]);
    assert_eq!(std::fs::read_to_string(&dto).unwrap(), rendered);

    // The same through the public helper.
    assert!(!proto_dto_build::write_if_changed(&dto, &rendered).unwrap());
    assert!(proto_dto_build::write_if_changed(&dto, "other").unwrap());
}

#[test]
fn reads_the_descriptor_set_from_a_path() {
    let dir = common::temp_dir("from-path");
    let descriptor = dir.join("rf_grpc_descriptor.bin");
    std::fs::write(&descriptor, common::encode(&common::rf_grpc())).unwrap();
    let from_path = Mirror::new(&descriptor)
        .package("rf_grpc")
        .import("rf_proto")
        .file("dto.rs", ["FolderDTO"])
        .render()
        .unwrap();
    let from_bytes = Mirror::from_bytes(common::encode(&common::rf_grpc()))
        .package("rf_grpc")
        .import("rf_proto")
        .file("dto.rs", ["FolderDTO"])
        .render()
        .unwrap();
    assert_eq!(from_path, from_bytes);

    let missing = Mirror::new(dir.join("missing.bin"))
        .import("rf_proto")
        .file("dto.rs", [".rf_grpc.FolderDTO"])
        .render();
    assert!(matches!(missing, Err(Error::Io { .. })), "{missing:?}");
}

fn render_err(mirror: &Mirror) -> String {
    mirror.render().expect_err("should fail").to_string()
}

#[test]
fn reports_configuration_errors() {
    let base = || Mirror::from_bytes(common::encode(&common::rf_grpc())).package("rf_grpc");

    let error = render_err(&base().file("a.rs", ["FolderDTO"]));
    assert!(error.contains("`import` is required"), "{error}");

    let error = render_err(&base().import("rf_proto"));
    assert!(error.contains("no mirror file"), "{error}");

    let error = render_err(
        &base()
            .import("rf_proto")
            .file("a.rs", Vec::<Message>::new()),
    );
    assert!(error.contains("a.rs lists no messages"), "{error}");

    let error = render_err(
        &base()
            .import("rf_proto")
            .file("a.rs", ["FolderDTO"])
            .check_file("a.rs"),
    );
    assert!(error.contains("a.rs is used for two outputs"), "{error}");

    let error = render_err(
        &base()
            .import("rf_proto")
            .file("a.rs", ["FolderDTO"])
            .file("b.rs", [".rf_grpc.FolderDTO"]),
    );
    assert!(
        error.contains(".rf_grpc.FolderDTO is listed more than once"),
        "{error}"
    );
}

#[test]
fn reports_schema_errors() {
    let rf = || {
        Mirror::from_bytes(common::encode(&common::rf_grpc()))
            .package("rf_grpc")
            .import("rf_proto")
    };

    let error = render_err(&rf().file("a.rs", ["Nope"]));
    assert_eq!(error, ".rf_grpc.Nope is not in the descriptor set");

    // DTOData.attributes is an Any, which is not mirrored here.
    let error = render_err(&rf().file("a.rs", ["DTOData"]));
    assert_eq!(
        error,
        "rf_grpc.DTOData.attributes uses .google.protobuf.Any, which has no mirror; add it to a mirror file"
    );

    let mut set = common::rf_grpc();
    let folder = &mut set.file[2].message_type[0];
    folder.oneof_decl.push(prost_types::OneofDescriptorProto {
        name: Some("kind".into()),
        ..Default::default()
    });
    folder.field[1].oneof_index = Some(1);
    let error = render_err(
        &Mirror::from_bytes(common::encode(&set))
            .package("rf_grpc")
            .import("rf_proto")
            .file("a.rs", ["FolderDTO"]),
    );
    assert_eq!(error, "rf_grpc.FolderDTO.name: oneof is not supported");

    let error = render_err(
        &Mirror::from_bytes(b"\xff\xff".to_vec())
            .import("rf_proto")
            .file("a.rs", [".x.Y"]),
    );
    assert!(error.starts_with("invalid descriptor set"), "{error}");
}

#[test]
fn rust_names_must_be_unique() {
    let mut set = common::kitchen_sink();
    // A top-level `Child` next to `KitchenSink.Child`.
    set.file[0]
        .message_type
        .push(common::message("Child", vec![]));
    let error = render_err(
        &Mirror::from_bytes(common::encode(&set))
            .package("kinds")
            .import("crate::kinds")
            .file("a.rs", ["Child", "KitchenSink.Child"]),
    );
    assert_eq!(
        error,
        "invalid mirror configuration: .kinds.Child and .kinds.KitchenSink.Child both map to the Rust name Child"
    );
}
