mod client;
mod tools;

use rmcp::ServiceExt;
use tools::MudMcpServer;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Log to stderr (stdout is used by MCP stdio transport)
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "warn".into()),
        )
        .init();

    let base_url = std::env::var("MUDCLIENT_API_URL")
        .unwrap_or_else(|_| "http://127.0.0.1:9527".to_string());

    let server = MudMcpServer::new(&base_url);
    let transport = (tokio::io::stdin(), tokio::io::stdout());
    let service = server.serve(transport).await?;
    let _ = service.waiting().await?;
    Ok(())
}
