# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0]

### Added

- `envelope!`: `from_<variant>` / `try_as_<variant>` inherent methods on a
  `google.protobuf.Any` envelope, a `TYPE_URL` constant per payload message,
  and (feature `serde`) `From`/`TryFrom` conversions with `DtoDataJson`. All
  dispatch is a single `match` on the protobuf discriminant.
- `frb_crud_service!`: `flutter_rust_bridge` opaque CRUD wrapper with `new`,
  `create_by_dto`, `update_dto_by_id` and `get_paginate_dto`.
- `AttributeError` stack enum and `DtoDataJson` (feature `serde`).
