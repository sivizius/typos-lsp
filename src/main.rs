use tower_lsp::*;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug)]
struct Backend {
    client: Client,
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, _: InitializeParams) -> jsonrpc::Result<InitializeResult> {
        Ok(InitializeResult {
            server_info: Some(ServerInfo {
                name: "typos-lsp".to_string(),
                version: Some(VERSION.into()),
            }),
            ..Default::default()
        })
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        self.create_diagnostics(TextDocumentItem {
            language_id: params.text_document.language_id,
            uri: params.text_document.uri,
            text: params.text_document.text,
            version: params.text_document.version,
        })
        .await
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "server initialized!")
            .await;
    }

    async fn shutdown(&self) -> jsonrpc::Result<()> {
        Ok(())
    }
}

impl Backend {
    async fn create_diagnostics(&self, params: TextDocumentItem) {
        let diagnostics = vec![Diagnostic::default()];
        self.client
            .publish_diagnostics(params.uri.clone(), diagnostics, Some(params.version))
            .await;
    }
}

#[tokio::main]
async fn main() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(|client| Backend { client });
    Server::new(stdin, stdout, socket).serve(service).await;
}

#[cfg(test)]
mod tests {
    use std::{error::Error, io::Write};

    use super::*;
    use bytes::{Buf, BytesMut, BufMut};
    use tracing::{trace, warn};
    use serde::ser::Serialize;

    #[tokio::test]
    async fn test_initialize() {
        let (service, _) = LspService::new(|client| Backend { client });

        let params = InitializeParams::default();
        let result = service.inner().initialize(params).await.unwrap();
        let server_info = result.server_info.unwrap();

        assert_eq!(server_info.name, "typos-lsp".to_string());
        assert_eq!(server_info.version, Some(env!("CARGO_PKG_VERSION").into()));
    }

    #[test_log::test(tokio::test)]
    async fn test_initialize_e2e() {
        let (service, socket) = LspService::new(|client| Backend { client });

        // new an vec u8

        let input: &[u8] = br#"{
            "jsonrpc": "2.0",
            "method": "initialize",
            "params": {
              "capabilities": {}
            },
            "id": 1
          }"#;

        let mut buf = BytesMut::new();
        encode(input, &mut buf).unwrap();

        //let mut output = String::new();
        let stdout = tokio::io::stdout();

        Server::new(buf.as_ref(), stdout, socket).serve(service).await;
    }

    // copied from tower_lsp
    fn encode<T: Serialize>(item: T, dst: &mut BytesMut) -> Result<(), anyhow::Error> {
        let msg = serde_json::to_string(&item)?;
        // TODO: log to stdout
        tracing::debug!("-> {}", msg);

        // Reserve just enough space to hold the `Content-Length: ` and `\r\n\r\n` constants,
        // the length of the message, and the message body.
        dst.reserve(msg.len() + number_of_digits(msg.len()) + 20);
        let mut writer = dst.writer();
        write!(writer, "Content-Length: {}\r\n\r\n{}", msg.len(), msg)?;
        writer.flush()?;

        Ok(())
    }

    #[inline]
    fn number_of_digits(mut n: usize) -> usize {
        let mut num_digits = 0;

        while n > 0 {
            n /= 10;
            num_digits += 1;
        }

        num_digits
    }


    #[tokio::test]
    async fn test_did_open() {
        let (service, socket) = LspService::new(|client| Backend { client });

        let params = DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: Url::parse("file:///foo.rs").unwrap(),
                language_id: "rust".into(),
                version: 1,
                text: "foobar".into()
            }
        };

        service.inner().did_open(params).await;


        // let stdin = tokio::io::stdin();
        // let stdout = tokio::io::stdout();

        // Server::new(stdin, stdout, socket).serve(service).await;


        // let (req_stream, res_sink) = client_socket.split();

        // let (client_requests, client_abort) = stream::abortable(req_stream);

        // let stdout = tokio::io::stdout();
        // let framed_stdout = FramedWrite::new(stdout, LanguageServerCodec::default());

        // let print_output = client_requests.map(Ok).forward(framed_stdout).await.unwrap();


        // let print_output = stream::select(responses_rx, client_requests.map(Message::Request))
        //     .map(Ok)
        //     .forward(framed_stdout.sink_map_err(|e| error!("failed to encode message: {}", e)))
        //     .map(|_| ());




        // while let Some(req) = client_socket.split().0.a {
        //     println!("Received: {:?}", req);

        // }

    }
}
