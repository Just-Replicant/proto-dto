// SPDX-License-Identifier: AGPL-3.0-or-later

//! Rust identifiers for protobuf names, with the exact rules `prost-build`
//! applies, so a mirror names the same fields and types prost generated.

use heck::{ToSnakeCase as _, ToUpperCamelCase as _};

/// Escapes `ident` the way prost does when it is not a valid Rust identifier.
///
/// * Strict and reserved keywords of every edition up to 2024 become raw
///   identifiers: `type` → `r#type`.
/// * Keywords that cannot be raw identifiers get a trailing underscore:
///   `self` → `self_`.
/// * A leading digit gets a leading underscore: `1st` → `_1st`.
#[must_use]
pub fn sanitize_identifier(ident: &str) -> String {
    match ident {
        // 2015 strict keywords.
        "as" | "break" | "const" | "continue" | "else" | "enum" | "false" | "fn" | "for" | "if"
        | "impl" | "in" | "let" | "loop" | "match" | "mod" | "move" | "mut" | "pub" | "ref"
        | "return" | "static" | "struct" | "trait" | "true" | "type" | "unsafe" | "use"
        | "where" | "while"
        // 2018 strict keywords.
        | "dyn"
        // 2015 reserved keywords.
        | "abstract" | "become" | "box" | "do" | "final" | "macro" | "override" | "priv"
        | "typeof" | "unsized" | "virtual" | "yield"
        // 2018 reserved keywords.
        | "async" | "await" | "try"
        // 2024 reserved keywords.
        | "gen" => format!("r#{ident}"),
        // Not allowed as raw identifiers.
        "_" | "super" | "self" | "Self" | "extern" | "crate" => format!("{ident}_"),
        _ if ident.starts_with(|c: char| c.is_ascii_digit()) => format!("_{ident}"),
        _ => ident.to_owned(),
    }
}

/// prost's field (and module) name: `snake_case`, then [`sanitize_identifier`].
///
/// ```
/// assert_eq!(proto_dto_build::naming::field_name("parentId"), "parent_id");
/// assert_eq!(proto_dto_build::naming::field_name("type"), "r#type");
/// ```
#[must_use]
pub fn field_name(proto: &str) -> String {
    sanitize_identifier(&proto.to_snake_case())
}

/// prost's type name: `UpperCamelCase`, then [`sanitize_identifier`].
///
/// ```
/// assert_eq!(proto_dto_build::naming::type_name("FolderDTO"), "FolderDto");
/// assert_eq!(proto_dto_build::naming::type_name("hard_drive"), "HardDrive");
/// ```
#[must_use]
pub fn type_name(proto: &str) -> String {
    sanitize_identifier(&proto.to_upper_camel_case())
}

/// Strips a raw-identifier prefix, for text that is not an identifier
/// (`_r#type` is not valid; the mirror of `Type` is `_Type`).
pub(crate) fn unraw(ident: &str) -> &str {
    ident.strip_prefix("r#").unwrap_or(ident)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keywords_become_raw_identifiers() {
        for keyword in ["type", "match", "async", "gen", "try", "dyn", "yield"] {
            assert_eq!(field_name(keyword), format!("r#{keyword}"));
        }
    }

    #[test]
    fn non_raw_keywords_get_an_underscore() {
        assert_eq!(field_name("self"), "self_");
        assert_eq!(field_name("super"), "super_");
        assert_eq!(field_name("crate"), "crate_");
        assert_eq!(type_name("Self"), "Self_");
    }

    #[test]
    fn leading_digit_gets_an_underscore() {
        assert_eq!(sanitize_identifier("1st"), "_1st");
    }

    #[test]
    fn cases_match_prost() {
        assert_eq!(field_name("createdAt"), "created_at");
        assert_eq!(field_name("HTTPStatus"), "http_status");
        assert_eq!(type_name("DTOData"), "DtoData");
        assert_eq!(type_name("HardDriveDTO"), "HardDriveDto");
        assert_eq!(type_name("any"), "Any");
        assert_eq!(unraw("r#type"), "type");
        assert_eq!(unraw("name"), "name");
    }
}
