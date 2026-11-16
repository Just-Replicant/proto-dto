// SPDX-License-Identifier: AGPL-3.0-or-later

//! Test-only stand-in for `flutter_rust_bridge::frb`.
//!
//! The real attribute rewrites itself into a doc marker that
//! `flutter_rust_bridge_codegen` reads after `cargo expand`. The tests in
//! `proto-dto` only need the attribute to resolve, so this one passes the item
//! through unchanged and records the arguments in a doc attribute.

use proc_macro::TokenStream;

/// No-op `#[frb(...)]` that keeps the item and notes the arguments.
#[proc_macro_attribute]
pub fn frb(attribute: TokenStream, item: TokenStream) -> TokenStream {
    let marker = format!(
        "#[doc = \"frb_shim: {}\"]",
        attribute.to_string().replace('"', "\\\"")
    );
    let mut out: TokenStream = marker.parse().expect("valid doc attribute");
    out.extend(item);
    out
}
