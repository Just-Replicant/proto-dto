// SPDX-License-Identifier: AGPL-3.0-or-later

//! The descriptor walk: every message of a `FileDescriptorSet`, by
//! fully-qualified name, with its doc comments and prost module path.

use std::collections::{BTreeMap, BTreeSet};

use prost_types::field_descriptor_proto::{Label, Type};
use prost_types::{DescriptorProto, FileDescriptorProto, FileDescriptorSet};

use crate::naming;

/// `FileDescriptorProto.message_type` field number (`source_code_info` paths).
const FILE_MESSAGE_TYPE: i32 = 4;
/// `DescriptorProto.field` field number.
const MESSAGE_FIELD: i32 = 2;
/// `DescriptorProto.nested_type` field number.
const MESSAGE_NESTED_TYPE: i32 = 3;

/// Which presence rules a `.proto` file uses.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Syntax {
    /// `syntax = "proto2"` (or no `syntax` line): every `optional` field has
    /// presence.
    Proto2,
    /// `syntax = "proto3"`: only `optional` scalars and messages have presence.
    Proto3,
    /// `edition = "..."`: not supported by the mirror.
    Editions,
}

/// One message found in the descriptor set.
pub(crate) struct MessageInfo<'a> {
    /// `.package.Outer.Inner`.
    pub(crate) full_name: String,
    /// `.proto` file that declares the message, as passed to `protoc`.
    pub(crate) file: &'a str,
    pub(crate) syntax: Syntax,
    pub(crate) descriptor: &'a DescriptorProto,
    /// prost modules between the package root and the type: `["outer"]`
    /// for `Outer.Inner`.
    pub(crate) modules: Vec<String>,
    /// Leading comment of the message.
    pub(crate) docs: Option<String>,
    /// Leading comment of every field, in field order.
    pub(crate) field_docs: Vec<Option<String>>,
}

impl MessageInfo<'_> {
    /// The full name without the leading dot, for messages and docs.
    pub(crate) fn display_name(&self) -> &str {
        self.full_name.trim_start_matches('.')
    }

    /// `true` for the synthetic `FooEntry` message of a `map<K, V>` field.
    pub(crate) fn is_map_entry(&self) -> bool {
        self.descriptor
            .options
            .as_ref()
            .is_some_and(prost_types::MessageOptions::map_entry)
    }
}

/// All messages of a descriptor set by fully-qualified name.
pub(crate) struct Index<'a> {
    messages: BTreeMap<String, MessageInfo<'a>>,
}

impl<'a> Index<'a> {
    pub(crate) fn new(set: &'a FileDescriptorSet) -> Self {
        let mut messages = BTreeMap::new();
        for file in &set.file {
            index_file(file, &mut messages);
        }
        Self { messages }
    }

    pub(crate) fn get(&self, full_name: &str) -> Option<&MessageInfo<'a>> {
        self.messages.get(full_name)
    }

    /// Whether `from` contains `to` through singular message fields, which
    /// is when prost boxes a field of type `from` inside `to`.
    pub(crate) fn reaches(&self, from: &str, to: &str) -> bool {
        let mut seen = BTreeSet::new();
        let mut stack = vec![from];
        while let Some(name) = stack.pop() {
            if name == to {
                return true;
            }
            if !seen.insert(name) {
                continue;
            }
            let Some(message) = self.messages.get(name) else {
                continue;
            };
            stack.extend(
                message
                    .descriptor
                    .field
                    .iter()
                    .filter(|field| {
                        field.r#type() == Type::Message && field.label() != Label::Repeated
                    })
                    .map(prost_types::FieldDescriptorProto::type_name),
            );
        }
        false
    }
}

fn index_file<'a>(file: &'a FileDescriptorProto, out: &mut BTreeMap<String, MessageInfo<'a>>) {
    let package = file.package();
    let syntax = match file.syntax() {
        "proto3" => Syntax::Proto3,
        "editions" => Syntax::Editions,
        _ => Syntax::Proto2,
    };
    // Well-known types carry long upstream docs that do not help Dart
    // callers; their mirrors keep only the one-line summary.
    let keep_docs = package != "google.protobuf";
    let comments: BTreeMap<&[i32], &str> = file
        .source_code_info
        .iter()
        .flat_map(|info| &info.location)
        .filter(|_| keep_docs)
        .filter_map(|location| {
            let comment = location.leading_comments.as_deref()?.trim();
            (!comment.is_empty()).then_some((location.path.as_slice(), comment))
        })
        .collect();

    let scope = if package.is_empty() {
        String::new()
    } else {
        format!(".{package}")
    };
    let context = FileContext {
        file: file.name(),
        syntax,
        comments: &comments,
    };
    for (index, message) in file.message_type.iter().enumerate() {
        let path = [FILE_MESSAGE_TYPE, to_i32(index)];
        index_message(&context, &scope, &[], message, &path, out);
    }
}

struct FileContext<'f, 'a> {
    file: &'a str,
    syntax: Syntax,
    comments: &'f BTreeMap<&'a [i32], &'a str>,
}

fn index_message<'a>(
    context: &FileContext<'_, 'a>,
    scope: &str,
    modules: &[String],
    message: &'a DescriptorProto,
    path: &[i32],
    out: &mut BTreeMap<String, MessageInfo<'a>>,
) {
    let full_name = format!("{scope}.{}", message.name());
    let comment = |path: &[i32]| context.comments.get(path).map(|&docs| docs.to_owned());

    let field_docs = (0..message.field.len())
        .map(|index| comment(&[path, &[MESSAGE_FIELD, to_i32(index)]].concat()))
        .collect();

    let mut nested_modules = modules.to_vec();
    nested_modules.push(naming::field_name(message.name()));
    for (index, nested) in message.nested_type.iter().enumerate() {
        let nested_path = [path, &[MESSAGE_NESTED_TYPE, to_i32(index)]].concat();
        index_message(
            context,
            &full_name,
            &nested_modules,
            nested,
            &nested_path,
            out,
        );
    }

    out.insert(
        full_name.clone(),
        MessageInfo {
            full_name,
            file: context.file,
            syntax: context.syntax,
            descriptor: message,
            modules: modules.to_vec(),
            docs: comment(path),
            field_docs,
        },
    );
}

fn to_i32(index: usize) -> i32 {
    i32::try_from(index).expect("descriptor index fits in i32")
}
