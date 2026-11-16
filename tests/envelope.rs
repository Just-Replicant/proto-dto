// SPDX-License-Identifier: AGPL-3.0-or-later

//! `envelope!` against hand-written prost types that mirror what prost-build
//! generates for the reference `rf_grpc` schema (`shared.proto`,
//! `device.proto`, `hard_drive.proto`, `file.proto`, `folder.proto`): same
//! field names, tags and Rust types.

#![allow(missing_docs, unreachable_pub, clippy::must_use_candidate)]

use proto_dto::AttributeError;

mod rf_grpc {
    /// `message DTOData`.
    #[derive(Clone, PartialEq, prost::Message)]
    #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
    #[cfg_attr(
        feature = "serde",
        serde(try_from = "crate::DtoDataJson", into = "crate::DtoDataJson")
    )]
    pub struct DtoData {
        #[prost(int32, optional, tag = "1")]
        pub id: Option<i32>,
        #[prost(enumeration = "dto_data::DataType", tag = "2")]
        pub r#type: i32,
        #[prost(message, optional, tag = "3")]
        pub attributes: Option<prost_types::Any>,
    }

    pub mod dto_data {
        /// `enum DTOData.DataType`.
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, prost::Enumeration)]
        #[repr(i32)]
        pub enum DataType {
            Unspecified = 0,
            File = 1,
            Folder = 2,
            Device = 3,
            HardDrive = 4,
        }

        impl DataType {
            // prost-build emits this; `#[derive(Enumeration)]` alone does not.
            pub fn as_str_name(&self) -> &'static str {
                match self {
                    Self::Unspecified => "UNSPECIFIED",
                    Self::File => "FILE",
                    Self::Folder => "FOLDER",
                    Self::Device => "DEVICE",
                    Self::HardDrive => "HARD_DRIVE",
                }
            }
        }
    }

    /// `message DeviceDTO`.
    #[derive(Clone, PartialEq, prost::Message)]
    #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
    pub struct DeviceDto {
        #[prost(string, tag = "1")]
        pub name: String,
        #[prost(string, tag = "2")]
        pub uuid: String,
        #[prost(string, tag = "3")]
        pub serial_number: String,
        #[prost(string, tag = "4")]
        pub vendor_name: String,
        #[prost(string, tag = "5")]
        pub version: String,
        #[prost(bool, tag = "6")]
        pub is_current: bool,
    }

    /// `message HardDriveDTO`.
    #[derive(Clone, PartialEq, prost::Message)]
    #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
    pub struct HardDriveDto {
        #[prost(int32, tag = "1")]
        pub device_id: i32,
        #[prost(string, tag = "2")]
        pub name: String,
        #[prost(string, tag = "3")]
        pub kind: String,
        #[prost(string, tag = "4")]
        pub file_system: String,
        #[prost(string, tag = "5")]
        pub mount_point: String,
        #[prost(int64, tag = "6")]
        pub total_space: i64,
        #[prost(int64, tag = "7")]
        pub available_space: i64,
        #[prost(bool, tag = "8")]
        pub is_removable: bool,
        #[prost(bool, tag = "9")]
        pub is_read_only: bool,
        #[prost(int64, tag = "10")]
        pub usage_read_bytes: i64,
        #[prost(int64, tag = "11")]
        pub usage_written_bytes: i64,
        #[prost(int64, tag = "12")]
        pub usage_total_read_bytes: i64,
        #[prost(int64, tag = "13")]
        pub usage_total_written_bytes: i64,
        #[prost(string, tag = "14")]
        pub created_at: String,
        #[prost(string, tag = "15")]
        pub updated_at: String,
    }

    /// `message FileDTO`.
    #[derive(Clone, PartialEq, prost::Message)]
    #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
    pub struct FileDto {
        #[prost(string, tag = "2")]
        pub name: String,
        #[prost(string, tag = "3")]
        pub extension: String,
        #[prost(string, tag = "4")]
        pub path: String,
        #[prost(string, tag = "5")]
        pub size: String,
        #[prost(string, tag = "6")]
        pub hash: String,
        #[prost(string, tag = "7")]
        pub created_at: String,
        #[prost(string, tag = "8")]
        pub updated_at: String,
    }

    /// `message FolderDTO`.
    #[derive(Clone, PartialEq, prost::Message)]
    #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
    pub struct FolderDto {
        #[prost(int32, optional, tag = "2")]
        pub parent_id: Option<i32>,
        #[prost(string, tag = "3")]
        pub name: String,
        #[prost(string, tag = "4")]
        pub path: String,
        #[prost(string, tag = "5")]
        pub created_at: String,
        #[prost(string, tag = "6")]
        pub updated_at: String,
    }
}

// What rf-proto's lib.rs re-exports, followed by its one invocation.
pub use rf_grpc::{DeviceDto, DtoData, FileDto, FolderDto, HardDriveDto, dto_data::DataType};

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

fn device() -> DeviceDto {
    DeviceDto {
        name: "desk".into(),
        uuid: "uuid".into(),
        serial_number: "sn".into(),
        vendor_name: "vendor".into(),
        version: "1".into(),
        is_current: true,
    }
}

fn folder() -> FolderDto {
    FolderDto {
        parent_id: Some(1),
        name: "docs".into(),
        path: "/docs".into(),
        created_at: "2026-01-01".into(),
        updated_at: "2026-01-02".into(),
    }
}

fn file() -> FileDto {
    FileDto {
        name: "a".into(),
        extension: "txt".into(),
        path: "/docs/a.txt".into(),
        size: "12".into(),
        hash: "abc".into(),
        created_at: String::new(),
        updated_at: String::new(),
    }
}

fn hard_drive() -> HardDriveDto {
    HardDriveDto {
        device_id: 3,
        name: "nvme0".into(),
        kind: "SSD".into(),
        file_system: "ext4".into(),
        mount_point: "/".into(),
        total_space: 1 << 40,
        available_space: 1 << 39,
        is_removable: false,
        is_read_only: false,
        usage_read_bytes: 1,
        usage_written_bytes: 2,
        usage_total_read_bytes: 3,
        usage_total_written_bytes: 4,
        created_at: String::new(),
        updated_at: String::new(),
    }
}

#[test]
fn from_sets_id_discriminant_and_type_url() {
    let dto = DtoData::from_folder(Some(5), folder());
    assert_eq!(dto.id, Some(5));
    assert_eq!(dto.r#type(), DataType::Folder);
    let any = dto.attributes.as_ref().expect("attributes");
    assert_eq!(any.type_url, FolderDto::TYPE_URL);
    assert_eq!(any.type_url, "type.googleapis.com/rf_grpc.FolderDTO");
    assert_eq!(any.value, prost::Message::encode_to_vec(&folder()));
}

#[test]
fn every_row_packs_and_unpacks_with_its_own_tag() {
    let cases = [
        (
            DtoData::from_file(Some(1), file()),
            DataType::File,
            FileDto::TYPE_URL,
        ),
        (
            DtoData::from_folder(Some(2), folder()),
            DataType::Folder,
            FolderDto::TYPE_URL,
        ),
        (
            DtoData::from_device(Some(3), device()),
            DataType::Device,
            DeviceDto::TYPE_URL,
        ),
        (
            DtoData::from_hard_drive(Some(4), hard_drive()),
            DataType::HardDrive,
            HardDriveDto::TYPE_URL,
        ),
    ];
    for (dto, data_type, type_url) in &cases {
        assert_eq!(dto.r#type(), *data_type);
        assert_eq!(&dto.attributes.as_ref().expect("any").type_url, type_url);
    }
    assert_eq!(cases[0].0.try_as_file().expect("file"), file());
    assert_eq!(cases[1].0.try_as_folder().expect("folder"), folder());
    assert_eq!(cases[2].0.try_as_device().expect("device"), device());
    assert_eq!(
        cases[3].0.try_as_hard_drive().expect("hard drive"),
        hard_drive()
    );
}

#[test]
fn type_urls_are_distinct() {
    let urls = [
        FileDto::TYPE_URL,
        FolderDto::TYPE_URL,
        DeviceDto::TYPE_URL,
        HardDriveDto::TYPE_URL,
    ];
    for (index, url) in urls.iter().enumerate() {
        assert!(url.starts_with("type.googleapis.com/rf_grpc."));
        assert!(!urls[index + 1..].contains(url));
    }
}

#[test]
fn protobuf_roundtrip_through_bytes() {
    let original = DtoData::from_device(Some(9), device());
    let bytes = prost::Message::encode_to_vec(&original);
    let decoded = <DtoData as prost::Message>::decode(bytes.as_slice()).expect("decode");
    assert_eq!(decoded, original);
    assert_eq!(decoded.try_as_device().expect("unpack"), device());
}

#[test]
fn default_message_packs_to_empty_bytes() {
    let dto = DtoData::from_device(None, DeviceDto::default());
    assert_eq!(
        dto.attributes.as_ref().expect("any").value,
        Vec::<u8>::new()
    );
    assert_eq!(dto.try_as_device().expect("unpack"), DeviceDto::default());
}

#[test]
fn try_as_rejects_other_discriminant() {
    let dto = DtoData::from_folder(None, folder());
    let error = dto.try_as_device().expect_err("mismatch");
    match &error {
        AttributeError::TypeMismatch { expected, actual } => {
            assert_eq!(*expected, "DEVICE");
            assert_eq!(*actual, DataType::Folder as i32);
        }
        other => panic!("unexpected {other:?}"),
    }
    assert!(error.to_string().contains("DEVICE"));
    assert!(matches!(
        dto.try_as_hard_drive(),
        Err(AttributeError::TypeMismatch {
            expected: "HARD_DRIVE",
            ..
        })
    ));
}

#[test]
fn try_as_rejects_unspecified_and_unknown_discriminants() {
    for kind in [0, 99, -1] {
        let dto = DtoData {
            id: None,
            r#type: kind,
            attributes: Some(prost_types::Any::default()),
        };
        assert!(matches!(
            dto.try_as_folder(),
            Err(AttributeError::TypeMismatch { actual, .. }) if actual == kind
        ));
    }
}

#[test]
fn try_as_reports_missing_attributes() {
    let dto = DtoData {
        id: Some(1),
        r#type: DataType::Device as i32,
        attributes: None,
    };
    let error = dto.try_as_device().expect_err("missing");
    assert!(matches!(error, AttributeError::Missing));
    assert_eq!(error.to_string(), "DtoData.attributes is missing");
}

#[test]
fn try_as_reports_corrupt_bytes() {
    let dto = DtoData {
        id: None,
        r#type: DataType::Folder as i32,
        // Field 3 (name), length 5, but only one byte follows.
        attributes: Some(prost_types::Any {
            type_url: FolderDto::TYPE_URL.into(),
            value: vec![0x1a, 0x05, b'a'],
        }),
    };
    let error = dto.try_as_folder().expect_err("truncated string");
    assert!(matches!(error, AttributeError::Decode(_)));
    assert!(core::error::Error::source(&error).is_some());
}

#[cfg(feature = "serde")]
mod json {
    use super::*;

    #[test]
    fn device_json_roundtrip_uses_typed_attributes() {
        let dto = DtoData::from_device(Some(7), device());

        let json = serde_json::to_value(&dto).expect("serialize");
        assert_eq!(json["id"], 7);
        assert_eq!(json["type"], DataType::Device as i32);
        assert_eq!(json["attributes"]["name"], "desk");
        assert!(json["attributes"].get("typeUrl").is_none());
        assert!(json["attributes"].get("type_url").is_none());

        let decoded: DtoData = serde_json::from_value(json).expect("deserialize");
        let device = decoded.try_as_device().expect("unpack");
        assert_eq!(device.name, "desk");
        assert!(device.is_current);
        assert_eq!(
            decoded.attributes.expect("any").type_url,
            DeviceDto::TYPE_URL
        );
    }

    #[test]
    fn device_json_string_roundtrip_is_lossless() {
        let dto = DtoData::from_device(None, device());
        let text = serde_json::to_string(&dto).expect("serialize");
        let back: DtoData = serde_json::from_str(&text).expect("deserialize");
        assert_eq!(back, dto);
        assert_eq!(back.try_as_device().expect("unpack"), device());
    }

    #[test]
    fn every_row_roundtrips_through_json() {
        let all = [
            DtoData::from_file(Some(1), file()),
            DtoData::from_folder(Some(2), folder()),
            DtoData::from_device(Some(3), device()),
            DtoData::from_hard_drive(Some(4), hard_drive()),
        ];
        for dto in all {
            let text = serde_json::to_string(&dto).expect("serialize");
            let back: DtoData = serde_json::from_str(&text).expect("deserialize");
            assert_eq!(back, dto);
        }
    }

    #[test]
    fn attributes_are_the_entity_object() {
        let json = serde_json::to_value(DtoData::from_folder(Some(2), folder())).expect("ser");
        assert_eq!(
            json,
            serde_json::json!({
                "id": 2,
                "type": 2,
                "attributes": {
                    "parent_id": 1,
                    "name": "docs",
                    "path": "/docs",
                    "created_at": "2026-01-01",
                    "updated_at": "2026-01-02"
                }
            })
        );
    }

    #[test]
    fn missing_id_and_attributes_default_to_none() {
        let dto: DtoData = serde_json::from_value(serde_json::json!({ "type": 3 })).expect("de");
        assert_eq!(dto.id, None);
        assert_eq!(dto.attributes, None);
        assert!(matches!(dto.try_as_device(), Err(AttributeError::Missing)));
    }

    #[test]
    fn missing_type_is_rejected() {
        assert!(serde_json::from_value::<DtoData>(serde_json::json!({ "id": 1 })).is_err());
    }

    #[test]
    fn unsupported_type_with_attributes_is_rejected() {
        let error = serde_json::from_value::<DtoData>(serde_json::json!({
            "type": 99,
            "attributes": { "name": "x" }
        }))
        .expect_err("unsupported");
        assert!(error.to_string().contains("unsupported DtoData.type 99"));

        let unspecified = DtoDataJson {
            id: None,
            r#type: 0,
            attributes: Some(serde_json::json!({})),
        };
        assert!(matches!(
            DtoData::try_from(unspecified),
            Err(AttributeError::UnsupportedType(0))
        ));
    }

    #[test]
    fn unsupported_type_without_attributes_is_accepted() {
        let dto: DtoData =
            serde_json::from_value(serde_json::json!({ "type": 99 })).expect("no attributes");
        assert_eq!(dto.r#type, 99);
    }

    #[test]
    fn invalid_attributes_keep_the_serde_error() {
        let json = DtoDataJson {
            id: None,
            r#type: DataType::Device as i32,
            attributes: Some(serde_json::json!({ "is_current": "yes" })),
        };
        let error = DtoData::try_from(json).expect_err("bad bool");
        match &error {
            AttributeError::Json { label, source } => {
                assert_eq!(*label, "DeviceDTO");
                assert!(source.is_data());
            }
            other => panic!("unexpected {other:?}"),
        }
        assert!(
            error
                .to_string()
                .starts_with("invalid DeviceDTO attributes")
        );
        assert!(core::error::Error::source(&error).is_some());
    }

    #[test]
    fn json_decode_packs_the_right_type_url() {
        let dto: DtoData = serde_json::from_value(serde_json::json!({
            "type": DataType::HardDrive as i32,
            "attributes": serde_json::to_value(hard_drive()).expect("ser")
        }))
        .expect("de");
        assert_eq!(
            dto.attributes.as_ref().expect("any").type_url,
            HardDriveDto::TYPE_URL
        );
        assert_eq!(dto.try_as_hard_drive().expect("unpack"), hard_drive());
    }

    #[test]
    fn unknown_or_corrupt_payload_serializes_without_attributes() {
        let corrupt = DtoData {
            id: Some(1),
            r#type: DataType::Folder as i32,
            attributes: Some(prost_types::Any {
                type_url: String::new(),
                value: vec![0xff],
            }),
        };
        let json = DtoDataJson::from(corrupt);
        assert_eq!(json.attributes, None);
        assert_eq!(json.r#type, 2);
    }
}
