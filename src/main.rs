pub mod norminette_msg;
pub mod parser;

use lsp_server::{Connection, ExtractError, Message, Notification, Request, RequestId, Response};
use lsp_types::notification::{DidOpenTextDocument, DidSaveTextDocument};
use lsp_types::request::DocumentDiagnosticRequest;
use lsp_types::{
    Diagnostic, DiagnosticOptions, DiagnosticServerCapabilities, DidOpenTextDocumentParams,
    DidSaveTextDocumentParams, InitializeParams, PublishDiagnosticsParams, SaveOptions,
    ServerCapabilities, TextDocumentSyncCapability, TextDocumentSyncOptions,
    TextDocumentSyncSaveOptions, WorkDoneProgressOptions,
};
use parser::parse_norminette;
use serde::Deserialize;
use std::error::Error;
use std::path::Path;
use std::{io, process};

#[derive(Deserialize)]
pub struct InitOptions {
    pub path: String,
    pub name: String,
    pub email: String,
    pub offset: Option<i32>,
}

macro_rules! diag_on_event {
    ($conn: expr, $noti: expr, $t: ident, $f: expr) => {
        match cast_noti::<$t>($noti) {
            Ok(params) => {
                eprintln!("got document notification: {params:?}");
                notify_diagnostics!($conn, &params, $f);
            }
            Err(_) => {}
        }
    };
    ($conn: expr, $noti: expr, $t: ident, $f: expr, $option: expr) => {
        match cast_noti::<$t>($noti) {
            Ok(params) => {
                eprintln!("got document notification: {params:?}");
                match $option {
                    Ok(Ok(ref op)) => {
                        let output = process::Command::new(&op.path)
                            .args([
                                "--name",
                                &op.name,
                                "--email",
                                &op.email,
                                "--path",
                                params.text_document.uri.path().as_str(),
                                "--offset",
                                &format!("{}", op.offset.unwrap_or(0)),
                            ])
                            .output()
                            .map_err(|e| format!("failed to execute {}: {e:?}", op.path))?;
                        if !output.status.success() {
                            eprintln!("{}", String::from_utf8(output.stderr).unwrap());
                        }
                    }
                    Err(ref e) => eprintln!("{e}"),
                    Ok(Err(ref e)) => eprintln!("{e}"),
                }
                notify_diagnostics!($conn, &params, $f);
            }
            Err(_) => {}
        }
    };
}

macro_rules! notify_diagnostics {
    ($conn: expr, $params: expr, $f: expr) => {
        let text = $f.map(|f_| f_($params));
        match read_norminette(&Path::new($params.text_document.uri.path().as_str()), text) {
            Ok(diags) => {
                $conn.sender.send(Message::Notification(Notification {
                    method: String::from("textDocument/publishDiagnostics"),
                    params: serde_json::to_value(&PublishDiagnosticsParams {
                        uri: $params.text_document.uri.clone(),
                        diagnostics: diags,
                        version: None,
                    })?,
                }))?;
            }
            Err(e) => {
                eprintln!(
                    "norminette read of {} failed: {}",
                    $params.text_document.uri.path(),
                    e
                );
            }
        }
    };
}

macro_rules! send_diagnostics {
    ($conn: expr, $id: expr, $params: expr) => {
        match read_norminette(&Path::new($params.text_document.uri.path().as_str()), None) {
            Ok(diags) => {
                $conn.sender.send(Message::Response(Response {
                    id: $id,
                    result: Some(serde_json::to_value(&PublishDiagnosticsParams {
                        uri: $params.text_document.uri,
                        diagnostics: diags,
                        version: None,
                    })?),
                    error: None,
                }))?;
            }
            Err(e) => {
                eprintln!(
                    "norminette read of {} failed: {}",
                    $params.text_document.uri.path(),
                    e
                );
            }
        }
    };
}

fn read_norminette(path: &Path, text: Option<String>) -> io::Result<Vec<Diagnostic>> {
    let mut cmd = process::Command::new("norminette");
    match text {
        Some(text) => {
            cmd.args(["--cfile", &text, "--filename", path.to_str().unwrap()]);
        }
        None => {
            cmd.arg(path);
        }
    }
    let (_, diags) = parse_norminette(
        &String::from_utf8(cmd.output()?.stdout)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?,
    )
    .map_err(|err| {
        io::Error::new(
            io::ErrorKind::Other,
            format!("norminette parse error: {:?}", err),
        )
    })?;

    Ok(diags
        .into_iter()
        .map(|d| d.to_diagnostic())
        .filter(|o| o.is_some())
        .map(|o| o.unwrap())
        .collect())
}

fn main() -> Result<(), Box<dyn Error + Sync + Send>> {
    eprintln!("starting norminette LSP server");

    let (connection, io_threads) = Connection::stdio();

    let server_capabilities = serde_json::to_value(&ServerCapabilities {
        diagnostic_provider: Some(DiagnosticServerCapabilities::Options(DiagnosticOptions {
            identifier: None,
            inter_file_dependencies: false,
            workspace_diagnostics: false,
            work_done_progress_options: WorkDoneProgressOptions::default(),
        })),
        text_document_sync: Some(TextDocumentSyncCapability::Options(
            TextDocumentSyncOptions {
                open_close: Some(true),
                change: None,
                save: Some(TextDocumentSyncSaveOptions::SaveOptions(SaveOptions {
                    include_text: Some(true),
                })),
                ..Default::default()
            },
        )),
        ..Default::default()
    })?;
    let initialization_params = match connection.initialize(server_capabilities) {
        Ok(it) => it,
        Err(e) => {
            if e.channel_is_disconnected() {
                io_threads.join()?;
            }
            return Err(e.into());
        }
    };
    eprintln!("initialized connection!");
    main_loop(connection, initialization_params)?;
    io_threads.join()?;
    eprintln!("shutting down server");
    Ok(())
}

fn main_loop(
    connection: Connection,
    params: serde_json::Value,
) -> Result<(), Box<dyn Error + Sync + Send>> {
    let params: InitializeParams = serde_json::from_value(params)?;
    let options = params
        .initialization_options
        .ok_or_else(|| "missing initialization options".to_string())
        .map(|o| {
            InitOptions::deserialize(&o).map_err(|e| format!("deserialization failed: {e:?}"))
        });

    for msg in &connection.receiver {
        eprintln!("got msg: {msg:?}");
        match msg {
            Message::Request(req) => {
                if connection.handle_shutdown(&req)? {
                    return Ok(());
                }
                eprintln!("got request: {req:?}");
                match cast::<DocumentDiagnosticRequest>(req) {
                    Ok((id, params)) => {
                        eprintln!("got doc diagnostic request #{id} params: {params:?}");
                        send_diagnostics!(connection, id, params);
                    }
                    Err(e) => {
                        eprintln!("got error: {e:?}");
                        continue;
                    }
                };
            }
            Message::Response(resp) => {
                eprintln!("got response: {resp:?}");
            }
            Message::Notification(not) => {
                eprintln!("got notification: {not:?}");
                diag_on_event!(
                    connection,
                    not.clone(),
                    DidOpenTextDocument,
                    Some(|p: &DidOpenTextDocumentParams| p.text_document.text.clone())
                );
                diag_on_event!(
                    connection,
                    not,
                    DidSaveTextDocument,
                    Some(|p: &DidSaveTextDocumentParams| p
                        .text
                        .clone()
                        .expect("includeText set to true yet text was None")),
                    options
                );
            }
        }
    }
    Ok(())
}

fn cast<R>(req: Request) -> Result<(RequestId, R::Params), ExtractError<Request>>
where
    R: lsp_types::request::Request,
    R::Params: serde::de::DeserializeOwned,
{
    req.extract(R::METHOD)
}

fn cast_noti<N>(noti: Notification) -> Result<N::Params, ExtractError<Notification>>
where
    N: lsp_types::notification::Notification,
    N::Params: serde::de::DeserializeOwned,
{
    noti.extract(N::METHOD)
}
