pub mod norminette_msg;
pub mod parser;

use std::io;
use std::path::Path;
use std::sync::Arc;

use parser::parse_norminette;
use serde_json::Value;
use tokio::sync::Mutex;
use tower_lsp::jsonrpc::{self, Result};
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};

#[derive(Debug)]
struct Backend {
    client: Client,
    // norminette_options: Arc<Mutex<?>>
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            server_info: None,
            capabilities: ServerCapabilities {
                diagnostic_provider: Some(DiagnosticServerCapabilities::Options(
                    DiagnosticOptions::default(),
                )),
                workspace: Some(WorkspaceServerCapabilities {
                    workspace_folders: Some(WorkspaceFoldersServerCapabilities {
                        supported: Some(true),
                        change_notifications: None,
                    }),
                    file_operations: None,
                }),
                ..Default::default()
            },
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "initialized!")
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_change_workspace_folders(&self, _: DidChangeWorkspaceFoldersParams) {
        self.client
            .log_message(MessageType::INFO, "workspace folders changed!")
            .await;
    }

    async fn did_change_configuration(&self, _: DidChangeConfigurationParams) {
        self.client
            .log_message(MessageType::INFO, "configuration changed!")
            .await;
    }

    async fn did_change_watched_files(&self, _: DidChangeWatchedFilesParams) {
        self.client
            .log_message(MessageType::INFO, "watched files have changed!")
            .await;
    }

    async fn execute_command(&self, _: ExecuteCommandParams) -> Result<Option<Value>> {
        Err(jsonrpc::Error::method_not_found())
    }

    async fn did_open(&self, _: DidOpenTextDocumentParams) {
        self.client
            .log_message(MessageType::INFO, "file opened!")
            .await;
    }

    async fn did_change(&self, _: DidChangeTextDocumentParams) {
        self.client
            .log_message(MessageType::INFO, "file changed!")
            .await;
    }

    async fn did_save(&self, p: DidSaveTextDocumentParams) {
        self.client
            .log_message(MessageType::INFO, format!("file saved: {:?}", p.text))
            .await;
        let diags = read_norminette(&Path::new(p.text_document.uri.as_str())).expect(&format!(
            "norminette read failed of {}",
            p.text_document.uri
        ));
        self.client
            .publish_diagnostics(p.text_document.uri, diags, None)
            .await
    }

    async fn did_close(&self, _: DidCloseTextDocumentParams) {
        self.client
            .log_message(MessageType::INFO, "file closed!")
            .await;
    }

    async fn completion(&self, _: CompletionParams) -> Result<Option<CompletionResponse>> {
        Err(jsonrpc::Error::method_not_found())
    }
}

pub fn read_norminette(path: &Path) -> io::Result<Vec<Diagnostic>> {
    let output = std::process::Command::new("norminette")
        .arg(path)
        .output()?;
    let (_, diags) = parse_norminette(&String::from_utf8(output.stdout).expect("not valid utf8"))
        .unwrap_or_else(|err| panic!("norminette parse error: {}", err));

    Ok(diags.into_iter().map(|d| d.to_diagnostic()).collect())
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt().init();

    let (stdin, stdout) = (tokio::io::stdin(), tokio::io::stdout());

    let (service, socket) = LspService::new(|client| Backend { client });
    Server::new(stdin, stdout, socket).serve(service).await;
}
