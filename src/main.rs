pub mod norminette_msg;
pub mod parser;

use std::io;
use std::path::Path;

use parser::parse_norminette;
use serde_json::Value;
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

    async fn did_open(&self, p: DidOpenTextDocumentParams) {
        self.client
            .log_message(MessageType::INFO, "file opened!")
            .await;
        match read_norminette(&Path::new(p.text_document.uri.path())) {
            Ok(diags) => {
                self.client
                    .publish_diagnostics(p.text_document.uri, diags, None)
                    .await;
            }
            Err(e) => {
                self.client.log_message(MessageType::ERROR, format!("norminette read of {} failed: {}", p.text_document.uri, e)).await;
            }
        }
    }

    async fn did_change(&self, p: DidChangeTextDocumentParams) {
        self.client
            .log_message(MessageType::INFO, "file changed!")
            .await;
        match read_norminette(&Path::new(p.text_document.uri.path())) {
            Ok(diags) => {
                self.client
                    .publish_diagnostics(p.text_document.uri, diags, None)
                    .await;
            }
            Err(e) => {
                self.client.log_message(MessageType::ERROR, format!("norminette read of {} failed: {}", p.text_document.uri, e)).await;
            }
        }

    }

    async fn did_save(&self, p: DidSaveTextDocumentParams) {
        self.client
            .log_message(MessageType::INFO, format!("file saved: {:?}", p.text))
            .await;
        match read_norminette(&Path::new(p.text_document.uri.path())) {
            Ok(diags) => {
                self.client
                    .publish_diagnostics(p.text_document.uri, diags, None)
                    .await;
            },
            Err(e) => {
                self.client.log_message(MessageType::ERROR, format!("norminette read of {} failed: {}", p.text_document.uri, e)).await;
            }
        }
    }

    async fn did_close(&self, _: DidCloseTextDocumentParams) {
        self.client
            .log_message(MessageType::INFO, "file closed!")
            .await;
    }

    async fn completion(&self, _: CompletionParams) -> Result<Option<CompletionResponse>> {
        Err(jsonrpc::Error::method_not_found())
    }

    async fn did_change_configuration(&self, _: DidChangeConfigurationParams) {
        self.client
            .log_message(MessageType::INFO, "configuration changed!")
            .await;
    }

    async fn did_change_workspace_folders(&self, _: DidChangeWorkspaceFoldersParams) {
        self.client
            .log_message(MessageType::INFO, "workspace folders changed!")
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
}

pub fn read_norminette(path: &Path) -> io::Result<Vec<Diagnostic>> {
    let output = std::process::Command::new("norminette")
        .arg(path)
        .output()?;
    let (_, diags) = parse_norminette(&String::from_utf8(output.stdout).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?)
        .map_err(|err| io::Error::new(io::ErrorKind::Other, format!("norminette parse error: {:?}", err)))?;

    Ok(diags
        .into_iter()
        .map(|d| d.to_diagnostic())
        .filter(|o| o.is_some())
        .map(|o| o.unwrap())
        .collect())
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt().init();

    let (stdin, stdout) = (tokio::io::stdin(), tokio::io::stdout());

    let (service, socket) = LspService::new(|client| Backend { client });
    Server::new(stdin, stdout, socket).serve(service).await;
}
