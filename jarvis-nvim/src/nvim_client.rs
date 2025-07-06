use anyhow::Result;
use jarvis_agent::AgentRunner;
use jarvis_core::{LLMRouter, MemoryStore};
use nvim_rs::{Neovim, create::tokio as create};
use serde_json::Value;
use std::sync::Arc;
use tokio::net::UnixStream;

pub struct JarvisNvim {
    nvim: Neovim<UnixStream>,
    agent: Arc<AgentRunner>,
    llm: Arc<LLMRouter>,
    memory: Arc<MemoryStore>,
}

impl JarvisNvim {
    pub async fn new(socket_path: &str) -> Result<Self> {
        // Connect to Neovim
        let stream = UnixStream::connect(socket_path).await?;
        let (nvim, io_handler) = create::new_unix(stream);

        // Spawn the IO handler
        tokio::spawn(io_handler);

        // Initialize Jarvis components
        let config = jarvis_core::Config::load(None).await?;
        let memory = Arc::new(MemoryStore::new(&config.database_path).await?);
        let llm = Arc::new(LLMRouter::new(&config).await?);
        let agent = Arc::new(AgentRunner::new(memory.clone(), llm.clone()).await?);

        Ok(Self {
            nvim,
            agent,
            llm,
            memory,
        })
    }

    pub async fn explain_selection(&self) -> Result<()> {
        // Get current visual selection
        let selection = self.get_visual_selection().await?;
        if selection.is_empty() {
            return Ok(());
        }

        // Get file context
        let context = self.get_file_context().await?;

        // Generate explanation
        let prompt = format!(
            "Explain this code selection:\n\n```\n{}\n```\n\nFile context:\n{}",
            selection, context
        );

        let response = self.llm.generate(&prompt, None).await?;

        // Display in floating window
        self.show_floating_window("Jarvis Explanation", &response)
            .await?;

        Ok(())
    }

    pub async fn suggest_improvements(&self) -> Result<()> {
        let selection = self.get_visual_selection().await?;
        let context = self.get_file_context().await?;

        let prompt = format!(
            "Suggest improvements for this code:\n\n```\n{}\n```\n\nContext:\n{}",
            selection, context
        );

        let response = self.llm.generate(&prompt, None).await?;
        self.show_floating_window("Jarvis Suggestions", &response)
            .await?;

        Ok(())
    }

    pub async fn fix_errors(&self) -> Result<()> {
        // Get diagnostics from LSP
        let diagnostics = self.get_diagnostics().await?;
        if diagnostics.is_empty() {
            self.nvim
                .echo(&[("No errors found!", None)], false, &Value::Null)
                .await?;
            return Ok(());
        }

        let current_line = self.get_current_line().await?;
        let file_content = self.get_buffer_content().await?;

        let prompt = format!(
            "Fix these errors in the code:\n\nErrors:\n{}\n\nCurrent line: {}\n\nFile content:\n{}",
            diagnostics.join("\n"),
            current_line,
            file_content
        );

        let response = self.llm.generate(&prompt, None).await?;
        self.show_floating_window("Jarvis Error Fix", &response)
            .await?;

        Ok(())
    }

    pub async fn chat_mode(&self) -> Result<()> {
        // Open chat interface in split window
        self.nvim.command("vsplit").await?;
        self.nvim.command("enew").await?;
        self.nvim.command("setlocal buftype=nofile").await?;
        self.nvim.command("setlocal bufhidden=hide").await?;
        self.nvim.command("setlocal noswapfile").await?;
        self.nvim.command("file Jarvis\\ Chat").await?;

        // Set up chat buffer with initial message
        let initial_content = vec![
            "🤖 Jarvis Chat Mode".to_string(),
            "==================".to_string(),
            "".to_string(),
            "Type your questions and press <leader>js to send".to_string(),
            "".to_string(),
            "> ".to_string(),
        ];

        self.nvim
            .set_current_line(&initial_content.join("\n"))
            .await?;

        Ok(())
    }

    pub async fn generate_code(&self, description: &str) -> Result<()> {
        let file_type = self.get_current_filetype().await?;
        let context = self.get_file_context().await?;

        let prompt = format!(
            "Generate {} code for: {}\n\nContext:\n{}",
            file_type, description, context
        );

        let response = self.llm.generate(&prompt, None).await?;

        // Insert generated code at cursor
        self.insert_at_cursor(&response).await?;

        Ok(())
    }

    async fn get_visual_selection(&self) -> Result<String> {
        // Get visual selection using Neovim API
        let result = self.nvim.eval("getline(\"'<\", \"'>\")").await?;
        if let Value::Array(lines) = result {
            let text_lines: Vec<String> = lines
                .into_iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect();
            Ok(text_lines.join("\n"))
        } else {
            Ok(String::new())
        }
    }

    async fn get_file_context(&self) -> Result<String> {
        let filename = self.nvim.eval("expand('%:t')").await?;
        let filetype = self.nvim.eval("&filetype").await?;
        let line_count = self.nvim.eval("line('$')").await?;

        Ok(format!(
            "File: {}\nType: {}\nLines: {}",
            filename.as_str().unwrap_or("unknown"),
            filetype.as_str().unwrap_or("unknown"),
            line_count.as_i64().unwrap_or(0)
        ))
    }

    async fn get_current_line(&self) -> Result<String> {
        let line = self.nvim.get_current_line().await?;
        Ok(line)
    }

    async fn get_buffer_content(&self) -> Result<String> {
        let lines = self.nvim.get_current_buf().get_lines(0, -1, false).await?;
        Ok(lines.join("\n"))
    }

    async fn get_current_filetype(&self) -> Result<String> {
        let filetype = self.nvim.eval("&filetype").await?;
        Ok(filetype.as_str().unwrap_or("text").to_string())
    }

    async fn get_diagnostics(&self) -> Result<Vec<String>> {
        // This would integrate with LSP diagnostics
        // For now, return empty vector
        Ok(vec![])
    }

    async fn show_floating_window(&self, title: &str, content: &str) -> Result<()> {
        // Create floating window with content
        let lua_code = format!(
            r#"
            local buf = vim.api.nvim_create_buf(false, true)
            local lines = vim.split({:?}, '\n')
            vim.api.nvim_buf_set_lines(buf, 0, -1, false, lines)
            
            local width = math.min(80, vim.o.columns - 4)
            local height = math.min(20, #lines + 2)
            
            local opts = {{
                relative = 'editor',
                width = width,
                height = height,
                col = (vim.o.columns - width) / 2,
                row = (vim.o.lines - height) / 2,
                anchor = 'NW',
                style = 'minimal',
                border = 'rounded',
                title = {:?},
                title_pos = 'center'
            }}
            
            local win = vim.api.nvim_open_win(buf, true, opts)
            vim.api.nvim_buf_set_option(buf, 'modifiable', false)
            vim.api.nvim_buf_set_option(buf, 'filetype', 'markdown')
            "#,
            content, title
        );

        self.nvim.exec_lua(&lua_code, vec![]).await?;
        Ok(())
    }

    async fn insert_at_cursor(&self, text: &str) -> Result<()> {
        let lines: Vec<&str> = text.lines().collect();
        self.nvim.put(&lines, "l", true, true).await?;
        Ok(())
    }
}
