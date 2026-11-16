// SPDX-License-Identifier: AGPL-3.0-or-later

//! Compiles the golden fixtures against stand-ins for the prost output.
//!
//! The stand-ins below are what prost generates for the schemas in
//! `tests/common` (field names, types, `Box`, nested modules). If the
//! generator renders a field differently from prost, the exhaustive
//! conversions in the check fixtures stop compiling, which is exactly what
//! they are meant to do in a real crate.

// The mirrors are read by flutter_rust_bridge, not rustdoc.
#![allow(missing_docs, unreachable_pub)]

// The rf_grpc fixtures import `rf_proto::...`, as the Flutter crate would.
extern crate self as rf_proto;

pub use prost_types::Any;

#[derive(Clone, Debug, PartialEq)]
pub struct DtoData {
    pub id: Option<i32>,
    pub r#type: i32,
    pub attributes: Option<Any>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FolderDto {
    pub parent_id: Option<i32>,
    pub name: String,
    pub path: String,
    pub created_at: String,
    pub updated_at: String,
}

pub mod kinds {
    use std::collections::HashMap;

    #[derive(Clone, Debug, Default, PartialEq)]
    pub struct KitchenSink {
        pub double_value: f64,
        pub float_value: f32,
        pub int64_value: i64,
        pub sint64_value: i64,
        pub sfixed64_value: i64,
        pub uint64_value: u64,
        pub fixed64_value: u64,
        pub int32_value: i32,
        pub sint32_value: i32,
        pub sfixed32_value: i32,
        pub uint32_value: u32,
        pub fixed32_value: u32,
        pub bool_value: bool,
        pub string_value: String,
        pub bytes_value: Vec<u8>,
        pub color: i32,
        pub maybe: Option<String>,
        pub tags: Vec<String>,
        pub colors: Vec<i32>,
        pub child: Option<kitchen_sink::Child>,
        pub children: Vec<kitchen_sink::Child>,
        pub counts: HashMap<String, i64>,
        pub by_name: HashMap<String, kitchen_sink::Child>,
        pub r#type: String,
        pub r#match: bool,
        pub self_: i32,
        pub r#async: i32,
        pub created_at: String,
        pub next: Option<Box<KitchenSink>>,
    }

    pub mod kitchen_sink {
        #[derive(Clone, Debug, Default, PartialEq)]
        pub struct Child {
            pub id: i64,
        }
    }

    #[derive(Clone, Debug, Default, PartialEq)]
    pub struct Empty {}

    #[derive(Clone, Debug, Default, PartialEq)]
    pub struct Legacy {
        pub optional_name: Option<String>,
        pub required_id: i32,
        pub values: Vec<i32>,
    }
}

/// What the Flutter crate's `src/api` looks like with the mirrors written.
pub mod api {
    pub mod shared {
        include!("fixtures/rf_grpc/shared_dto.rs");
    }
    pub mod file_explorer {
        include!("fixtures/rf_grpc/file_explorer_dto.rs");
    }
    pub mod sink {
        include!("fixtures/kinds/sink.rs");
    }
    pub mod child {
        include!("fixtures/kinds/child.rs");
    }
}

mod rf_grpc_check {
    include!("fixtures/rf_grpc/check.rs");

    #[test]
    fn round_trips_every_field() {
        let folder = crate::FolderDto {
            parent_id: Some(1),
            name: "docs".into(),
            path: "/docs".into(),
            created_at: "c".into(),
            updated_at: "u".into(),
        };
        assert_eq!(
            crate::FolderDto::from(_FolderDto::from(folder.clone())),
            folder
        );

        let any = crate::Any {
            type_url: "type.googleapis.com/rf_grpc.FolderDTO".into(),
            value: vec![1, 2, 3],
        };
        assert_eq!(crate::Any::from(_Any::from(any.clone())), any);

        let dto = crate::DtoData {
            id: Some(4),
            r#type: 2,
            attributes: Some(any),
        };
        assert_eq!(crate::DtoData::from(_DtoData::from(dto.clone())), dto);
    }
}

mod kinds_check {
    include!("fixtures/kinds/check.rs");

    use crate::kinds::{Empty, KitchenSink, Legacy, kitchen_sink::Child};

    #[test]
    fn round_trips_every_field() {
        let sink = KitchenSink {
            r#type: "t".into(),
            self_: 3,
            child: Some(Child { id: 9 }),
            counts: [("a".to_owned(), 1)].into(),
            next: Some(Box::new(KitchenSink::default())),
            ..KitchenSink::default()
        };
        assert_eq!(KitchenSink::from(_KitchenSink::from(sink.clone())), sink);
        assert_eq!(Empty::from(_Empty::from(Empty {})), Empty {});
        let legacy = Legacy {
            optional_name: None,
            required_id: 7,
            values: vec![1],
        };
        assert_eq!(Legacy::from(_Legacy::from(legacy.clone())), legacy);
    }
}

#[test]
fn mirrors_resolve() {
    // The `pub use` lines re-export the prost types next to their mirrors.
    let _: api::file_explorer::FolderDto = FolderDto {
        parent_id: None,
        name: String::new(),
        path: String::new(),
        created_at: String::new(),
        updated_at: String::new(),
    };
    let _: api::sink::Empty = kinds::Empty {};
    let _: api::child::Legacy = kinds::Legacy::default();
}
