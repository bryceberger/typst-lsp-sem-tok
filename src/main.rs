mod tokens;

use tokens::{ToSemanticToken, TypstSemanticToken};

use strum::IntoEnumIterator;

use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};
use typst::ide::{highlight, Tag};
use typst::syntax::{LinkedNode, SyntaxKind};

use dashmap::DashMap;
use ropey::Rope;

struct Backend {
    client: Client,
    document_map: DashMap<Url, Rope>,
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, _params: InitializeParams) -> Result<InitializeResult> {
        let text_document_sync = Some(TextDocumentSyncCapability::Kind(TextDocumentSyncKind::FULL));

        let semantic_tokens_provider = Some(
            SemanticTokensServerCapabilities::SemanticTokensOptions(SemanticTokensOptions {
                legend: SemanticTokensLegend {
                    token_types: TypstSemanticToken::iter()
                        .map(|var| SemanticTokenType::new(var.to_name()))
                        .collect(),
                    token_modifiers: vec![],
                },
                full: Some(SemanticTokensFullOptions::Bool(true)),
                ..Default::default()
            }),
        );

        let capabilities = ServerCapabilities {
            text_document_sync,
            semantic_tokens_provider,
            ..Default::default()
        };

        Ok(InitializeResult {
            capabilities,
            ..Default::default()
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "server initialized!")
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn semantic_tokens_full(
        &self,
        params: SemanticTokensParams,
    ) -> Result<Option<SemanticTokensResult>> {
        let uri = params.text_document.uri;
        // shouldn't be able to ask for the tokens of a document without opening it
        let text = self.document_map.get(&uri).unwrap();
        let source = typst::syntax::parse(&text.chunks().collect::<String>());
        let root = LinkedNode::new(&source);

        let mut data = Vec::new();

        traverse_highlight(root, &mut data);

        Ok(Some(SemanticTokensResult::Tokens(SemanticTokens {
            result_id: None,
            data,
        })))
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        self.client
            .log_message(MessageType::INFO, "file opened!")
            .await;
        let rope = ropey::Rope::from_str(&params.text_document.text);
        self.document_map
            .insert(params.text_document.uri, rope.clone());
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri;
        let changes = params.content_changes;
        // shouldn't be able to change a document without opening it
        let mut rope = self.document_map.get_mut(&uri).unwrap();

        for change in changes {
            if let Some(Range { start, end }) = change.range {
                let start_idx = rope.line_to_char(start.line as usize) + start.character as usize;
                let end_idx = rope.line_to_char(end.line as usize) + end.character as usize;
                rope.remove(start_idx..end_idx);
                rope.insert(start_idx, &change.text);
            } else {
                *rope = Rope::from_str(&change.text);
            }
        }
    }
}

struct HighlightFeedForward {
    delta_line: u32,
    delta_start: u32,
}

fn traverse_highlight(node: LinkedNode, tokens: &mut Vec<SemanticToken>) {
    traverse_highlight_rec(
        node,
        tokens,
        HighlightFeedForward {
            delta_line: 0,
            delta_start: 0,
        },
    );
}

fn traverse_highlight_rec(
    node: LinkedNode,
    tokens: &mut Vec<SemanticToken>,
    mut ff: HighlightFeedForward,
) -> HighlightFeedForward {
    let children = node.children();

    let len = children.len();
    // emph and strong decompose into `*` `text` `*`, want to highlight entire thing as one
    // group
    if !matches!(
        highlight(&node).into(),
        TypstSemanticToken::Emph | TypstSemanticToken::Strong
    ) {
        for child in children {
            ff = traverse_highlight_rec(child, tokens, ff);
        }
        if len > 0 {
            return ff;
        }
    }

    // leaf node (or strong or emph)
    let HighlightFeedForward {
        delta_line,
        delta_start,
    } = ff;

    let highlight_type = highlight(&node).into();
    let node_len = node.range().len() as u32;

    let (skip_line, skip_start) = if matches!(highlight_type, TypstSemanticToken::None) {
        (delta_line, delta_start + node_len)
    } else {
        tokens.push(SemanticToken {
            delta_line,
            delta_start,
            length: node_len,
            token_type: highlight_type.to_idx(),
            token_modifiers_bitset: 0,
        });
        (0, node_len)
    };

    if matches!(node.kind(), SyntaxKind::Space | SyntaxKind::Parbreak) && node.text().contains("\n")
    {
        HighlightFeedForward {
            // one or more linebreaks
            delta_line: skip_line + node.text().matches("\n").count() as u32,
            // might have spaces at the beginning of the last line
            delta_start: node
                .text()
                .split("\n")
                .last()
                .map_or(0, |string| string.len()) as u32,
        }
    } else if matches!(node.kind(), SyntaxKind::Raw) && node.text().contains("\n") {
        // this is a multiline raw block
        // mark each included line as raw
        let mut last_len = 0;
        // skip the first because we've already done it
        for line in node.text().split("\n").skip(1) {
            tokens.push(SemanticToken {
                delta_line: 1,
                delta_start: 0,
                length: line.len() as u32,
                token_type: Tag::Raw.to_idx(),
                token_modifiers_bitset: 0,
            });
            last_len = line.len() as u32;
        }
        HighlightFeedForward {
            delta_line: 0,
            delta_start: last_len,
        }
    } else {
        HighlightFeedForward {
            delta_line: skip_line,
            delta_start: skip_start,
        }
    }
}

#[tokio::main]
async fn main() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(|client| Backend {
        client,
        document_map: DashMap::new(),
    });
    Server::new(stdin, stdout, socket).serve(service).await;
}
