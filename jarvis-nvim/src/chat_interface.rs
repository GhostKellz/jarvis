use anyhow::Result;
use nvim_rs::{Neovim, Value};
use tokio::net::UnixStream;
use serde_json::json;
use std::sync::Arc;
use crate::ai_integration::AIIntegration;

pub struct ChatInterface {
    nvim: Neovim<UnixStream>,
    ai: Arc<AIIntegration>,
    chat_buffer: Option<i64>,
}

impl ChatInterface {
    pub fn new(nvim: Neovim<UnixStream>, ai: Arc<AIIntegration>) -> Self {
        Self {
            nvim,
            ai,
            chat_buffer: None,
        }
    }

    pub async fn open_chat_window(&mut self) -> Result<()> {
        // Create a new buffer for chat
        let buf = self.nvim.create_buf(false, true).await?;
        self.chat_buffer = Some(buf.id().await?);

        // Set buffer options
        buf.set_option("buftype", Value::from("nofile")).await?;
        buf.set_option("bufhidden", Value::from("hide")).await?;
        buf.set_option("swapfile", Value::from(false)).await?;
        buf.set_name("Jarvis Chat").await?;

        // Create floating window
        let lua_code = r#"
        local buf = ...
        local width = math.floor(vim.o.columns * 0.8)
        local height = math.floor(vim.o.lines * 0.8)
        
        local opts = {
            relative = 'editor',
            width = width,
            height = height,
            col = (vim.o.columns - width) / 2,
            row = (vim.o.lines - height) / 2,
            anchor = 'NW',
            style = 'minimal',
            border = 'rounded',
            title = ' 🤖 Jarvis Chat ',
            title_pos = 'center'
        }
        
        local win = vim.api.nvim_open_win(buf, true, opts)
        
        -- Set up chat interface
        local initial_content = {
            '🤖 Jarvis AI Assistant',
            '═══════════════════════',
            '',
            'Welcome to Jarvis chat! Ask me anything about your code, system, or development.',
            '',
            'Commands:',
            '  • Type your message and press Enter to send',
            '  • Use :q to close this window',
            '  • Use <C-c> to cancel current input',
            '',
            '────────────────────────────────────────────────────────────────────',
            '',
            '> '
        }
        
        vim.api.nvim_buf_set_lines(buf, 0, -1, false, initial_content)
        vim.api.nvim_win_set_cursor(win, {#initial_content, 2})
        
        -- Set up autocommands for chat interaction
        local group = vim.api.nvim_create_augroup('JarvisChat', { clear = true })
        
        vim.api.nvim_create_autocmd('InsertEnter', {
            group = group,
            buffer = buf,
            callback = function()
                local line = vim.api.nvim_get_current_line()
                if not line:match('^> ') then
                    vim.api.nvim_set_current_line('> ' .. line)
                    vim.api.nvim_win_set_cursor(0, {vim.fn.line('.'), vim.fn.col('$')})
                end
            end
        })
        
        return win
        "#;

        self.nvim.exec_lua(lua_code, vec![Value::from(buf.id().await?)]).await?;

        // Set up key mappings for chat
        self.setup_chat_keymaps().await?;

        Ok(())
    }

    async fn setup_chat_keymaps(&self) -> Result<()> {
        if let Some(buf_id) = self.chat_buffer {
            let lua_code = format!(r#"
            local buf = {}
            
            -- Enter key sends message
            vim.keymap.set('i', '<CR>', function()
                local line = vim.api.nvim_get_current_line()
                if line:match('^> ') and #line > 2 then
                    local message = line:sub(3)
                    -- Call Rust function to handle message
                    vim.api.nvim_buf_add_highlight(buf, -1, 'Comment', vim.fn.line('.') - 1, 0, -1)
                    vim.api.nvim_buf_set_lines(buf, vim.fn.line('.'), vim.fn.line('.'), false, {{'', '🤖 Thinking...', '', '> '}})
                    vim.api.nvim_win_set_cursor(0, {{vim.fn.line('.') + 3, 2}})
                    -- TODO: Call Rust message handler
                else
                    return '<CR>'
                end
            end, {{ buffer = buf, expr = true, desc = 'Send message to Jarvis' }})
            
            -- Escape key to normal mode
            vim.keymap.set('i', '<Esc>', '<Esc>', {{ buffer = buf }})
            
            -- Ctrl+C to cancel
            vim.keymap.set('i', '<C-c>', function()
                vim.api.nvim_set_current_line('> ')
                vim.api.nvim_win_set_cursor(0, {{vim.fn.line('.'), 2}})
            end, {{ buffer = buf, desc = 'Cancel current input' }})
            "#, buf_id);

            self.nvim.exec_lua(&lua_code, vec![]).await?;
        }

        Ok(())
    }

    pub async fn send_message(&mut self, message: &str) -> Result<()> {
        if let Some(buf_id) = self.chat_buffer {
            // Add user message to chat
            self.add_chat_message("👤 You", message, "Normal").await?;

            // Get AI response
            let response = self.ai.send_message(message, Some("Neovim chat session")).await?;

            // Add AI response to chat
            self.add_chat_message("🤖 Jarvis", &response, "String").await?;

            // Add new prompt line
            self.add_chat_prompt().await?;
        }

        Ok(())
    }

    async fn add_chat_message(&self, sender: &str, message: &str, highlight: &str) -> Result<()> {
        if let Some(buf_id) = self.chat_buffer {
            let buf = self.nvim.get_buf_from_id(buf_id).await?;
            
            let lines = vec![
                format!("{}: {}", sender, message),
                String::new(),
            ];

            let line_count = buf.line_count().await?;
            buf.set_lines(line_count, line_count, false, lines).await?;

            // Add syntax highlighting
            let lua_code = format!(r#"
            local buf = {}
            local line = vim.api.nvim_buf_line_count(buf) - 2
            vim.api.nvim_buf_add_highlight(buf, -1, '{}', line, 0, -1)
            "#, buf_id, highlight);

            self.nvim.exec_lua(&lua_code, vec![]).await?;
        }

        Ok(())
    }

    async fn add_chat_prompt(&self) -> Result<()> {
        if let Some(buf_id) = self.chat_buffer {
            let buf = self.nvim.get_buf_from_id(buf_id).await?;
            
            let line_count = buf.line_count().await?;
            buf.set_lines(line_count, line_count, false, vec!["> ".to_string()]).await?;

            // Move cursor to end of prompt
            let lua_code = format!(r#"
            local buf = {}
            local line = vim.api.nvim_buf_line_count(buf)
            vim.api.nvim_win_set_cursor(0, {{line, 2}})
            vim.cmd('startinsert!')
            "#, buf_id);

            self.nvim.exec_lua(&lua_code, vec![]).await?;
        }

        Ok(())
    }

    pub async fn close_chat(&mut self) -> Result<()> {
        if let Some(buf_id) = self.chat_buffer {
            let buf = self.nvim.get_buf_from_id(buf_id).await?;
            buf.detach().await?;
            self.chat_buffer = None;
        }

        Ok(())
    }

    pub async fn export_chat_history(&self) -> Result<String> {
        if let Some(buf_id) = self.chat_buffer {
            let buf = self.nvim.get_buf_from_id(buf_id).await?;
            let lines = buf.get_lines(0, -1, false).await?;
            Ok(lines.join("\n"))
        } else {
            Ok(String::new())
        }
    }
}
