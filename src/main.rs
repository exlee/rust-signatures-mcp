use std::{env, fs};
use syn::{visit::Visit, Attribute, File, ItemEnum, ItemFn, ItemImpl, ItemStruct, ItemTrait, Lit, Meta};
use quote::quote;

fn extract_docs(attrs: &[Attribute]) -> Vec<String> {
    attrs.iter().filter_map(|a| {
        if !a.path().is_ident("doc") { return None; }
        if let Meta::NameValue(nv) = &a.meta {
            if let syn::Expr::Lit(expr_lit) = &nv.value {
                if let Lit::Str(s) = &expr_lit.lit {
                    return Some(s.value().trim().to_string());
                }
            }
        }
        None
    }).collect()
}

fn print_docs(docs: &[String]) {
    for line in docs {
        println!("/// {}", line);
    }
}

struct SignatureVisitor;

impl<'ast> Visit<'ast> for SignatureVisitor {

    fn visit_item_struct(&mut self, i: &'ast ItemStruct) {
        let docs = extract_docs(&i.attrs);
        print_docs(&docs);
        let (name, generics) = (&i.ident, &i.generics);
        println!("struct {}{}", name, quote! { #generics });
    }

    fn visit_item_enum(&mut self, i: &'ast ItemEnum) {
        let docs = extract_docs(&i.attrs);
        print_docs(&docs);
        let (name, generics) = (&i.ident, &i.generics);
        println!("enum  {}{} {{", name, quote! { #generics });
        for variant in &i.variants {
            let vdocs = extract_docs(&variant.attrs);
            for line in &vdocs {
                println!("  /// {}", line);
            }
            let vname = &variant.ident;
            match &variant.fields {
                syn::Fields::Named(f) => {
                    let fields: Vec<_> = f.named.iter().map(|f| {
                        let fname = &f.ident;
                        let ty = &f.ty;
                        quote! { #fname: #ty }
                    }).collect();
                    println!("  {} {{ {} }}", vname, quote! { #(#fields),* });
                }
                syn::Fields::Unnamed(f) => {
                    let types: Vec<_> = f.unnamed.iter().map(|f| { let ty = &f.ty; quote! { #ty } }).collect();
                    println!("  {}({})", vname, quote! { #(#types),* });
                }
                syn::Fields::Unit => {
                    println!("  {}", vname);
                }
            }
        }
        println!("}}");
    }
    fn visit_item_trait(&mut self, i: &'ast ItemTrait) {
        let docs = extract_docs(&i.attrs);
        print_docs(&docs);
        let (name, generics) = (&i.ident, &i.generics);
        println!("trait  {}{}", name, quote! { #generics });
    }

    fn visit_item_fn(&mut self, i: &'ast ItemFn) {
        let docs = extract_docs(&i.attrs);
        print_docs(&docs);
        let sig = &i.sig;
        println!("fn   {}", quote! { #sig });
    }

    fn visit_item_impl(&mut self, i: &'ast ItemImpl) {
        if let Some((_, trait_, _)) = &i.trait_ {
            let ty = &i.self_ty;
            println!("impl   {} for {}", quote! { #trait_ }, quote! { #ty });
        } else {
            let ty = &i.self_ty;
            println!("impl   {}", quote! { #ty });
        }
        syn::visit::visit_item_impl(self, i);

    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: list-signatures <file.rs>");
        std::process::exit(1);
    }

    for path in &args[1..] {
        let src = fs::read_to_string(path).expect("failed to read file");
        let ast: File = syn::parse_file(&src).expect("failed to parse file");
        println!("=== {} ===", path);
        SignatureVisitor.visit_file(&ast);
        println!();
    }
}


