use quote::quote;
use syn::{
    visit::Visit, Attribute, File, ItemEnum, ItemFn, ItemImpl, ItemStruct, ItemTrait, Lit, Meta,
};

use crate::types::{EnumVariant, Signature};

pub fn extract_docs(attrs: &[Attribute]) -> Vec<String> {
    attrs
        .iter()
        .filter_map(|a| {
            if !a.path().is_ident("doc") {
                return None;
            }
            if let Meta::NameValue(nv) = &a.meta {
                if let syn::Expr::Lit(expr_lit) = &nv.value {
                    if let Lit::Str(s) = &expr_lit.lit {
                        return Some(s.value().trim().to_string());
                    }
                }
            }
            None
        })
        .collect()
}

struct SignatureCollector {
    signatures: Vec<Signature>,
}

impl SignatureCollector {
    fn new() -> Self {
        Self {
            signatures: Vec::new(),
        }
    }
}

impl<'ast> Visit<'ast> for SignatureCollector {
    fn visit_item_fn(&mut self, i: &'ast ItemFn) {
        let docs = extract_docs(&i.attrs);
        let sig = &i.sig;
        self.signatures.push(Signature::Fn {
            docs,
            signature: format!("fn   {}", quote! { #sig }),
        });
    }

    fn visit_item_struct(&mut self, i: &'ast ItemStruct) {
        let docs = extract_docs(&i.attrs);
        let name = i.ident.to_string();
        let generics = format!("{}", quote! { #i.generics });
        self.signatures.push(Signature::Struct {
            docs,
            name,
            generics,
        });
    }

    fn visit_item_enum(&mut self, i: &'ast ItemEnum) {
        let docs = extract_docs(&i.attrs);
        let name = i.ident.to_string();
        let generics = format!("{}", quote! { #i.generics });
        let mut variants = Vec::new();
        for variant in &i.variants {
            let vdocs = extract_docs(&variant.attrs);
            let vname = variant.ident.to_string();
            match &variant.fields {
                syn::Fields::Named(f) => {
                    let fields: Vec<String> = f
                        .named
                        .iter()
                        .map(|f| {
                            let (fname, ty) = (&f.ident, &f.ty);
                            format!("{}: {}", quote! { #fname }, quote! { #ty })
                        })
                        .collect();
                    variants.push(EnumVariant::Named {
                        name: vname,
                        docs: vdocs,
                        fields,
                    });
                }
                syn::Fields::Unnamed(f) => {
                    let types: Vec<String> = f
                        .unnamed
                        .iter()
                        .map(|f| {
                            let ty = &f.ty;
                            format!("{}", quote! { #ty })
                        })
                        .collect();
                    variants.push(EnumVariant::Tuple {
                        name: vname,
                        docs: vdocs,
                        types,
                    });
                }
                syn::Fields::Unit => {
                    variants.push(EnumVariant::Unit {
                        name: vname,
                        docs: vdocs,
                    });
                }
            }
        }
        self.signatures.push(Signature::Enum {
            docs,
            name,
            generics,
            variants,
        });
    }

    fn visit_item_trait(&mut self, i: &'ast ItemTrait) {
        let docs = extract_docs(&i.attrs);
        let name = i.ident.to_string();
        let generics = format!("{}", quote! { #i.generics });
        self.signatures.push(Signature::Trait {
            docs,
            name,
            generics,
        });
    }

    fn visit_item_impl(&mut self, i: &'ast ItemImpl) {
        let ty_str = format!("{}", quote! { #i.self_ty });
        let (trait_name, for_type) = if let Some((_, trait_, _)) = &i.trait_ {
            (Some(format!("{}", quote! { #trait_ })), ty_str)
        } else {
            (None, ty_str)
        };
        let mut associated_collector = SignatureCollector::new();
        for item in &i.items {
            match item {
                syn::ImplItem::Fn(f) => {
                    let docs = extract_docs(&f.attrs);
                    let sig = &f.sig;
                    associated_collector.signatures.push(Signature::Fn {
                        docs,
                        signature: format!("fn   {}", quote! { #sig }),
                    });
                }
                syn::ImplItem::Const(c) => {
                    associated_collector.signatures.push(Signature::Fn {
                        docs: extract_docs(&c.attrs),
                        signature: format!("const   {}: {}", c.ident, quote! { #c.ty }),
                    });
                }
                syn::ImplItem::Type(t) => {
                    associated_collector.signatures.push(Signature::Impl {
                        trait_name: None,
                        for_type: t.ident.to_string(),
                        associated: vec![],
                    });
                }
                _ => {}
            }
        }
        self.signatures.push(Signature::Impl {
            trait_name,
            for_type,
            associated: associated_collector.signatures,
        });
    }
}

pub fn analyze_source(src: &str) -> Vec<Signature> {
    let ast: File = match syn::parse_file(src) {
        Ok(ast) => ast,
        Err(_) => return Vec::new(),
    };
    let mut collector = SignatureCollector::new();
    collector.visit_file(&ast);
    collector.signatures
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_function_signature() {
        let src = r#"
pub fn greet(name: &str) -> String {
    format!("Hello, {}!", name)
}
"#;
        let sigs = analyze_source(src);
        assert_eq!(sigs.len(), 1);
        match &sigs[0] {
            Signature::Fn { docs, signature } => {
                assert!(docs.is_empty());
                assert!(signature.contains("greet"));
                assert!(signature.contains("name"));
                assert!(signature.contains("String"));
            }
            other => panic!("expected Fn, got {:?}", other),
        }
    }

    #[test]
    fn extracts_function_with_docs() {
        let src = r#"
/// Adds two numbers together.
///
/// # Panics
/// Panics on overflow.
fn add(a: i32, b: i32) -> i32 {
    a + b
}
"#;
        let sigs = analyze_source(src);
        assert_eq!(sigs.len(), 1);
        match &sigs[0] {
            Signature::Fn { docs, signature } => {
                assert!(docs.iter().any(|d| d == "Adds two numbers together."));
                assert!(docs.iter().any(|d| d == "# Panics"));
                assert!(docs.iter().any(|d| d.contains("Panics on overflow")));
                assert!(signature.contains("add"));
                assert!(signature.contains("i32"));
            }
            other => panic!("expected Fn, got {:?}", other),
        }
    }

    #[test]
    fn extracts_struct_with_generics() {
        let src = r#"
/// A key-value store backed by a Vec.
struct KeyValueStore<K, V> {
    keys: Vec<K>,
    values: Vec<V>,
}
"#;
        let sigs = analyze_source(src);
        assert_eq!(sigs.len(), 1);
        match &sigs[0] {
            Signature::Struct {
                docs,
                name,
                generics,
            } => {
                assert_eq!(docs.len(), 1);
                assert_eq!(docs[0], "A key-value store backed by a Vec.");
                assert_eq!(name, "KeyValueStore");
                assert!(generics.contains('K'));
                assert!(generics.contains('V'));
            }
            other => panic!("expected Struct, got {:?}", other),
        }
    }

    #[test]
    fn extracts_enum_with_all_variant_kinds() {
        let src = r#"
/// HTTP request methods.
enum HttpMethod {
    Get,
    Post(PostData),
    /// A custom method with headers.
    Custom { name: String, headers: Vec<u8> },
}
"#;
        let sigs = analyze_source(src);
        assert_eq!(sigs.len(), 1);
        match &sigs[0] {
            Signature::Enum {
                docs,
                name,
                variants,
                ..
            } => {
                assert_eq!(docs.len(), 1);
                assert_eq!(docs[0], "HTTP request methods.");
                assert_eq!(name, "HttpMethod");
                assert_eq!(variants.len(), 3);

                match &variants[0] {
                    EnumVariant::Unit { name, docs } => {
                        assert_eq!(name, "Get");
                        assert!(docs.is_empty());
                    }
                    other => panic!("variant 0: expected Unit, got {:?}", other),
                }

                match &variants[1] {
                    EnumVariant::Tuple { name, docs, types } => {
                        assert_eq!(name, "Post");
                        assert!(docs.is_empty());
                        assert!(types.iter().any(|t| t.contains("PostData")));
                    }
                    other => panic!("variant 1: expected Tuple, got {:?}", other),
                }

                match &variants[2] {
                    EnumVariant::Named { name, docs, fields } => {
                        assert_eq!(name, "Custom");
                        assert_eq!(docs.len(), 1);
                        assert!(fields.iter().any(|f| f.contains("name")));
                        assert!(fields.iter().any(|f| f.contains("headers")));
                    }
                    other => panic!("variant 2: expected Named, got {:?}", other),
                }
            }
            other => panic!("expected Enum, got {:?}", other),
        }
    }

    #[test]
    fn extracts_trait() {
        let src = r#"
/// A generic iterator.
trait Iterator {
    type Item;
    fn next(&mut self) -> Option<Self::Item>;
}
"#;
        let sigs = analyze_source(src);
        assert!(sigs
            .iter()
            .any(|s| matches!(s, Signature::Trait { name, .. } if name == "Iterator")));
    }

    #[test]
    fn extracts_inherent_impl_with_associated_items() {
        let src = r#"
struct Point {
    x: f64,
    y: f64,
}

impl Point {
    fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    fn distance_from_origin(&self) -> f64 {
        (self.x.powi(2) + self.y.powi(2)).sqrt()
    }
}
"#;
        let sigs = analyze_source(src);
        let impl_sig = sigs.iter().find(|s| matches!(s, Signature::Impl { .. }));
        assert!(impl_sig.is_some(), "expected an Impl signature");
        match impl_sig.unwrap() {
            Signature::Impl {
                trait_name,
                for_type,
                associated,
            } => {
                assert!(trait_name.is_none());
                assert!(for_type.contains("Point"));
                assert_eq!(associated.len(), 2);
                assert!(associated.iter().any(
                    |s| matches!(s, Signature::Fn { signature, .. } if signature.contains("new"))
                ));
                assert!(associated.iter().any(|s| matches!(s, Signature::Fn { signature, .. } if signature.contains("distance_from_origin"))));
            }
            other => panic!("expected Impl, got {:?}", other),
        }
    }

    #[test]
    fn extracts_trait_impl() {
        let src = r#"
struct MyVec(Vec<i32>);

impl std::fmt::Display for MyVec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.0)
    }
}
"#;
        let sigs = analyze_source(src);
        let impl_sig = sigs.iter().find(|s| matches!(s, Signature::Impl { .. }));
        match impl_sig.unwrap() {
            Signature::Impl {
                trait_name,
                for_type,
                ..
            } => {
                assert!(trait_name.as_ref().unwrap().contains("Display"));
                assert!(for_type.contains("MyVec"));
            }
            other => panic!("expected Impl, got {:?}", other),
        }
    }

    #[test]
    fn extracts_multiple_items() {
        let src = r#"
/// A helper struct.
struct Helper;

/// Does helper things.
fn do_help() {}
"#;
        let sigs = analyze_source(src);
        assert_eq!(sigs.len(), 2);
        assert!(sigs.iter().any(|s| matches!(s, Signature::Struct { .. })));
        assert!(sigs.iter().any(|s| matches!(s, Signature::Fn { .. })));
    }

    #[test]
    fn returns_empty_on_invalid_syntax() {
        let sigs = analyze_source("this is not valid rust {{{");
        assert!(sigs.is_empty());
    }

    #[test]
    fn extract_docs_from_source() {
        let src = r#"
/// Single line doc.
/// Second line.
fn documented() {}
"#;
        let sigs = analyze_source(src);
        match &sigs[0] {
            Signature::Fn { docs, .. } => {
                assert_eq!(docs.len(), 2);
                assert_eq!(docs[0], "Single line doc.");
                assert_eq!(docs[1], "Second line.");
            }
            other => panic!("expected Fn, got {:?}", other),
        }
    }

    #[test]
    fn doc_comments_stripped_of_whitespace() {
        let src = r#"
///   Leading spaces.
///   
/// Trailing spaces after trim.   
fn spaced() {}
"#;
        let sigs = analyze_source(src);
        match &sigs[0] {
            Signature::Fn { docs, .. } => {
                assert_eq!(docs[0], "Leading spaces.");
                assert_eq!(docs[1], "");
                assert_eq!(docs[2], "Trailing spaces after trim.");
            }
            other => panic!("expected Fn, got {:?}", other),
        }
    }
}
