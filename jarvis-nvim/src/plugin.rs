use crate::{AIIntegration, JarvisNvim};
use anyhow::Result;
use mlua::{Function, Lua, Table, UserData, UserDataMethods};
use std::sync::Arc;

pub struct Plugin {
    jarvis: Arc<JarvisNvim>,
    ai: Arc<AIIntegration>,
}

impl Plugin {
    pub fn new(jarvis: Arc<JarvisNvim>, ai: Arc<AIIntegration>) -> Self {
        Self { jarvis, ai }
    }

    pub fn register_lua_module(&self, lua: &Lua) -> Result<()> {
        let jarvis_module = lua.create_table()?;

        // Register main functions
        let jarvis_clone = self.jarvis.clone();
        let explain_fn = lua.create_async_function(move |_, ()| {
            let jarvis = jarvis_clone.clone();
            async move {
                jarvis
                    .explain_selection()
                    .await
                    .map_err(mlua::Error::external)
            }
        })?;
        jarvis_module.set("explain", explain_fn)?;

        let jarvis_clone = self.jarvis.clone();
        let improve_fn = lua.create_async_function(move |_, ()| {
            let jarvis = jarvis_clone.clone();
            async move {
                jarvis
                    .suggest_improvements()
                    .await
                    .map_err(mlua::Error::external)
            }
        })?;
        jarvis_module.set("improve", improve_fn)?;

        let jarvis_clone = self.jarvis.clone();
        let fix_fn = lua.create_async_function(move |_, ()| {
            let jarvis = jarvis_clone.clone();
            async move { jarvis.fix_errors().await.map_err(mlua::Error::external) }
        })?;
        jarvis_module.set("fix", fix_fn)?;

        let jarvis_clone = self.jarvis.clone();
        let chat_fn = lua.create_async_function(move |_, ()| {
            let jarvis = jarvis_clone.clone();
            async move { jarvis.chat_mode().await.map_err(mlua::Error::external) }
        })?;
        jarvis_module.set("chat", chat_fn)?;

        let jarvis_clone = self.jarvis.clone();
        let generate_fn = lua.create_async_function(move |_, description: String| {
            let jarvis = jarvis_clone.clone();
            async move {
                jarvis
                    .generate_code(&description)
                    .await
                    .map_err(mlua::Error::external)
            }
        })?;
        jarvis_module.set("generate", generate_fn)?;

        // Register AI functions
        let ai_clone = self.ai.clone();
        let ai_explain_fn = lua.create_async_function(
            move |_, (code, language, context): (String, String, String)| {
                let ai = ai_clone.clone();
                async move {
                    ai.explain_code(&code, &language, &context)
                        .await
                        .map_err(mlua::Error::external)
                }
            },
        )?;
        jarvis_module.set("ai_explain", ai_explain_fn)?;

        let ai_clone = self.ai.clone();
        let ai_improve_fn =
            lua.create_async_function(move |_, (code, language): (String, String)| {
                let ai = ai_clone.clone();
                async move {
                    ai.suggest_improvements(&code, &language)
                        .await
                        .map_err(mlua::Error::external)
                }
            })?;
        jarvis_module.set("ai_improve", ai_improve_fn)?;

        // Register the module globally
        lua.globals().set("jarvis", jarvis_module)?;

        Ok(())
    }

    pub fn get_lua_setup_script(&self) -> &'static str {
        r#"
-- Jarvis Neovim Plugin Setup
local jarvis = require('jarvis')

-- Create user commands
vim.api.nvim_create_user_command('JarvisExplain', function()
    jarvis.explain()
end, { desc = 'Explain selected code with Jarvis' })

vim.api.nvim_create_user_command('JarvisImprove', function()
    jarvis.improve()
end, { desc = 'Get improvement suggestions from Jarvis' })

vim.api.nvim_create_user_command('JarvisFix', function()
    jarvis.fix()
end, { desc = 'Fix errors with Jarvis' })

vim.api.nvim_create_user_command('JarvisChat', function()
    jarvis.chat()
end, { desc = 'Open Jarvis chat interface' })

vim.api.nvim_create_user_command('JarvisGenerate', function(opts)
    jarvis.generate(opts.args)
end, { nargs = '*', desc = 'Generate code with Jarvis' })

-- Default keymaps
local function setup_keymaps()
    local opts = { noremap = true, silent = true }
    
    -- Visual mode mappings
    vim.keymap.set('v', '<leader>je', '<cmd>JarvisExplain<cr>', vim.tbl_extend('force', opts, { desc = 'Jarvis: Explain selection' }))
    vim.keymap.set('v', '<leader>ji', '<cmd>JarvisImprove<cr>', vim.tbl_extend('force', opts, { desc = 'Jarvis: Improve selection' }))
    vim.keymap.set('v', '<leader>jf', '<cmd>JarvisFix<cr>', vim.tbl_extend('force', opts, { desc = 'Jarvis: Fix selection' }))
    
    -- Normal mode mappings
    vim.keymap.set('n', '<leader>jc', '<cmd>JarvisChat<cr>', vim.tbl_extend('force', opts, { desc = 'Jarvis: Open chat' }))
    vim.keymap.set('n', '<leader>jg', function()
        local input = vim.fn.input('Generate code for: ')
        if input ~= '' then
            vim.cmd('JarvisGenerate ' .. input)
        end
    end, vim.tbl_extend('force', opts, { desc = 'Jarvis: Generate code' }))
    
    -- Line-specific mappings
    vim.keymap.set('n', '<leader>jl', function()
        -- Select current line and explain
        vim.cmd('normal! V')
        vim.cmd('JarvisExplain')
    end, vim.tbl_extend('force', opts, { desc = 'Jarvis: Explain current line' }))
end

-- Setup function for users to customize
local function setup(user_config)
    user_config = user_config or {}
    
    -- Set up keymaps if not disabled
    if user_config.keymaps ~= false then
        setup_keymaps()
    end
    
    -- Custom keymaps
    if user_config.custom_keymaps then
        for mode, mappings in pairs(user_config.custom_keymaps) do
            for lhs, rhs in pairs(mappings) do
                vim.keymap.set(mode, lhs, rhs, { noremap = true, silent = true })
            end
        end
    end
    
    -- Auto commands
    vim.api.nvim_create_augroup('Jarvis', { clear = true })
    
    -- Show Jarvis status in statusline
    if user_config.statusline ~= false then
        vim.api.nvim_create_autocmd('BufEnter', {
            group = 'Jarvis',
            callback = function()
                vim.g.jarvis_ready = true
            end
        })
    end
end

-- Export setup function
return {
    setup = setup,
    jarvis = jarvis
}
"#
    }
}

impl UserData for Plugin {
    fn add_methods<'lua, M: UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_async_method("explain", |_, this, ()| async move {
            this.jarvis
                .explain_selection()
                .await
                .map_err(mlua::Error::external)
        });

        methods.add_async_method("improve", |_, this, ()| async move {
            this.jarvis
                .suggest_improvements()
                .await
                .map_err(mlua::Error::external)
        });

        methods.add_async_method("fix", |_, this, ()| async move {
            this.jarvis
                .fix_errors()
                .await
                .map_err(mlua::Error::external)
        });

        methods.add_async_method("chat", |_, this, ()| async move {
            this.jarvis.chat_mode().await.map_err(mlua::Error::external)
        });

        methods.add_async_method("generate", |_, this, description: String| async move {
            this.jarvis
                .generate_code(&description)
                .await
                .map_err(mlua::Error::external)
        });
    }
}
