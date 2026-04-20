use serde::Serialize;

#[derive(Debug, Serialize, Clone)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Signature {
    Fn { docs: Vec<String>, signature: String },
    Struct { docs: Vec<String>, name: String, generics: String },
    Enum {
        docs: Vec<String>,
        name: String,
        generics: String,
        variants: Vec<EnumVariant>,
    },
    Trait { docs: Vec<String>, name: String, generics: String },
    Impl { trait_name: Option<String>, for_type: String, associated: Vec<Signature> },
}

#[derive(Debug, Serialize, Clone)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum EnumVariant {
    Named { name: String, docs: Vec<String>, fields: Vec<String> },
    Tuple { name: String, docs: Vec<String>, types: Vec<String> },
    Unit { name: String, docs: Vec<String> },
}

#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AnalyzeResult {
    Success { files: Vec<FileResult> },
    Error { message: String },
}

#[derive(Debug, Serialize)]
pub struct FileResult {
    pub path: String,
    pub signatures: Vec<Signature>,
}

#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SearchResult {
    Success { matches: Vec<MatchResult>, total_matched: usize },
    Error { message: String },
}

#[derive(Debug, Serialize)]
pub struct MatchResult {
    pub file: String,
    pub line: String,
}

pub fn render_signature(sig: &Signature) -> String {
    match sig {
        Signature::Fn { docs, signature } => {
            let mut out = String::new();
            for d in docs { out.push_str(&format!("/// {}\n", d)); }
            out.push_str(signature);
            out
        }
        Signature::Struct { docs, name, generics } => {
            let mut out = String::new();
            for d in docs { out.push_str(&format!("/// {}\n", d)); }
            out.push_str(&format!("struct {}{}", name, generics));
            out
        }
        Signature::Enum { docs, name, generics, variants } => {
            let mut out = String::new();
            for d in docs { out.push_str(&format!("/// {}\n", d)); }
            out.push_str(&format!("enum  {}{} {{\n", name, generics));
            for v in variants {
                match v {
                    EnumVariant::Named { name, docs, fields } => {
                        for d in docs { out.push_str(&format!("  /// {}\n", d)); }
                        out.push_str(&format!("  {} {{ {} }}\n", name, fields.join(", ")));
                    }
                    EnumVariant::Tuple { name, docs, types } => {
                        for d in docs { out.push_str(&format!("  /// {}\n", d)); }
                        out.push_str(&format!("  {}({})\n", name, types.join(", ")));
                    }
                    EnumVariant::Unit { name, docs } => {
                        for d in docs { out.push_str(&format!("  /// {}\n", d)); }
                        out.push_str(&format!("  {}\n", name));
                    }
                }
            }
            out.push_str("}");
            out
        }
        Signature::Trait { docs, name, generics } => {
            let mut out = String::new();
            for d in docs { out.push_str(&format!("/// {}\n", d)); }
            out.push_str(&format!("trait  {}{}", name, generics));
            out
        }
        Signature::Impl { trait_name, for_type, associated } => {
            let mut out = String::new();
            if let Some(tn) = trait_name {
                out.push_str(&format!("impl   {} for {}", tn, for_type));
            } else {
                out.push_str(&format!("impl   {}", for_type));
            }
            for sig in associated {
                out.push_str(&format!("\n  {}", render_signature(sig).replace('\n', "\n  ")));
            }
            out
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_fn_includes_docs_and_signature() {
        let sig = Signature::Fn {
            docs: vec!["A doc comment.".into()],
            signature: "fn   foo(x: i32)".into(),
        };
        let rendered = render_signature(&sig);
        assert!(rendered.starts_with("/// A doc comment.\n"));
        assert!(rendered.contains("fn   foo(x: i32)"));
    }

    #[test]
    fn render_enum_includes_all_variants() {
        let sig = Signature::Enum {
            docs: vec![],
            name: "Color".into(),
            generics: String::new(),
            variants: vec![
                EnumVariant::Unit { name: "Red".into(), docs: vec![] },
                EnumVariant::Tuple { name: "Rgb".into(), docs: vec![], types: vec!["u8".into(), "u8".into(), "u8".into()] },
                EnumVariant::Named { name: "Custom".into(), docs: vec!["hex value".into()], fields: vec!["hex: String".into()] },
            ],
        };
        let rendered = render_signature(&sig);
        assert!(rendered.contains("enum  Color {"));
        assert!(rendered.contains("Red"));
        assert!(rendered.contains("Rgb(u8, u8, u8)"));
        assert!(rendered.contains("Custom { hex: String }"));
        assert!(rendered.contains("/// hex value"));
        assert!(rendered.ends_with('}'));
    }

    #[test]
    fn analyze_result_serializes_success() {
        let result = AnalyzeResult::Success {
            files: vec![FileResult {
                path: "foo.rs".into(),
                signatures: vec![Signature::Fn {
                    docs: vec!["A test fn.".into()],
                    signature: "fn   bar() -> bool".into(),
                }],
            }],
        };
        let json = serde_json::to_string(&result).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["type"], "success");
        assert_eq!(parsed["files"][0]["path"], "foo.rs");
        assert_eq!(parsed["files"][0]["signatures"][0]["kind"], "fn");
        assert_eq!(parsed["files"][0]["signatures"][0]["signature"], "fn   bar() -> bool");
    }

    #[test]
    fn analyze_result_serializes_error() {
        let result = AnalyzeResult::Error { message: "not found".into() };
        let json = serde_json::to_string(&result).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["type"], "error");
        assert_eq!(parsed["message"], "not found");
    }

    #[test]
    fn search_result_serializes_success() {
        let result = SearchResult::Success {
            matches: vec![MatchResult {
                file: "lib.rs".into(),
                line: "fn   handle_request()".into(),
            }],
            total_matched: 1,
        };
        let json = serde_json::to_string(&result).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["type"], "success");
        assert_eq!(parsed["total_matched"], 1);
        assert_eq!(parsed["matches"][0]["file"], "lib.rs");
    }
}
