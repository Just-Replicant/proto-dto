// SPDX-License-Identifier: AGPL-3.0-or-later

//! In-memory descriptor sets for the tests, shaped like what `protoc`
//! writes for the reference `rf_grpc` schema plus one message per field kind.

#![allow(dead_code, unreachable_pub)]

use prost::Message as _;
use prost_types::field_descriptor_proto::{Label, Type};
use prost_types::source_code_info::Location;
use prost_types::{
    DescriptorProto, EnumDescriptorProto, EnumValueDescriptorProto, FieldDescriptorProto,
    FileDescriptorProto, FileDescriptorSet, MessageOptions, OneofDescriptorProto, SourceCodeInfo,
};

pub fn field(name: &str, number: i32, ty: Type) -> FieldDescriptorProto {
    FieldDescriptorProto {
        name: Some(name.to_owned()),
        number: Some(number),
        label: Some(Label::Optional as i32),
        r#type: Some(ty as i32),
        ..FieldDescriptorProto::default()
    }
}

pub fn typed(name: &str, number: i32, ty: Type, type_name: &str) -> FieldDescriptorProto {
    FieldDescriptorProto {
        type_name: Some(type_name.to_owned()),
        ..field(name, number, ty)
    }
}

pub fn repeated(mut field: FieldDescriptorProto) -> FieldDescriptorProto {
    field.label = Some(Label::Repeated as i32);
    field
}

/// `optional` in a proto3 file: a synthetic oneof at `oneof_index`.
pub fn proto3_optional(mut field: FieldDescriptorProto, oneof_index: i32) -> FieldDescriptorProto {
    field.proto3_optional = Some(true);
    field.oneof_index = Some(oneof_index);
    field
}

pub fn message(name: &str, fields: Vec<FieldDescriptorProto>) -> DescriptorProto {
    DescriptorProto {
        name: Some(name.to_owned()),
        field: fields,
        ..DescriptorProto::default()
    }
}

pub fn map_entry(name: &str, key: Type, value: FieldDescriptorProto) -> DescriptorProto {
    DescriptorProto {
        options: Some(MessageOptions {
            map_entry: Some(true),
            ..MessageOptions::default()
        }),
        ..message(name, vec![field("key", 1, key), value])
    }
}

pub fn oneofs(names: &[&str]) -> Vec<OneofDescriptorProto> {
    names
        .iter()
        .map(|name| OneofDescriptorProto {
            name: Some((*name).to_owned()),
            ..OneofDescriptorProto::default()
        })
        .collect()
}

pub fn file(
    name: &str,
    package: &str,
    syntax: &str,
    messages: Vec<DescriptorProto>,
) -> FileDescriptorProto {
    FileDescriptorProto {
        name: Some(name.to_owned()),
        package: Some(package.to_owned()),
        syntax: (!syntax.is_empty()).then(|| syntax.to_owned()),
        message_type: messages,
        ..FileDescriptorProto::default()
    }
}

pub fn comment(path: &[i32], text: &str) -> Location {
    Location {
        path: path.to_vec(),
        leading_comments: Some(text.to_owned()),
        ..Location::default()
    }
}

/// `google/protobuf/any.proto`, with upstream-style docs that the mirror drops.
pub fn any_file() -> FileDescriptorProto {
    FileDescriptorProto {
        source_code_info: Some(SourceCodeInfo {
            location: vec![comment(&[4, 0], " Long upstream documentation.\n")],
        }),
        ..file(
            "google/protobuf/any.proto",
            "google.protobuf",
            "proto3",
            vec![message(
                "Any",
                vec![
                    field("type_url", 1, Type::String),
                    field("value", 2, Type::Bytes),
                ],
            )],
        )
    }
}

/// The reference schema: `DTOData` around an `Any`, and `FolderDTO`.
pub fn rf_grpc() -> FileDescriptorSet {
    let mut dto_data = message(
        "DTOData",
        vec![
            proto3_optional(field("id", 1, Type::Int32), 0),
            typed("type", 2, Type::Enum, ".rf_grpc.DTOData.DataType"),
            typed("attributes", 3, Type::Message, ".google.protobuf.Any"),
        ],
    );
    dto_data.oneof_decl = oneofs(&["_id"]);
    dto_data.enum_type = vec![EnumDescriptorProto {
        name: Some("DataType".into()),
        value: vec![EnumValueDescriptorProto {
            name: Some("UNSPECIFIED".into()),
            number: Some(0),
            ..EnumValueDescriptorProto::default()
        }],
        ..EnumDescriptorProto::default()
    }];
    let shared = file("shared.proto", "rf_grpc", "proto3", vec![dto_data]);

    let mut folder = message(
        "FolderDTO",
        vec![
            proto3_optional(field("parent_id", 1, Type::Int32), 0),
            field("name", 2, Type::String),
            field("path", 3, Type::String),
            field("created_at", 4, Type::String),
            field("updated_at", 5, Type::String),
        ],
    );
    folder.oneof_decl = oneofs(&["_parent_id"]);
    let folder = FileDescriptorProto {
        source_code_info: Some(SourceCodeInfo {
            location: vec![
                comment(&[4, 0], " A folder on a device.\n\n Paths are absolute.\n"),
                comment(&[4, 0, 2, 0], " `None` for a root folder.\n"),
            ],
        }),
        ..file("explorer/folder.proto", "rf_grpc", "proto3", vec![folder])
    };

    FileDescriptorSet {
        file: vec![any_file(), shared, folder],
    }
}

/// One message per field kind, in package `kinds`.
pub fn kitchen_sink() -> FileDescriptorSet {
    let mut sink = message(
        "KitchenSink",
        vec![
            field("double_value", 1, Type::Double),
            field("float_value", 2, Type::Float),
            field("int64_value", 3, Type::Int64),
            field("sint64_value", 4, Type::Sint64),
            field("sfixed64_value", 5, Type::Sfixed64),
            field("uint64_value", 6, Type::Uint64),
            field("fixed64_value", 7, Type::Fixed64),
            field("int32_value", 8, Type::Int32),
            field("sint32_value", 9, Type::Sint32),
            field("sfixed32_value", 10, Type::Sfixed32),
            field("uint32_value", 11, Type::Uint32),
            field("fixed32_value", 12, Type::Fixed32),
            field("bool_value", 13, Type::Bool),
            field("string_value", 14, Type::String),
            field("bytes_value", 15, Type::Bytes),
            typed("color", 16, Type::Enum, ".kinds.Color"),
            proto3_optional(field("maybe", 17, Type::String), 0),
            repeated(field("tags", 18, Type::String)),
            repeated(typed("colors", 19, Type::Enum, ".kinds.Color")),
            typed("child", 20, Type::Message, ".kinds.KitchenSink.Child"),
            repeated(typed(
                "children",
                21,
                Type::Message,
                ".kinds.KitchenSink.Child",
            )),
            repeated(typed(
                "counts",
                22,
                Type::Message,
                ".kinds.KitchenSink.CountsEntry",
            )),
            repeated(typed(
                "by_name",
                23,
                Type::Message,
                ".kinds.KitchenSink.ByNameEntry",
            )),
            // Rust keywords.
            field("type", 24, Type::String),
            field("match", 25, Type::Bool),
            field("self", 26, Type::Int32),
            field("async", 27, Type::Int32),
            // camelCase in the .proto file.
            field("createdAt", 28, Type::String),
            typed("next", 29, Type::Message, ".kinds.KitchenSink"),
        ],
    );
    sink.oneof_decl = oneofs(&["_maybe"]);
    sink.nested_type = vec![
        message("Child", vec![field("id", 1, Type::Int64)]),
        map_entry("CountsEntry", Type::String, field("value", 2, Type::Int64)),
        map_entry(
            "ByNameEntry",
            Type::String,
            typed("value", 2, Type::Message, ".kinds.KitchenSink.Child"),
        ),
    ];
    let kinds = FileDescriptorProto {
        source_code_info: Some(SourceCodeInfo {
            location: vec![comment(&[4, 0, 2, 23], " Escaped as `r#type`.\n")],
        }),
        ..file(
            "kinds.proto",
            "kinds",
            "proto3",
            vec![sink, message("Empty", vec![])],
        )
    };

    let legacy = file(
        "legacy.proto",
        "kinds",
        "proto2",
        vec![message(
            "Legacy",
            vec![
                field("optional_name", 1, Type::String),
                FieldDescriptorProto {
                    label: Some(Label::Required as i32),
                    ..field("required_id", 2, Type::Int32)
                },
                repeated(field("values", 3, Type::Int32)),
            ],
        )],
    );

    FileDescriptorSet {
        file: vec![kinds, legacy],
    }
}

pub fn encode(set: &FileDescriptorSet) -> Vec<u8> {
    set.encode_to_vec()
}

/// A fresh, empty directory under the target temp dir.
pub fn temp_dir(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir()
        .join("proto-dto-build-tests")
        .join(format!("{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create temp dir");
    dir
}
