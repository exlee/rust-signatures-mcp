use regex::Regex;
use crate::analyze::analyze_path_structured;
use crate::types::{FileResult, MatchResult, SearchResult};
use crate::types::render_signature;
use crate::types::AnalyzeResult;

pub fn search_in_files(files: &[FileResult], query: &str) -> Result<SearchResult, String> {
    let re = Regex::new(query).map_err(|e| format!("Invalid regex \"{}\": {}", query, e))?;
    let mut matches = Vec::new();

    for file_result in files {
        for sig in &file_result.signatures {
            let rendered = render_signature(sig);
            if re.is_match(&rendered) {
                matches.push(MatchResult {
                    file: file_result.path.clone(),
                    line: rendered,
                });
            }
        }
    }

    let total_matched = matches.len();
    if matches.is_empty() {
        return Ok(SearchResult::Error {
            message: format!("No signatures matching regex \"{}\" found.", query),
        });
    }

    Ok(SearchResult::Success { matches, total_matched })
}

pub fn search_signatures_json(target: &str, query: &str) -> String {
    let result = analyze_path_structured(target, None);
    let files = match result {
        AnalyzeResult::Success { files } => files,
        AnalyzeResult::Error { message } => {
            return serde_json::to_string(&SearchResult::Error { message }).unwrap();
        }
    };

    match search_in_files(&files, query) {
        Ok(search_result) => serde_json::to_string(&search_result).unwrap(),
        Err(e) => serde_json::to_string(&SearchResult::Error { message: e }).unwrap(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Signature;

    #[test]
    fn search_finds_matching_signature() {
        let files = vec![FileResult {
            path: "test.rs".into(),
            signatures: vec![
                Signature::Fn {
                    docs: vec![],
                    signature: "fn   process_data(input: &str) -> Vec<u8>".into(),
                },
                Signature::Fn {
                    docs: vec![],
                    signature: "fn   cleanup()".into(),
                },
            ],
        }];
        let result = search_in_files(&files, "process_data").unwrap();
        match result {
            SearchResult::Success { matches, total_matched } => {
                assert_eq!(total_matched, 1);
                assert_eq!(matches[0].file, "test.rs");
                assert!(matches[0].line.contains("process_data"));
            }
            SearchResult::Error { message } => panic!("expected success, got error: {}", message),
        }
    }

    #[test]
    fn search_is_case_insensitive() {
        let files = vec![FileResult {
            path: "test.rs".into(),
            signatures: vec![Signature::Fn {
                docs: vec![],
                signature: "fn   ParseJSON()".into(),
            }],
        }];
        let result = search_in_files(&files, "(?i)parsejson").unwrap();
        match result {
            SearchResult::Success { total_matched, .. } => assert_eq!(total_matched, 1),
            SearchResult::Error { message } => panic!("expected success, got: {}", message),
        }
    }

    #[test]
    fn search_is_case_sensitive_by_default() {
        let files = vec![FileResult {
            path: "test.rs".into(),
            signatures: vec![Signature::Fn {
                docs: vec![],
                signature: "fn   ParseJSON()".into(),
            }],
        }];
        let result = search_in_files(&files, "parsejson").unwrap();
        match result {
            SearchResult::Error { message } => assert!(message.contains("No signatures matching")),
            SearchResult::Success { .. } => panic!("expected error for case-sensitive match"),
        }
    }

    #[test]
    fn search_returns_error_when_no_matches() {
        let files = vec![FileResult {
            path: "test.rs".into(),
            signatures: vec![Signature::Fn {
                docs: vec![],
                signature: "fn   foo()".into(),
            }],
        }];
        match search_in_files(&files, "nonexistent").unwrap() {
            SearchResult::Error { message } => assert!(message.contains("No signatures matching")),
            SearchResult::Success { .. } => panic!("expected error"),
        }
    }

    #[test]
    fn search_matches_doc_comments() {
        let files = vec![FileResult {
            path: "test.rs".into(),
            signatures: vec![Signature::Fn {
                docs: vec!["Retrieves the user profile from the database.".into()],
                signature: "fn   get_user()".into(),
            }],
        }];
        let result = search_in_files(&files, "database").unwrap();
        match result {
            SearchResult::Success { total_matched, .. } => assert_eq!(total_matched, 1),
            SearchResult::Error { message } => panic!("expected success, got: {}", message),
        }
    }

    #[test]
    fn search_regex_patterns_work() {
        let files = vec![FileResult {
            path: "test.rs".into(),
            signatures: vec![
                Signature::Fn {
                    docs: vec![],
                    signature: "async fn fetch_data() -> Result<Data>".into(),
                },
                Signature::Fn {
                    docs: vec![],
                    signature: "fn sync_process()".into(),
                },
            ],
        }];
        let result = search_in_files(&files, "async fn\\s+\\w+").unwrap();
        match result {
            SearchResult::Success { total_matched, .. } => assert_eq!(total_matched, 1),
            SearchResult::Error { message } => panic!("expected success, got: {}", message),
        }
    }

    #[test]
    fn search_invalid_regex_returns_error() {
        let files = vec![FileResult {
            path: "test.rs".into(),
            signatures: vec![],
        }];
        let result = search_in_files(&files, "(?invalid");
        match result {
            Err(e) => assert!(e.contains("Invalid regex")),
            Ok(_) => panic!("expected error for invalid regex"),
        }
    }

    #[test]
    fn search_signatures_json_produces_valid_json() {
        let tmp = std::env::temp_dir().join("rust_sig_search.rs");
        std::fs::write(&tmp, "fn fetch_user(id: u64) -> User { unimplemented!() }\n").unwrap();
        let json = search_signatures_json(tmp.to_str().unwrap(), "fetch_user");
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["type"], "success");
        assert_eq!(parsed["total_matched"], 1);
        assert!(parsed["matches"][0]["line"].as_str().unwrap().contains("fetch_user"));
        std::fs::remove_file(&tmp).ok();
    }

    #[test]
    fn search_signatures_json_no_match() {
        let tmp = std::env::temp_dir().join("rust_sig_nomatch.rs");
        std::fs::write(&tmp, "fn hello() {}\n").unwrap();
        let json = search_signatures_json(tmp.to_str().unwrap(), "goodbye");
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["type"], "error");
        std::fs::remove_file(&tmp).ok();
    }

    #[test]
    fn search_signatures_json_invalid_regex() {
        let tmp = std::env::temp_dir().join("rust_sig_badregex.rs");
        std::fs::write(&tmp, "fn hello() {}\n").unwrap();
        let json = search_signatures_json(tmp.to_str().unwrap(), "[invalid");
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["type"], "error");
        std::fs::remove_file(&tmp).ok();
    }
}
