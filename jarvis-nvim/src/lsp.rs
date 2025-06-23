use anyhow::Result;
use tower_lsp::jsonrpc::Result as LspResult;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};
use std::sync::Arc;
use crate::ai_integration::AIIntegration;

pub struct JarvisLspServer {
    client: Client,
    ai: Arc<AIIntegration>,
}

impl JarvisLspServer {
    pub fn new(client: Client, ai: Arc<AIIntegration>) -> Self {
        Self { client, ai }
    }

    pub async fn start(ai: Arc<AIIntegration>) -> Result<()> {
        let stdin = tokio::io::stdin();
        let stdout = tokio::io::stdout();

        let (service, socket) = LspService::new(|client| JarvisLspServer::new(client, ai));
        Server::new(stdin, stdout, socket).serve(service).await;

        Ok(())
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for JarvisLspServer {
    async fn initialize(&self, _: InitializeParams) -> LspResult<InitializeResult> {
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::INCREMENTAL,
                )),
                code_action_provider: Some(CodeActionProviderCapability::Simple(true)),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                completion_provider: Some(CompletionOptions {
                    resolve_provider: Some(false),
                    trigger_characters: Some(vec![".".to_string(), "::".to_string()]),
                    work_done_progress_options: Default::default(),
                    all_commit_characters: None,
                    completion_item: None,
                }),
                execute_command_provider: Some(ExecuteCommandOptions {
                    commands: vec![
                        "jarvis.explain".to_string(),
                        "jarvis.improve".to_string(),
                        "jarvis.fix".to_string(),
                        "jarvis.generate".to_string(),
                        "jarvis.refactor".to_string(),
                        "jarvis.test".to_string(),
                    ],
                    work_done_progress_options: Default::default(),
                }),
                ..ServerCapabilities::default()
            },
            ..Default::default()
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "Jarvis LSP Server initialized!")
            .await;
    }

    async fn shutdown(&self) -> LspResult<()> {
        Ok(())
    }

    async fn hover(&self, params: HoverParams) -> LspResult<Option<Hover>> {
        let uri = &params.text_document_position_params.text_document.uri;
        let position = &params.text_document_position_params.position;

        // This would get the symbol at position and provide AI-powered explanations
        let content = format!("Jarvis AI explanation for symbol at {}:{}", position.line, position.character);
        
        Ok(Some(Hover {
            contents: HoverContents::Scalar(MarkedString::String(content)),
            range: None,
        }))
    }

    async fn code_action(&self, params: CodeActionParams) -> LspResult<Option<CodeActionResponse>> {
        let mut actions = Vec::new();

        // Add Jarvis-specific code actions
        actions.push(CodeActionOrCommand::CodeAction(CodeAction {
            title: "🤖 Jarvis: Explain Code".to_string(),
            kind: Some(CodeActionKind::REFACTOR),
            diagnostics: None,
            edit: None,
            command: Some(Command {
                title: "Explain Code".to_string(),
                command: "jarvis.explain".to_string(),
                arguments: Some(vec![
                    serde_json::to_value(&params.text_document.uri).unwrap(),
                    serde_json::to_value(&params.range).unwrap(),
                ]),
            }),
            is_preferred: Some(false),
            disabled: None,
            data: None,
        }));

        actions.push(CodeActionOrCommand::CodeAction(CodeAction {
            title: "🚀 Jarvis: Suggest Improvements".to_string(),
            kind: Some(CodeActionKind::REFACTOR),
            diagnostics: None,
            edit: None,
            command: Some(Command {
                title: "Suggest Improvements".to_string(),
                command: "jarvis.improve".to_string(),
                arguments: Some(vec![
                    serde_json::to_value(&params.text_document.uri).unwrap(),
                    serde_json::to_value(&params.range).unwrap(),
                ]),
            }),
            is_preferred: Some(false),
            disabled: None,
            data: None,
        }));

        actions.push(CodeActionOrCommand::CodeAction(CodeAction {
            title: "🔧 Jarvis: Fix Issues".to_string(),
            kind: Some(CodeActionKind::QUICK_FIX),
            diagnostics: Some(params.context.diagnostics),
            edit: None,
            command: Some(Command {
                title: "Fix Issues".to_string(),
                command: "jarvis.fix".to_string(),
                arguments: Some(vec![
                    serde_json::to_value(&params.text_document.uri).unwrap(),
                    serde_json::to_value(&params.range).unwrap(),
                ]),
            }),
            is_preferred: Some(true),
            disabled: None,
            data: None,
        }));

        Ok(Some(actions))
    }

    async fn execute_command(&self, params: ExecuteCommandParams) -> LspResult<Option<serde_json::Value>> {
        match params.command.as_str() {
            "jarvis.explain" => {
                // Extract parameters and call AI explanation
                self.client
                    .log_message(MessageType::INFO, "Jarvis explaining code...")
                    .await;
                // TODO: Implement actual explanation logic
            }
            "jarvis.improve" => {
                self.client
                    .log_message(MessageType::INFO, "Jarvis suggesting improvements...")
                    .await;
                // TODO: Implement improvement suggestions
            }
            "jarvis.fix" => {
                self.client
                    .log_message(MessageType::INFO, "Jarvis fixing issues...")
                    .await;
                // TODO: Implement issue fixing
            }
            "jarvis.generate" => {
                self.client
                    .log_message(MessageType::INFO, "Jarvis generating code...")
                    .await;
                // TODO: Implement code generation
            }
            _ => {
                self.client
                    .log_message(MessageType::ERROR, &format!("Unknown command: {}", params.command))
                    .await;
            }
        }

        Ok(None)
    }

    async fn completion(&self, params: CompletionParams) -> LspResult<Option<CompletionResponse>> {
        // AI-powered code completion
        let items = vec![
            CompletionItem {
                label: "jarvis_suggest".to_string(),
                kind: Some(CompletionItemKind::FUNCTION),
                detail: Some("AI-powered suggestion".to_string()),
                documentation: Some(Documentation::String("Get AI suggestions for code completion".to_string())),
                ..Default::default()
            }
        ];

        Ok(Some(CompletionResponse::Array(items)))
    }
}
