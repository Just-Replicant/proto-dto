// SPDX-License-Identifier: AGPL-3.0-or-later

//! Rendering of the mirror files and the exhaustiveness check.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

use prost_types::FieldDescriptorProto;
use prost_types::field_descriptor_proto::{Label, Type};

use crate::descriptor::{Index, MessageInfo, Syntax};
use crate::{Error, Mirror, naming};

/// A mirrored message, resolved against the descriptor set.
struct Target<'i, 'a> {
    info: &'i MessageInfo<'a>,
    /// prost's type name, e.g. `FolderDto`.
    ident: String,
    /// Path below the import root, e.g. `FolderDto` or `outer::Inner`.
    relative_path: String,
    freezed: bool,
    /// Index of the mirror file the message is rendered into.
    file: usize,
}

/// One field of a mirrored message.
struct Field {
    name: String,
    /// Type as written in the mirror file (mirrored types by name).
    mirror_ty: String,
    /// Type as written in the check file (mirrored types by full path).
    check_ty: String,
    docs: Option<String>,
}

/// The rendered contents, in configuration order; the check file comes last.
pub(crate) struct Output {
    pub(crate) mirrors: Vec<String>,
    pub(crate) check: Option<String>,
}

pub(crate) fn render(config: &Mirror, index: &Index<'_>) -> Result<Output, Error> {
    let targets = resolve(config, index)?;

    let mut fields: BTreeMap<&str, Vec<Field>> = BTreeMap::new();
    let mut references: Vec<BTreeSet<String>> = vec![BTreeSet::new(); config.files.len()];
    for target in targets.values() {
        let rendered = render_fields(
            config,
            index,
            &targets,
            target,
            &mut references[target.file],
        )?;
        fields.insert(&target.info.full_name, rendered);
    }

    let mut mirrors = Vec::with_capacity(config.files.len());
    for (file_index, file) in config.files.iter().enumerate() {
        let mut out = mirror_header(config);
        let own: Vec<&Target<'_, '_>> = file
            .messages
            .iter()
            .map(|message| &targets[&qualify(config, &message.name)])
            .collect();
        write_uses(&mut out, "pub use", config, &own);
        let foreign: Vec<&Target<'_, '_>> = references[file_index]
            .iter()
            .map(|name| &targets[name])
            .filter(|target| target.file != file_index)
            .collect();
        write_uses(&mut out, "use", config, &foreign);
        for target in own {
            render_mirror(&mut out, target, &fields[target.info.full_name.as_str()]);
        }
        mirrors.push(out);
    }

    let check = config.check_file.as_ref().map(|_| {
        let mut out = check_header(config);
        for file in &config.files {
            for message in &file.messages {
                let target = &targets[&qualify(config, &message.name)];
                render_check(
                    &mut out,
                    config,
                    target,
                    &fields[target.info.full_name.as_str()],
                );
            }
        }
        out
    });

    Ok(Output { mirrors, check })
}

/// `.package.Name` for a message name given relative to the package.
pub(crate) fn qualify(config: &Mirror, name: &str) -> String {
    if name.starts_with('.') {
        name.to_owned()
    } else if config.package.is_empty() {
        format!(".{name}")
    } else {
        format!(".{}.{name}", config.package)
    }
}

fn resolve<'i, 'a>(
    config: &Mirror,
    index: &'i Index<'a>,
) -> Result<BTreeMap<String, Target<'i, 'a>>, Error> {
    let mut targets = BTreeMap::new();
    let mut idents = BTreeMap::new();
    for (file_index, file) in config.files.iter().enumerate() {
        for message in &file.messages {
            let full_name = qualify(config, &message.name);
            if targets.contains_key(&full_name) {
                return Err(Error::Config(format!(
                    "{full_name} is listed more than once"
                )));
            }
            let info = index
                .get(&full_name)
                .ok_or_else(|| Error::MessageNotFound {
                    message: full_name.clone(),
                })?;
            if info.is_map_entry() {
                return Err(Error::Unsupported {
                    message: full_name,
                    field: String::new(),
                    what: "mirroring a map entry",
                });
            }
            if info.syntax == Syntax::Editions {
                return Err(Error::Unsupported {
                    message: full_name,
                    field: String::new(),
                    what: "an edition-based .proto file",
                });
            }
            let ident = naming::type_name(info.descriptor.name());
            if let Some(previous) = idents.insert(ident.clone(), full_name.clone()) {
                return Err(Error::Config(format!(
                    "{previous} and {full_name} both map to the Rust name {ident}"
                )));
            }
            let relative_path = info
                .modules
                .iter()
                .map(String::as_str)
                .chain([ident.as_str()])
                .collect::<Vec<_>>()
                .join("::");
            let target = Target {
                info,
                ident,
                relative_path,
                freezed: message.freezed,
                file: file_index,
            };
            targets.insert(full_name, target);
        }
    }
    Ok(targets)
}

fn render_fields(
    config: &Mirror,
    index: &Index<'_>,
    targets: &BTreeMap<String, Target<'_, '_>>,
    target: &Target<'_, '_>,
    referenced: &mut BTreeSet<String>,
) -> Result<Vec<Field>, Error> {
    let message = target.info;
    message
        .descriptor
        .field
        .iter()
        .zip(&message.field_docs)
        .map(|(field, docs)| {
            let unsupported = |what| Error::Unsupported {
                message: message.display_name().to_owned(),
                field: field.name().to_owned(),
                what,
            };
            if field.oneof_index.is_some() && !field.proto3_optional() {
                return Err(unsupported("oneof"));
            }
            let resolver = TypeResolver {
                config,
                index,
                targets,
                owner: message,
            };
            let ty = resolver.field_type(field)?;
            referenced.extend(ty.referenced);
            Ok(Field {
                name: naming::field_name(field.name()),
                mirror_ty: ty.mirror,
                check_ty: ty.check,
                docs: docs.clone(),
            })
        })
        .collect()
}

struct TypeResolver<'r, 'i, 'a> {
    config: &'r Mirror,
    index: &'r Index<'a>,
    targets: &'r BTreeMap<String, Target<'i, 'a>>,
    owner: &'r MessageInfo<'a>,
}

/// A field type rendered for both output files.
struct FieldType {
    mirror: String,
    check: String,
    /// Mirrored message the type mentions, if any.
    referenced: Option<String>,
}

impl TypeResolver<'_, '_, '_> {
    /// The type prost generates for `field`.
    fn field_type(&self, field: &FieldDescriptorProto) -> Result<FieldType, Error> {
        if field.r#type() == Type::Message {
            if let Some(entry) = self.index.get(field.type_name()) {
                if entry.is_map_entry() {
                    return self.map_type(field, entry);
                }
            }
        }
        let element = self.element_type(field)?;
        let wrap = |ty: String| match field.label() {
            Label::Repeated => format!("Vec<{ty}>"),
            _ if self.has_presence(field) => format!("Option<{ty}>"),
            _ => ty,
        };
        Ok(FieldType {
            mirror: wrap(element.mirror),
            check: wrap(element.check),
            referenced: element.referenced,
        })
    }

    fn has_presence(&self, field: &FieldDescriptorProto) -> bool {
        field.proto3_optional()
            || field.r#type() == Type::Message
            || (self.owner.syntax == Syntax::Proto2 && field.label() == Label::Optional)
    }

    /// The type of one value of `field`, before `Option`/`Vec`.
    fn element_type(&self, field: &FieldDescriptorProto) -> Result<FieldType, Error> {
        let scalar = match field.r#type() {
            Type::Double => "f64",
            Type::Float => "f32",
            Type::Int64 | Type::Sint64 | Type::Sfixed64 => "i64",
            Type::Uint64 | Type::Fixed64 => "u64",
            // prost stores enums as their `i32` value.
            Type::Int32 | Type::Sint32 | Type::Sfixed32 | Type::Enum => "i32",
            Type::Uint32 | Type::Fixed32 => "u32",
            Type::Bool => "bool",
            Type::String => "String",
            Type::Bytes => "Vec<u8>",
            Type::Group => {
                return Err(Error::Unsupported {
                    message: self.owner.display_name().to_owned(),
                    field: field.name().to_owned(),
                    what: "group",
                });
            }
            Type::Message => return self.message_type(field),
        };
        Ok(FieldType {
            mirror: scalar.to_owned(),
            check: scalar.to_owned(),
            referenced: None,
        })
    }

    fn message_type(&self, field: &FieldDescriptorProto) -> Result<FieldType, Error> {
        let target = self
            .targets
            .get(field.type_name())
            .ok_or_else(|| Error::NotMirrored {
                message: self.owner.display_name().to_owned(),
                field: field.name().to_owned(),
                type_name: field.type_name().to_owned(),
            })?;
        let check = format!("{}::{}", self.config.check_import, target.relative_path);
        // Like prost: a singular field whose type (transitively) contains
        // the owner is boxed, or the struct would have infinite size.
        let boxed = field.label() != Label::Repeated
            && self.index.reaches(field.type_name(), &self.owner.full_name);
        let (mirror, check) = if boxed {
            (format!("Box<{}>", target.ident), format!("Box<{check}>"))
        } else {
            (target.ident.clone(), check)
        };
        Ok(FieldType {
            mirror,
            check,
            referenced: Some(target.info.full_name.clone()),
        })
    }

    fn map_type(
        &self,
        field: &FieldDescriptorProto,
        entry: &MessageInfo<'_>,
    ) -> Result<FieldType, Error> {
        let part = |number: i32| {
            entry
                .descriptor
                .field
                .iter()
                .find(|part| part.number() == number)
                .ok_or_else(|| Error::Unsupported {
                    message: self.owner.display_name().to_owned(),
                    field: field.name().to_owned(),
                    what: "a map entry without key/value fields",
                })
        };
        let key = self.element_type(part(1)?)?;
        let value = self.element_type(part(2)?)?;
        Ok(FieldType {
            mirror: format!(
                "::std::collections::HashMap<{}, {}>",
                key.mirror, value.mirror
            ),
            check: format!(
                "::std::collections::HashMap<{}, {}>",
                key.check, value.check
            ),
            referenced: value.referenced,
        })
    }
}

fn mirror_header(config: &Mirror) -> String {
    format!(
        "\
// @generated by {generated_by} from the protobuf descriptor set.
// Do not edit: change the .proto file and rebuild.
//
// flutter_rust_bridge mirrors of `{import}` types, so the bridge can emit
// Dart classes for them. Rendered by proto-dto-build.

use flutter_rust_bridge::frb;

",
        generated_by = config.generated_by,
        import = config.import,
    )
}

fn check_header(config: &Mirror) -> String {
    format!(
        "\
// @generated by {generated_by} (proto-dto-build). Exhaustive conversions
// between each prost type and a copy of its mirror: adding, removing or
// retyping a field without regenerating the mirrors is a compile error.
",
        generated_by = config.generated_by,
    )
}

/// `pub use import::{A, B};`, grouped by module.
fn write_uses(out: &mut String, keyword: &str, config: &Mirror, targets: &[&Target<'_, '_>]) {
    let mut groups: BTreeMap<String, Vec<&str>> = BTreeMap::new();
    for target in targets {
        let module = std::iter::once(config.import.as_str())
            .chain(target.info.modules.iter().map(String::as_str))
            .collect::<Vec<_>>()
            .join("::");
        groups.entry(module).or_default().push(&target.ident);
    }
    for (module, mut idents) in groups {
        idents.sort_unstable();
        idents.dedup();
        if let [single] = idents.as_slice() {
            let _ = writeln!(out, "{keyword} {module}::{single};");
        } else {
            let _ = writeln!(out, "{keyword} {module}::{{{}}};", idents.join(", "));
        }
    }
}

fn render_mirror(out: &mut String, target: &Target<'_, '_>, fields: &[Field]) {
    let ident = &target.ident;
    let mirror = mirror_ident(ident);
    let _ = writeln!(out);
    let _ = writeln!(
        out,
        "/// Mirror of [`{ident}`] (`{}`).",
        target.info.display_name()
    );
    let _ = writeln!(out, "///");
    if let Some(docs) = &target.info.docs {
        write_docs(out, "", docs);
        let _ = writeln!(out, "///");
    }
    let _ = writeln!(out, "/// Source: `{}`.", target.info.file);
    if target.freezed {
        let _ = writeln!(
            out,
            "#[frb(mirror({ident}), dart_metadata = (\"freezed\"))]"
        );
    } else {
        let _ = writeln!(out, "#[frb(mirror({ident}))]");
    }
    if fields.is_empty() {
        let _ = writeln!(out, "pub struct {mirror} {{}}");
        return;
    }
    let _ = writeln!(out, "pub struct {mirror} {{");
    for field in fields {
        if let Some(docs) = &field.docs {
            write_docs(out, "    ", docs);
        }
        let _ = writeln!(out, "    pub {}: {},", field.name, field.mirror_ty);
    }
    let _ = writeln!(out, "}}");
}

/// The exhaustive `let Type { fields } = value` conversions for one message.
fn render_check(out: &mut String, config: &Mirror, target: &Target<'_, '_>, fields: &[Field]) {
    let source = format!("{}::{}", config.check_import, target.relative_path);
    let mirror = mirror_ident(&target.ident);
    let names = fields
        .iter()
        .map(|field| field.name.as_str())
        .collect::<Vec<_>>()
        .join(", ");
    let pattern = if names.is_empty() {
        String::from("{}")
    } else {
        format!("{{ {names} }}")
    };

    let _ = writeln!(out);
    let _ = writeln!(
        out,
        "/// Copy of the `{}` mirror.",
        target.info.display_name()
    );
    let _ = writeln!(out, "#[allow(dead_code, non_camel_case_types)]");
    if fields.is_empty() {
        let _ = writeln!(out, "struct {mirror} {{}}");
    } else {
        let _ = writeln!(out, "struct {mirror} {{");
        for field in fields {
            let _ = writeln!(out, "    {}: {},", field.name, field.check_ty);
        }
        let _ = writeln!(out, "}}");
    }
    for (from, to) in [(source.as_str(), mirror.as_str()), (&mirror, &source)] {
        let _ = writeln!(out);
        let _ = writeln!(out, "impl ::core::convert::From<{from}> for {to} {{");
        let _ = writeln!(out, "    fn from(source: {from}) -> Self {{");
        let _ = writeln!(out, "        let {from} {pattern} = source;");
        let _ = writeln!(out, "        Self {pattern}");
        let _ = writeln!(out, "    }}");
        let _ = writeln!(out, "}}");
    }
}

/// `_FolderDto`: the struct `flutter_rust_bridge` reads `#[frb(mirror)]` from.
fn mirror_ident(ident: &str) -> String {
    format!("_{}", naming::unraw(ident))
}

fn write_docs(out: &mut String, indent: &str, docs: &str) {
    for line in docs.lines() {
        let line = line.strip_prefix(' ').unwrap_or(line).trim_end();
        if line.is_empty() {
            let _ = writeln!(out, "{indent}///");
        } else {
            let _ = writeln!(out, "{indent}/// {line}");
        }
    }
}
