use super::{App, AppError, ScanRef, extract_structured_marker_ids, read_bounded, safe_join};
use syn::spanned::Spanned;

#[derive(Debug, Clone)]
pub(super) struct RustItemScope {
    marker_line: usize,
    marker_id: String,
    pub(super) scope: &'static str,
    pub(super) identity: String,
}

fn rust_doc_markers(attrs: &[syn::Attribute]) -> Vec<(usize, String)> {
    let mut markers = Vec::new();
    for attr in attrs {
        if !attr.path().is_ident("doc") {
            continue;
        }
        let syn::Meta::NameValue(value) = &attr.meta else {
            continue;
        };
        let syn::Expr::Lit(expression) = &value.value else {
            continue;
        };
        let syn::Lit::Str(literal) = &expression.lit else {
            continue;
        };
        for id in extract_structured_marker_ids(&literal.value(), "dwv:req ") {
            markers.push((attr.span().start().line, id));
        }
    }
    markers
}

fn rust_test_scope(attrs: &[syn::Attribute]) -> bool {
    attrs.iter().any(|attr| {
        if attr.path().is_ident("test") {
            return true;
        }
        if !attr.path().is_ident("cfg") && !attr.path().is_ident("cfg_attr") {
            return false;
        }
        let syn::Meta::List(list) = &attr.meta else {
            return false;
        };
        list.tokens
            .to_string()
            .split(|character: char| !character.is_ascii_alphanumeric() && character != '_')
            .any(|part| part == "test")
    })
}

fn record_rust_item(
    attrs: &[syn::Attribute],
    kind: &str,
    name: String,
    parent: &str,
    inherited_test: bool,
    items: &mut Vec<RustItemScope>,
) -> String {
    let identity = if parent.is_empty() {
        format!("{kind}::{name}")
    } else {
        format!("{parent}::{kind}::{name}")
    };
    let scope = if inherited_test || rust_test_scope(attrs) {
        "test"
    } else {
        "production"
    };
    for (marker_line, marker_id) in rust_doc_markers(attrs) {
        items.push(RustItemScope {
            marker_line,
            marker_id,
            scope,
            identity: identity.clone(),
        });
    }
    identity
}

fn rust_path_name(path: &syn::Path) -> String {
    path.segments
        .last()
        .map(|segment| segment.ident.to_string())
        .unwrap_or_else(|| "anonymous".to_owned())
}

fn rust_type_name(ty: &syn::Type) -> String {
    match ty {
        syn::Type::Path(path) => rust_path_name(&path.path),
        syn::Type::Reference(reference) => rust_type_name(&reference.elem),
        _ => "anonymous".to_owned(),
    }
}

fn rust_impl_name(item: &syn::ItemImpl) -> String {
    let self_name = rust_type_name(&item.self_ty);
    item.trait_
        .as_ref()
        .map(|(_, path, _)| format!("{} for {self_name}", rust_path_name(path)))
        .unwrap_or(self_name)
}

fn collect_rust_items(
    items: &[syn::Item],
    parent: &str,
    inherited_test: bool,
    collected: &mut Vec<RustItemScope>,
) {
    for item in items {
        match item {
            syn::Item::Const(item) => {
                record_rust_item(
                    &item.attrs,
                    "const",
                    item.ident.to_string(),
                    parent,
                    inherited_test,
                    collected,
                );
            }
            syn::Item::Enum(item) => {
                record_rust_item(
                    &item.attrs,
                    "enum",
                    item.ident.to_string(),
                    parent,
                    inherited_test,
                    collected,
                );
            }
            syn::Item::ExternCrate(item) => {
                record_rust_item(
                    &item.attrs,
                    "extern-crate",
                    item.ident.to_string(),
                    parent,
                    inherited_test,
                    collected,
                );
            }
            syn::Item::Fn(item) => {
                record_rust_item(
                    &item.attrs,
                    "fn",
                    item.sig.ident.to_string(),
                    parent,
                    inherited_test,
                    collected,
                );
            }
            syn::Item::ForeignMod(item) => {
                let identity = record_rust_item(
                    &item.attrs,
                    "extern",
                    "block".to_owned(),
                    parent,
                    inherited_test,
                    collected,
                );
                collect_foreign_items(&item.items, &identity, inherited_test, collected);
            }
            syn::Item::Impl(item) => {
                let identity = record_rust_item(
                    &item.attrs,
                    "impl",
                    rust_impl_name(item),
                    parent,
                    inherited_test,
                    collected,
                );
                collect_impl_items(&item.items, &identity, inherited_test, collected);
            }
            syn::Item::Macro(item) => {
                record_rust_item(
                    &item.attrs,
                    "macro",
                    item.ident
                        .as_ref()
                        .map(ToString::to_string)
                        .unwrap_or_else(|| "anonymous".to_owned()),
                    parent,
                    inherited_test,
                    collected,
                );
            }
            syn::Item::Mod(item) => {
                let identity = record_rust_item(
                    &item.attrs,
                    "mod",
                    item.ident.to_string(),
                    parent,
                    inherited_test,
                    collected,
                );
                if let Some((_, nested)) = &item.content {
                    collect_rust_items(
                        nested,
                        &identity,
                        inherited_test || rust_test_scope(&item.attrs),
                        collected,
                    );
                }
            }
            syn::Item::Static(item) => {
                record_rust_item(
                    &item.attrs,
                    "static",
                    item.ident.to_string(),
                    parent,
                    inherited_test,
                    collected,
                );
            }
            syn::Item::Struct(item) => {
                record_rust_item(
                    &item.attrs,
                    "struct",
                    item.ident.to_string(),
                    parent,
                    inherited_test,
                    collected,
                );
            }
            syn::Item::Trait(item) => {
                let identity = record_rust_item(
                    &item.attrs,
                    "trait",
                    item.ident.to_string(),
                    parent,
                    inherited_test,
                    collected,
                );
                collect_trait_items(&item.items, &identity, inherited_test, collected);
            }
            syn::Item::TraitAlias(item) => {
                record_rust_item(
                    &item.attrs,
                    "trait-alias",
                    item.ident.to_string(),
                    parent,
                    inherited_test,
                    collected,
                );
            }
            syn::Item::Type(item) => {
                record_rust_item(
                    &item.attrs,
                    "type",
                    item.ident.to_string(),
                    parent,
                    inherited_test,
                    collected,
                );
            }
            syn::Item::Union(item) => {
                record_rust_item(
                    &item.attrs,
                    "union",
                    item.ident.to_string(),
                    parent,
                    inherited_test,
                    collected,
                );
            }
            syn::Item::Use(_) | syn::Item::Verbatim(_) => {}
            _ => {}
        }
    }
}

fn collect_impl_items(
    items: &[syn::ImplItem],
    parent: &str,
    inherited_test: bool,
    collected: &mut Vec<RustItemScope>,
) {
    for item in items {
        match item {
            syn::ImplItem::Const(item) => {
                record_rust_item(
                    &item.attrs,
                    "const",
                    item.ident.to_string(),
                    parent,
                    inherited_test,
                    collected,
                );
            }
            syn::ImplItem::Fn(item) => {
                record_rust_item(
                    &item.attrs,
                    "fn",
                    item.sig.ident.to_string(),
                    parent,
                    inherited_test,
                    collected,
                );
            }
            syn::ImplItem::Macro(item) => {
                record_rust_item(
                    &item.attrs,
                    "macro",
                    "anonymous".to_owned(),
                    parent,
                    inherited_test,
                    collected,
                );
            }
            syn::ImplItem::Type(item) => {
                record_rust_item(
                    &item.attrs,
                    "type",
                    item.ident.to_string(),
                    parent,
                    inherited_test,
                    collected,
                );
            }
            syn::ImplItem::Verbatim(_) => {}
            _ => {}
        }
    }
}

fn collect_trait_items(
    items: &[syn::TraitItem],
    parent: &str,
    inherited_test: bool,
    collected: &mut Vec<RustItemScope>,
) {
    for item in items {
        match item {
            syn::TraitItem::Const(item) => {
                record_rust_item(
                    &item.attrs,
                    "const",
                    item.ident.to_string(),
                    parent,
                    inherited_test,
                    collected,
                );
            }
            syn::TraitItem::Fn(item) => {
                record_rust_item(
                    &item.attrs,
                    "fn",
                    item.sig.ident.to_string(),
                    parent,
                    inherited_test,
                    collected,
                );
            }
            syn::TraitItem::Macro(item) => {
                record_rust_item(
                    &item.attrs,
                    "macro",
                    "anonymous".to_owned(),
                    parent,
                    inherited_test,
                    collected,
                );
            }
            syn::TraitItem::Type(item) => {
                record_rust_item(
                    &item.attrs,
                    "type",
                    item.ident.to_string(),
                    parent,
                    inherited_test,
                    collected,
                );
            }
            syn::TraitItem::Verbatim(_) => {}
            _ => {}
        }
    }
}

fn collect_foreign_items(
    items: &[syn::ForeignItem],
    parent: &str,
    inherited_test: bool,
    collected: &mut Vec<RustItemScope>,
) {
    for item in items {
        match item {
            syn::ForeignItem::Fn(item) => {
                record_rust_item(
                    &item.attrs,
                    "fn",
                    item.sig.ident.to_string(),
                    parent,
                    inherited_test,
                    collected,
                );
            }
            syn::ForeignItem::Static(item) => {
                record_rust_item(
                    &item.attrs,
                    "static",
                    item.ident.to_string(),
                    parent,
                    inherited_test,
                    collected,
                );
            }
            syn::ForeignItem::Type(item) => {
                record_rust_item(
                    &item.attrs,
                    "type",
                    item.ident.to_string(),
                    parent,
                    inherited_test,
                    collected,
                );
            }
            syn::ForeignItem::Macro(item) => {
                record_rust_item(
                    &item.attrs,
                    "macro",
                    "anonymous".to_owned(),
                    parent,
                    inherited_test,
                    collected,
                );
            }
            syn::ForeignItem::Verbatim(_) => {}
            _ => {}
        }
    }
}

pub(super) fn rust_marker_scope(app: &App, reference: &ScanRef) -> Result<RustItemScope, AppError> {
    let path = safe_join(&app.root, &reference.path)?;
    let text = read_bounded(&path, &reference.path, app)?;
    let syntax = syn::parse_file(&text).map_err(|error| {
        AppError::new(
            "rust_source_parse_failed",
            format!("{}: {error}", reference.path),
        )
    })?;
    let mut items = Vec::new();
    collect_rust_items(&syntax.items, "", false, &mut items);
    Ok(items
        .into_iter()
        .find(|item| item.marker_line == reference.line && item.marker_id == reference.id)
        .unwrap_or(RustItemScope {
            marker_line: reference.line,
            marker_id: reference.id.clone(),
            scope: "unscoped",
            identity: "unscoped".to_owned(),
        }))
}
