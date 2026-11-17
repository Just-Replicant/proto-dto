// SPDX-License-Identifier: AGPL-3.0-or-later

//! Build-script helper for [`proto-dto`](https://docs.rs/proto-dto): renders
//! `flutter_rust_bridge` mirror structs from a protobuf descriptor set.
//!
//! The bridge only generates Dart classes for types under the Flutter crate's
//! `crate::api`. For a prost type defined elsewhere it needs a
//! `#[frb(mirror(T))] pub struct _T { .. }` that lists every field again. This
//! crate renders those files from the `FileDescriptorSet` that `prost-build` /
//! `tonic-prost-build` already write, so the mirrors cannot drift from the
//! `.proto` files:
//!
//! * the descriptor walk maps prost field kinds to the Rust types prost
//!   generates: `optional` and message fields → `Option<T>`, `repeated` →
//!   `Vec<T>`, `map<K, V>` → `HashMap<K, V>`, enums → `i32`, recursive
//!   fields → `Box<T>`, scalars to their primitive;
//! * names follow prost through [`heck`] (`FolderDTO` → `FolderDto`,
//!   `parentId` → `parent_id`) and Rust keywords are escaped the same way
//!   (`type` → `r#type`, `self` → `self_`), see [`naming`];
//! * outputs are written only when their content changes
//!   ([`write_if_changed`]), so file watchers are not retriggered;
//! * an optional check file holds exhaustive `let Type { a, b } = value`
//!   conversions between every prost type and a copy of its mirror. Include it
//!   in a `#[cfg(test)]` module of the crate that owns the prost types, and a
//!   `.proto` change without regenerated mirrors stops compiling.
//!
//! This crate is meant for `[build-dependencies]` only. Library users of
//! `proto-dto` never compile `heck` or the file writer.
//!
//! # Example: `rf-proto/build.rs`
//!
//! ```no_run
//! # fn compile_protos(_: &std::path::Path) {}
//! use std::path::PathBuf;
//!
//! use proto_dto_build::{Message, Mirror};
//!
//! fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let out_dir = PathBuf::from(std::env::var_os("OUT_DIR").ok_or("OUT_DIR is not set")?);
//!     let descriptor_path = out_dir.join("rf_grpc_descriptor.bin");
//!     // tonic_prost_build::configure()
//!     //     .file_descriptor_set_path(&descriptor_path)
//!     //     .compile_protos(PROTOS, INCLUDES)?;
//!     # compile_protos(&descriptor_path);
//!
//!     Mirror::new(&descriptor_path)
//!         .package("rf_grpc")
//!         .import("rf_proto")
//!         .generated_by("rust/rf-proto/build.rs")
//!         .file(
//!             "../src/api/shared/dto.rs",
//!             [Message::new(".google.protobuf.Any"), Message::new("DTOData")],
//!         )
//!         .file(
//!             "../src/api/file_explorer/dto.rs",
//!             [Message::new("FolderDTO").freezed()],
//!         )
//!         .check_file(out_dir.join("frb_mirror_check.rs"))
//!         .generate()?;
//!     Ok(())
//! }
//! ```
//!
//! `../src/api/file_explorer/dto.rs` (relative paths resolve against the
//! build script's working directory, the package root) then contains:
//!
//! ```text
//! use flutter_rust_bridge::frb;
//!
//! pub use rf_proto::FolderDto;
//!
//! /// Mirror of [`FolderDto`] (`rf_grpc.FolderDTO`).
//! ///
//! /// Source: `explorer/folder.proto`.
//! #[frb(mirror(FolderDto), dart_metadata = ("freezed"))]
//! pub struct _FolderDto {
//!     pub parent_id: Option<i32>,
//!     pub name: String,
//!     // ...
//! }
//! ```
//!
//! and `rf-proto/src/lib.rs` checks it with:
//!
//! ```ignore
//! #[cfg(test)]
//! mod mirror_check {
//!     include!(concat!(env!("OUT_DIR"), "/frb_mirror_check.rs"));
//! }
//! ```
//!
//! # Limitations
//!
//! Real `oneof`s, `group`s and edition-based `.proto` files are rejected with
//! [`Error::Unsupported`]. The mirror assumes prost's defaults: `bytes` as
//! `Vec<u8>`, maps as `HashMap`, and no `Config::boxed` overrides.

#![cfg_attr(docsrs, feature(doc_cfg))]

mod descriptor;
mod error;
pub mod naming;
mod render;
mod write;

use std::path::{Path, PathBuf};

use prost::Message as _;
use prost_types::FileDescriptorSet;

pub use error::Error;
pub use write::write_if_changed;

/// Where the descriptor set comes from.
#[derive(Clone, Debug)]
enum Source {
    Path(PathBuf),
    Bytes(Vec<u8>),
}

/// Builder for the mirror files of one descriptor set.
///
/// Configure the package, the import path and one or more output files, then
/// call [`generate`](Self::generate) (writes) or [`render`](Self::render)
/// (returns the contents).
#[derive(Clone, Debug)]
#[must_use = "a Mirror does nothing until `generate` or `render` is called"]
pub struct Mirror {
    source: Source,
    package: String,
    import: String,
    check_import: String,
    generated_by: String,
    files: Vec<MirrorFile>,
    check_file: Option<PathBuf>,
}

/// One generated mirror file and the messages it holds.
#[derive(Clone, Debug)]
struct MirrorFile {
    path: PathBuf,
    messages: Vec<Message>,
}

/// A protobuf message to mirror.
///
/// The name is relative to [`Mirror::package`] (`FolderDTO`, or
/// `Outer.Inner` for a nested message), or fully qualified with a leading dot
/// for a message of another package (`.google.protobuf.Any`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Message {
    name: String,
    freezed: bool,
}

impl Message {
    /// Mirrors the message called `name`.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            freezed: false,
        }
    }

    /// Emits a `freezed` Dart class: `dart_metadata = ("freezed")`.
    #[must_use]
    pub fn freezed(mut self) -> Self {
        self.freezed = true;
        self
    }
}

impl From<&str> for Message {
    fn from(name: &str) -> Self {
        Self::new(name)
    }
}

impl From<String> for Message {
    fn from(name: String) -> Self {
        Self::new(name)
    }
}

/// One rendered output file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RenderedFile {
    /// Where [`Mirror::generate`] writes it.
    pub path: PathBuf,
    /// The full file contents.
    pub contents: String,
}

/// What [`Mirror::generate`] did.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Generated {
    /// Every output file, mirrors first, then the check file.
    pub files: Vec<PathBuf>,
    /// The files whose contents changed and were rewritten.
    pub written: Vec<PathBuf>,
}

impl Mirror {
    /// Reads the descriptor set from `path`, e.g. the file passed to
    /// `prost_build::Config::file_descriptor_set_path`.
    ///
    /// Include source info (prost-build does by default) to carry the
    /// `.proto` comments over as rustdoc.
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self::with_source(Source::Path(path.into()))
    }

    /// Uses an already encoded `FileDescriptorSet`.
    pub fn from_bytes(bytes: impl Into<Vec<u8>>) -> Self {
        Self::with_source(Source::Bytes(bytes.into()))
    }

    fn with_source(source: Source) -> Self {
        Self {
            source,
            package: String::new(),
            import: String::new(),
            check_import: String::from("crate"),
            generated_by: String::from("build.rs"),
            files: Vec::new(),
            check_file: None,
        }
    }

    /// Protobuf package that relative message names belong to, e.g.
    /// `rf_grpc`.
    pub fn package(mut self, package: impl Into<String>) -> Self {
        package
            .into()
            .trim_start_matches('.')
            .clone_into(&mut self.package);
        self
    }

    /// Rust path the mirror files import the prost types from, e.g.
    /// `rf_proto` for `pub use rf_proto::FolderDto;`. Required.
    ///
    /// Every mirrored type must be reachable there, nested messages in
    /// prost's modules (`rf_proto::outer::Inner`); re-export the types of
    /// other packages, such as `prost_types::Any`, at the same place.
    pub fn import(mut self, path: impl Into<String>) -> Self {
        path.into()
            .trim_end_matches("::")
            .clone_into(&mut self.import);
        self
    }

    /// Path the check file names the prost types by, from the module that
    /// `include!`s it. Defaults to `crate`.
    pub fn check_import(mut self, path: impl Into<String>) -> Self {
        path.into()
            .trim_end_matches("::")
            .clone_into(&mut self.check_import);
        self
    }

    /// Names the generator in the `@generated` header, e.g.
    /// `rust/rf-proto/build.rs`. Defaults to `build.rs`.
    pub fn generated_by(mut self, generator: impl Into<String>) -> Self {
        self.generated_by = generator.into();
        self
    }

    /// Adds a mirror file at `path` holding `messages`, in that order.
    ///
    /// Relative paths resolve against the current directory, which for a
    /// build script is the package root.
    pub fn file<M: Into<Message>>(
        mut self,
        path: impl Into<PathBuf>,
        messages: impl IntoIterator<Item = M>,
    ) -> Self {
        self.files.push(MirrorFile {
            path: path.into(),
            messages: messages.into_iter().map(Into::into).collect(),
        });
        self
    }

    /// Also writes the exhaustive conversion check to `path`, usually in
    /// `OUT_DIR`.
    pub fn check_file(mut self, path: impl Into<PathBuf>) -> Self {
        self.check_file = Some(path.into());
        self
    }

    /// Renders every output without writing anything.
    ///
    /// # Errors
    ///
    /// * [`Error::Io`] / [`Error::Decode`] when the descriptor set cannot be
    ///   read;
    /// * [`Error::Config`] for no files, an empty file, no import, duplicate
    ///   paths or messages;
    /// * [`Error::MessageNotFound`], [`Error::NotMirrored`] and
    ///   [`Error::Unsupported`] for messages the mirror cannot express.
    pub fn render(&self) -> Result<Vec<RenderedFile>, Error> {
        self.validate()?;
        let set = self.descriptor_set()?;
        let index = descriptor::Index::new(&set);
        let output = render::render(self, &index)?;

        let mut files: Vec<RenderedFile> = self
            .files
            .iter()
            .zip(output.mirrors)
            .map(|(file, contents)| RenderedFile {
                path: file.path.clone(),
                contents,
            })
            .collect();
        if let (Some(path), Some(contents)) = (&self.check_file, output.check) {
            files.push(RenderedFile {
                path: path.clone(),
                contents,
            });
        }
        Ok(files)
    }

    /// Renders every output and writes the ones whose content changed.
    ///
    /// # Errors
    ///
    /// Everything [`render`](Self::render) returns, and [`Error::Io`] when a
    /// file cannot be written.
    pub fn generate(&self) -> Result<Generated, Error> {
        let mut generated = Generated::default();
        for file in self.render()? {
            if write_if_changed(&file.path, &file.contents)? {
                generated.written.push(file.path.clone());
            }
            generated.files.push(file.path);
        }
        Ok(generated)
    }

    fn validate(&self) -> Result<(), Error> {
        let config = |reason: String| Err(Error::Config(reason));
        if self.import.is_empty() {
            return config("`import` is required, e.g. `.import(\"rf_proto\")`".into());
        }
        if self.files.is_empty() {
            return config("no mirror file; add one with `.file(path, messages)`".into());
        }
        let mut paths: Vec<&Path> = self.files.iter().map(|file| file.path.as_path()).collect();
        paths.extend(self.check_file.as_deref());
        for (index, path) in paths.iter().enumerate() {
            if paths[..index].contains(path) {
                return config(format!("{} is used for two outputs", path.display()));
            }
        }
        if let Some(file) = self.files.iter().find(|file| file.messages.is_empty()) {
            return config(format!("{} lists no messages", file.path.display()));
        }
        if let Some(message) = self
            .files
            .iter()
            .flat_map(|file| &file.messages)
            .find(|message| message.name.trim_start_matches('.').is_empty())
        {
            return config(format!("invalid message name {:?}", message.name));
        }
        Ok(())
    }

    fn descriptor_set(&self) -> Result<FileDescriptorSet, Error> {
        match &self.source {
            Source::Path(path) => {
                let bytes = std::fs::read(path).map_err(|source| Error::Io {
                    path: path.clone(),
                    source,
                })?;
                Ok(FileDescriptorSet::decode(bytes.as_slice())?)
            }
            Source::Bytes(bytes) => Ok(FileDescriptorSet::decode(bytes.as_slice())?),
        }
    }
}
