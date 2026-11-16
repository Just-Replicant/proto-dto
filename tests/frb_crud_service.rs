// SPDX-License-Identifier: AGPL-3.0-or-later

//! `frb_crud_service!` against stand-ins for the paths it names.
//!
//! The macro only pastes tokens: `RF`, `rf_entity::init`, `rf_cgp::prelude`,
//! `log` and `flutter_rust_bridge::frb` resolve at the call site. Here they
//! resolve to the small modules below, which proves the expansion does not
//! need the real crates.

#![allow(missing_docs, unreachable_pub, async_fn_in_trait)]
// Stand-ins mimic the real crates, not library-quality API.
#![allow(
    clippy::unused_async,
    clippy::unused_async_trait_impl,
    clippy::missing_errors_doc,
    clippy::must_use_candidate
)]

use std::cell::RefCell;
use std::future::Future;
use std::pin::pin;
use std::sync::Mutex;
use std::task::{Context, Poll, Waker};

/// Drives a future that never actually waits.
fn block_on<F: Future>(future: F) -> F::Output {
    let mut future = pin!(future);
    let mut context = Context::from_waker(Waker::noop());
    loop {
        if let Poll::Ready(output) = future.as_mut().poll(&mut context) {
            return output;
        }
    }
}

thread_local! {
    static CALLS: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
}

fn record(call: impl Into<String>) {
    CALLS.with(|calls| calls.borrow_mut().push(call.into()));
}

fn take_calls() -> Vec<String> {
    CALLS.with(|calls| std::mem::take(&mut *calls.borrow_mut()))
}

#[derive(Debug, Clone, PartialEq)]
pub struct DtoData(pub i32);

#[derive(Debug, PartialEq)]
pub struct DbError(String);

impl From<String> for DbError {
    fn from(error: String) -> Self {
        Self(error)
    }
}

pub struct Config(&'static str);

pub struct RF;

impl RF {
    pub fn load() -> Result<Config, String> {
        record("RF::load");
        Ok(Config("sqlite::memory:"))
    }
}

pub mod rf_entity {
    pub struct Connection(pub String);

    pub async fn init(config: &super::Config) -> Result<Connection, String> {
        super::record("rf_entity::init");
        Ok(Connection(config.0.to_owned()))
    }
}

pub mod rf_cgp {
    use super::{DtoData, record};
    use crate::rf_entity::Connection;
    use std::sync::Mutex;

    pub mod prelude {
        use crate::DtoData;

        pub trait CanCreateByDto {
            async fn create_by_dto(&self, dto: DtoData) -> Result<(DtoData, bool), String>;
        }

        pub trait CanUpdateDtoById {
            async fn update_dto_by_id(&self, dto: DtoData) -> Result<Option<DtoData>, String>;
        }

        pub trait CanListDto {
            async fn list_dto(
                &self,
                page: Option<u64>,
                size: Option<u64>,
            ) -> Result<Vec<DtoData>, String>;
        }
    }

    pub struct FolderApp {
        pub connection: Connection,
        pub rows: Mutex<Vec<DtoData>>,
    }

    impl FolderApp {
        pub fn new(connection: Connection) -> Self {
            record("FolderApp::new");
            Self {
                connection,
                rows: Mutex::new(Vec::new()),
            }
        }
    }

    impl prelude::CanCreateByDto for FolderApp {
        async fn create_by_dto(&self, dto: DtoData) -> Result<(DtoData, bool), String> {
            if dto.0 < 0 {
                return Err(format!("negative id {}", dto.0));
            }
            self.rows.lock().unwrap().push(dto.clone());
            Ok((dto, true))
        }
    }

    impl prelude::CanUpdateDtoById for FolderApp {
        async fn update_dto_by_id(&self, dto: DtoData) -> Result<Option<DtoData>, String> {
            let rows = self.rows.lock().unwrap();
            Ok(rows.iter().find(|row| row.0 == dto.0).cloned())
        }
    }

    impl prelude::CanListDto for FolderApp {
        async fn list_dto(
            &self,
            page: Option<u64>,
            size: Option<u64>,
        ) -> Result<Vec<DtoData>, String> {
            let rows = self.rows.lock().unwrap();
            let size = usize::try_from(size.unwrap_or(10)).unwrap();
            let page = usize::try_from(page.unwrap_or(0)).unwrap();
            Ok(rows.iter().skip(page * size).take(size).cloned().collect())
        }
    }

    pub type DeviceApp = FolderApp;
}

proto_dto::frb_crud_service! {
    wrapper: FolderServiceWrapper,
    app: rf_cgp::FolderApp,
    error: crate::DbError,
    label: "folder",
    dto: crate::DtoData,
}

// The four-key form defaults `dto` to `crate::api::shared::dto::DtoData`, the
// bridge mirror path in the Flutter crate.
pub mod api {
    pub mod shared {
        pub mod dto {
            pub use crate::DtoData;
        }
    }

    pub mod device {
        use crate::{RF, rf_cgp, rf_entity};

        proto_dto::frb_crud_service! {
            wrapper: DeviceServiceWrapper,
            app: rf_cgp::DeviceApp,
            error: crate::DbError,
            label: "device",
        }
    }
}

static SERIAL: Mutex<()> = Mutex::new(());

#[test]
fn new_loads_config_then_inits_db_then_builds_app() {
    let _guard = SERIAL.lock().unwrap();
    take_calls();
    let service = block_on(FolderServiceWrapper::new()).expect("new");
    assert_eq!(
        take_calls(),
        ["RF::load", "rf_entity::init", "FolderApp::new"]
    );
    assert_eq!(service.app.connection.0, "sqlite::memory:");
}

#[test]
fn crud_methods_forward_to_the_app() {
    let _guard = SERIAL.lock().unwrap();
    let service = block_on(FolderServiceWrapper::new()).expect("new");

    assert_eq!(block_on(service.create_by_dto(DtoData(1))), Ok(DtoData(1)));
    assert_eq!(block_on(service.create_by_dto(DtoData(2))), Ok(DtoData(2)));
    assert_eq!(
        block_on(service.update_dto_by_id(DtoData(2))),
        Ok(Some(DtoData(2)))
    );
    assert_eq!(block_on(service.update_dto_by_id(DtoData(7))), Ok(None));
    assert_eq!(
        block_on(service.get_paginate_dto(Some(1), Some(1))),
        Ok(vec![DtoData(2)])
    );
}

#[test]
fn app_errors_convert_into_the_service_error() {
    let _guard = SERIAL.lock().unwrap();
    let service = block_on(FolderServiceWrapper::new()).expect("new");
    assert_eq!(
        block_on(service.create_by_dto(DtoData(-1))),
        Err(DbError("negative id -1".into()))
    );
}

#[test]
fn default_dto_path_is_the_bridge_mirror() {
    let _guard = SERIAL.lock().unwrap();
    let service = block_on(api::device::DeviceServiceWrapper::new()).expect("new");
    let created: api::shared::dto::DtoData =
        block_on(service.create_by_dto(DtoData(3))).expect("create");
    assert_eq!(created, DtoData(3));
}

#[test]
fn wrapper_is_send_and_sync_for_the_bridge() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<FolderServiceWrapper>();
    assert_send_sync::<api::device::DeviceServiceWrapper>();
}
