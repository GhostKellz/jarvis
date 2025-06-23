# Jarvis Neovim Plugin

A powerful Neovim plugin that integrates Jarvis AI assistant directly into your editor, similar to Avante and Claude Code projects.

## 🚀 Features

- **AI-Powered Code Assistance**
  - Explain code selections with context-aware analysis
  - Suggest improvements and optimizations
  - Fix errors and issues automatically
  - Generate code from natural language descriptions

- **Interactive Chat Interface**
  - Floating chat window for conversations with Jarvis
  - Persistent chat history
  - Context-aware responses based on current file

- **LSP Integration**
  - Code actions powered by AI
  - Hover information with AI explanations
  - Smart completions

- **Language-Specific Features**
  - Rust: Memory safety analysis, performance optimization
  - Python: Pythonic code suggestions
  - JavaScript/TypeScript: Modern syntax recommendations
  - Go: Best practices enforcement

## 📦 Installation

### Using [lazy.nvim](https://github.com/folke/lazy.nvim)

```lua
{
  'jarvis-nvim/jarvis.nvim',
  config = function()
    require('jarvis').setup({
      -- Your configuration here
    })
  end,
  dependencies = {
    'nvim-lua/plenary.nvim',
    'MunifTanjim/nui.nvim',
  },
}
```

### Using [packer.nvim](https://github.com/wbthomason/packer.nvim)

```lua
use {
  'jarvis-nvim/jarvis.nvim',
  requires = {
    'nvim-lua/plenary.nvim',
    'MunifTanjim/nui.nvim',
  },
  config = function()
    require('jarvis').setup()
  end
}
```

## ⚙️ Configuration

```lua
require('jarvis').setup({
  -- Keymaps
  keymaps = {
    explain = '<leader>je',      -- Explain selected code
    improve = '<leader>ji',      -- Suggest improvements
    fix = '<leader>jf',          -- Fix errors
    chat = '<leader>jc',         -- Open chat interface
    generate = '<leader>jg',     -- Generate code
    line_explain = '<leader>jl', -- Explain current line
  },
  
  -- UI settings
  floating_window = {
    width = 0.8,
    height = 0.8,
    border = 'rounded',
  },
  
  -- Jarvis daemon settings
  jarvis = {
    socket_path = vim.fn.stdpath('data') .. '/jarvis.sock',
    auto_start = true,
    timeout = 30000,
  },
  
  -- LSP integration
  lsp = {
    enabled = true,
    code_actions = true,
    hover = true,
    completion = true,
  },
})
```

## 🎮 Usage

### Basic Commands

- `:JarvisExplain` - Explain selected code
- `:JarvisImprove` - Get improvement suggestions
- `:JarvisFix` - Fix errors in selection
- `:JarvisChat` - Open interactive chat
- `:JarvisGenerate <description>` - Generate code

### Default Keymaps

| Mode | Keymap | Action |
|------|--------|--------|
| Visual | `<leader>je` | Explain selection |
| Visual | `<leader>ji` | Improve selection |
| Visual | `<leader>jf` | Fix selection |
| Normal | `<leader>jc` | Open chat |
| Normal | `<leader>jg` | Generate code |
| Normal | `<leader>jl` | Explain current line |

### Code Actions

Right-click in your code or use `:lua vim.lsp.buf.code_action()` to see available Jarvis actions:

- 🤖 **Jarvis: Explain Code**
- 🚀 **Jarvis: Suggest Improvements**
- 🔧 **Jarvis: Fix Issues**
- 📝 **Jarvis: Add Comments**
- 🧪 **Jarvis: Generate Tests**
- ♻️ **Jarvis: Refactor Code**

### Language-Specific Actions

**Rust:**
- 🦀 **Jarvis: Optimize Rust Code**
- 🔒 **Jarvis: Check Memory Safety**

**Python:**
- 🐍 **Jarvis: Pythonic Improvements**

**JavaScript/TypeScript:**
- 📦 **Jarvis: Modern JS/TS**

**Go:**
- 🐹 **Jarvis: Go Best Practices**

## 🔧 Advanced Features

### Chat Interface

The chat interface provides a floating window where you can have extended conversations with Jarvis:

```
🤖 Jarvis AI Assistant
═══════════════════════

Welcome to Jarvis chat! Ask me anything about your code, system, or development.

Commands:
  • Type your message and press <Enter> to send
  • Use <Esc> or :q to close this window
  • Use <C-c> to cancel current input

────────────────────────────────────────────────────────────────────

> How can I optimize this Rust function?
🤖 Jarvis: Here are several ways to optimize your Rust function...

> 
```

### Context Awareness

Jarvis automatically provides context about:
- Current file type and name
- Line numbers and cursor position
- Git repository information
- Project structure
- Active LSP diagnostics

### Integration with Jarvis CLI

The plugin communicates with the main Jarvis CLI tool, ensuring consistent AI responses across your terminal and editor.

## 🛠️ Development

### Building from Source

```bash
# Clone the repository
git clone https://github.com/yourusername/jarvis
cd jarvis/jarvis-nvim

# Build the Rust components
cargo build --release

# Install the Lua plugin
mkdir -p ~/.local/share/nvim/site/pack/jarvis/start/
ln -s $(pwd) ~/.local/share/nvim/site/pack/jarvis/start/jarvis.nvim
```

### Architecture

The plugin consists of:

1. **Rust Library** (`src/lib.rs`) - Core functionality
2. **Neovim Client** (`src/nvim_client.rs`) - Neovim API integration
3. **AI Integration** (`src/ai_integration.rs`) - LLM communication
4. **LSP Server** (`src/lsp.rs`) - Language server implementation
5. **Lua Plugin** (`lua/jarvis/init.lua`) - Neovim interface

## 🤝 Contributing

Contributions are welcome! Please see our [Contributing Guide](../CONTRIBUTING.md) for details.

## 📄 License

MIT License - see [LICENSE](../LICENSE) for details.

## 🙏 Acknowledgments

Inspired by:
- [Avante.nvim](https://github.com/yetone/avante.nvim)
- [Claude Code](https://claude.ai/code)
- [Copilot.vim](https://github.com/github/copilot.vim)
