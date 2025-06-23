-- Jarvis Neovim Plugin Configuration
-- Example configuration file

return {
  'jarvis-nvim/jarvis.nvim',
  config = function()
    require('jarvis').setup({
      -- Keymaps configuration
      keymaps = {
        explain = '<leader>je',      -- Explain code
        improve = '<leader>ji',      -- Improve code  
        fix = '<leader>jf',          -- Fix errors
        chat = '<leader>jc',         -- Open chat
        generate = '<leader>jg',     -- Generate code
        line_explain = '<leader>jl', -- Explain current line
      },
      
      -- UI settings
      floating_window = {
        width = 0.8,     -- 80% of screen width
        height = 0.8,    -- 80% of screen height
        border = 'rounded', -- Window border style
      },
      
      -- Jarvis connection settings
      jarvis = {
        socket_path = vim.fn.stdpath('data') .. '/jarvis.sock',
        auto_start = true,  -- Auto-start Jarvis daemon
        timeout = 30000,    -- Connection timeout in ms
      },
      
      -- LSP integration
      lsp = {
        enabled = true,       -- Enable LSP features
        code_actions = true,  -- Show Jarvis code actions
        hover = true,         -- AI-powered hover
        completion = true,    -- AI-powered completion
      },
      
      -- Auto commands
      autocmds = {
        highlight_on_explain = true,  -- Highlight explained code
        save_chat_history = true,     -- Save chat sessions
      },
    })
  end,
  
  -- Optional: lazy loading
  event = 'VeryLazy',
  
  -- Dependencies
  dependencies = {
    'nvim-lua/plenary.nvim',
    'MunifTanjim/nui.nvim',
  },
}
