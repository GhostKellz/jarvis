-- Plugin specification for plugin managers
return {
  name = "jarvis.nvim",
  description = "AI-powered coding assistant for Neovim",
  version = "0.1.0",
  author = "Christopher Kelley <ckelley@ghostkellz.sh>",
  license = "MIT",
  
  dependencies = {
    "nvim-lua/plenary.nvim",
    "MunifTanjim/nui.nvim", -- Optional but recommended for better UI
  },
  
  requirements = {
    nvim = "0.8.0", -- Minimum Neovim version
  },
  
  config = {
    -- Default configuration
    keymaps = {
      explain = "<leader>je",
      improve = "<leader>ji", 
      fix = "<leader>jf",
      chat = "<leader>jc",
      generate = "<leader>jg",
      line_explain = "<leader>jl",
    },
    
    floating_window = {
      width = 0.8,
      height = 0.8,
      border = "rounded",
    },
    
    jarvis = {
      socket_path = vim.fn.stdpath('data') .. '/jarvis.sock',
      auto_start = true,
      timeout = 30000,
    },
    
    lsp = {
      enabled = true,
      code_actions = true,
      hover = true,
      completion = true,
    },
  },
}
