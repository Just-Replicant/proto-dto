# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0](https://github.com/Just-Replicant/proto-dto/releases/tag/proto-dto-build-v0.1.0) - 2026-11-17

### Added

- *(proto-dto-build)* add descriptor-driven FRB mirror generator

### Other

- document workspace and proto-dto-build
- *(proto-dto-build)* add generate and compile fixtures

### Added

- `Mirror` builder: reads a `FileDescriptorSet` (path or bytes) and renders
  `#[frb(mirror(T))] pub struct _T { .. }` files for the listed messages,
  importing the prost types from a configurable path (`rf_proto`).
- prost's field types (`Option`, `Vec`, `HashMap`, `Box` for recursive
  fields, enums as `i32`, proto2/proto3 presence), `heck` names and keyword
  escaping (`naming`).
- Optional check file with exhaustive `let T { .. } = value` conversions
  between every prost type and its mirror.
- `write_if_changed`: outputs keep their mtime when unchanged.
- `examples/render.rs`: preview the output for a descriptor set.
