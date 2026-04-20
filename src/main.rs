mod analyze;
mod collector;
mod registry;
mod search;
mod server;
mod types;

use rmcp::ServiceExt;
use server::RustSigServer;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let transport = rmcp::transport::stdio();
    let server = RustSigServer::new();
    server.serve(transport).await?.waiting().await?;
    Ok(())
}

#[cfg(test)]
mod integration_tests {
    use crate::analyze::analyze_to_json;

    #[test]
    fn analyze_self_returns_structured_json() {
        let json = analyze_to_json("src/main.rs", Some(5));
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["type"], "success");
        assert_eq!(parsed["files"][0]["path"], "src/main.rs");
        let sigs = &parsed["files"][0]["signatures"];
        assert!(sigs.as_array().unwrap().len() <= 5);
    }
}
