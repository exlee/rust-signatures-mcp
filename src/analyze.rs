use std::fs;
use std::path::Path;
use ignore::WalkBuilder;

use crate::collector::analyze_source;
use crate::types::{AnalyzeResult, FileResult};

pub fn analyze_file(path: &Path) -> Option<FileResult> {
    let src = fs::read_to_string(path).ok()?;
    let signatures = analyze_source(&src);
    Some(FileResult {
        path: path.display().to_string(),
        signatures,
    })
}

pub fn analyze_path_structured(target: &str, max_signatures: Option<usize>) -> AnalyzeResult {
    let path = Path::new(target);
    if !path.exists() {
        return AnalyzeResult::Error { message: format!("Path not found: {}", target) };
    }
    if path.is_file() {
        if path.extension().and_then(|e| e.to_str()) != Some("rs") {
            return AnalyzeResult::Error { message: "Not a Rust file.".to_string() };
        }
        match analyze_file(path) {
            Some(mut file_result) => {
                if let Some(max) = max_signatures {
                    file_result.signatures.truncate(max);
                }
                AnalyzeResult::Success { files: vec![file_result] }
            }
            None => AnalyzeResult::Error { message: "Failed to parse file.".to_string() },
        }
    } else {
        let mut file_results = Vec::new();
        let mut total_sigs = 0usize;
        for entry in WalkBuilder::new(target).build().filter_map(|e| e.ok()) {
            let entry_path = entry.path();
            if entry_path.extension().and_then(|e| e.to_str()) != Some("rs") { continue; }
            if let Some(mut file_result) = analyze_file(entry_path) {
                if let Some(max) = max_signatures {
                    let remaining = max.saturating_sub(total_sigs);
                    if remaining == 0 { break; }
                    file_result.signatures.truncate(remaining);
                    total_sigs += file_result.signatures.len();
                }
                file_results.push(file_result);
            }
        }
        if file_results.is_empty() {
            return AnalyzeResult::Error { message: "No Rust files found.".to_string() };
        }
        AnalyzeResult::Success { files: file_results }
    }
}

pub fn analyze_to_json(target: &str, max_signatures: Option<usize>) -> String {
    let result = analyze_path_structured(target, max_signatures);
    serde_json::to_string(&result).unwrap_or_else(|_| r#"{"type":"error","message":"Failed to serialize result"}"#.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn analyze_path_nonexistent() {
        let result = analyze_path_structured("/nonexistent/path/to/file.rs", None);
        match result {
            AnalyzeResult::Error { message } => assert!(message.contains("Path not found")),
            AnalyzeResult::Success { .. } => panic!("expected error"),
        }
    }

    #[test]
    fn analyze_path_non_rs_file() {
        let tmp = std::env::temp_dir().join("rust_sig_test_non_rs.txt");
        fs::write(&tmp, "not rust").unwrap();
        let result = analyze_path_structured(tmp.to_str().unwrap(), None);
        match result {
            AnalyzeResult::Error { message } => assert!(message.contains("Not a Rust file")),
            AnalyzeResult::Success { .. } => panic!("expected error"),
        }
        fs::remove_file(&tmp).ok();
    }

    #[test]
    fn analyze_path_valid_file() {
        let tmp = std::env::temp_dir().join("rust_sig_test_valid.rs");
        fs::write(&tmp, "fn test_fn() -> bool { true }\nstruct TestStruct;\n").unwrap();
        let result = analyze_path_structured(tmp.to_str().unwrap(), None);
        match result {
            AnalyzeResult::Success { files } => {
                assert_eq!(files.len(), 1);
                assert_eq!(files[0].signatures.len(), 2);
            }
            AnalyzeResult::Error { message } => panic!("expected success, got: {}", message),
        }
        fs::remove_file(&tmp).ok();
    }

    #[test]
    fn analyze_path_max_signatures() {
        let tmp = std::env::temp_dir().join("rust_sig_test_max.rs");
        fs::write(&tmp, "fn a() {}\nfn b() {}\nfn c() {}\nfn d() {}\nfn e() {}\n").unwrap();
        let result = analyze_path_structured(tmp.to_str().unwrap(), Some(2));
        match result {
            AnalyzeResult::Success { files } => {
                assert_eq!(files[0].signatures.len(), 2);
            }
            AnalyzeResult::Error { message } => panic!("expected success, got: {}", message),
        }
        fs::remove_file(&tmp).ok();
    }

    #[test]
    fn analyze_path_directory_walks_rs_files() {
        let dir = std::env::temp_dir().join("rust_sig_test_dir");
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("a.rs"), "fn from_a() {}\n").unwrap();
        fs::write(dir.join("b.rs"), "fn from_b() {}\n").unwrap();
        fs::write(dir.join("c.txt"), "ignore me").unwrap();
        fs::create_dir_all(dir.join("nested")).unwrap();
        fs::write(dir.join("nested/d.rs"), "fn from_d() {}\n").unwrap();

        let result = analyze_path_structured(dir.to_str().unwrap(), None);
        match result {
            AnalyzeResult::Success { files } => {
                assert_eq!(files.len(), 3);
                let paths: Vec<_> = files.iter().map(|f| &f.path).collect();
                assert!(paths.iter().any(|p| p.ends_with("a.rs")));
                assert!(paths.iter().any(|p| p.ends_with("b.rs")));
                assert!(paths.iter().any(|p| p.contains("nested") && p.ends_with("d.rs")));
            }
            AnalyzeResult::Error { message } => panic!("expected success, got: {}", message),
        }

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn analyze_path_empty_directory() {
        let dir = std::env::temp_dir().join("rust_sig_test_empty");
        fs::create_dir_all(&dir).unwrap();
        let result = analyze_path_structured(dir.to_str().unwrap(), None);
        match result {
            AnalyzeResult::Error { message } => assert!(message.contains("No Rust files found")),
            AnalyzeResult::Success { .. } => panic!("expected error"),
        }
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn analyze_path_directory_max_signatures() {
        let dir = std::env::temp_dir().join("rust_sig_test_dirmax");
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("a.rs"), "fn a1() {}\nfn a2() {}\n").unwrap();
        fs::write(dir.join("b.rs"), "fn b1() {}\nfn b2() {}\n").unwrap();

        let result = analyze_path_structured(dir.to_str().unwrap(), Some(3));
        match result {
            AnalyzeResult::Success { files } => {
                let total: usize = files.iter().map(|f| f.signatures.len()).sum();
                assert_eq!(total, 3);
            }
            AnalyzeResult::Error { message } => panic!("expected success, got: {}", message),
        }
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn analyze_to_json_produces_valid_json() {
        let tmp = std::env::temp_dir().join("rust_sig_json.rs");
        fs::write(&tmp, "fn json_test() -> u64 { 42 }\n").unwrap();
        let json = analyze_to_json(tmp.to_str().unwrap(), None);
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["type"], "success");
        assert_eq!(parsed["files"][0]["signatures"][0]["kind"], "fn");
        fs::remove_file(&tmp).ok();
    }
}
