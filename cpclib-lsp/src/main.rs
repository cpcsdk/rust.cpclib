use tower_lsp::{LspService, Server};
use cpclib_lsp::CpcLspBackend;

#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .with_writer(std::io::stderr)
        .init();

    tracing::info!("Starting cpclib-lsp server");

    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(CpcLspBackend::new);
    
    Server::new(stdin, stdout, socket)
        .serve(service)
        .await;
}
