-- Jarvis Neovim Plugin
-- Main entry point for the Jarvis AI assistant

local M = {}

-- Default configuration
local default_config = {
  -- Keymaps
  keymaps = {
    explain = '<leader>je',
    improve = '<leader>ji', 
    fix = '<leader>jf',
    chat = '<leader>jc',
    generate = '<leader>jg',
    line_explain = '<leader>jl',
  },
  
  -- UI settings
  floating_window = {
    width = 0.8,
    height = 0.8,
    border = 'rounded',
  },
  
  -- Jarvis settings
  jarvis = {
    socket_path = vim.fn.stdpath('data') .. '/jarvis.sock',
    auto_start = true,
    timeout = 30000, -- 30 seconds
  },
  
  -- LSP integration
  lsp = {
    enabled = true,
    code_actions = true,
    hover = true,
    completion = true,
  },
  
  -- Auto commands
  autocmds = {
    highlight_on_explain = true,
    save_chat_history = true,
  },
}

local config = {}
local jarvis_client = nil

-- Initialize Jarvis client
local function init_jarvis()
  if jarvis_client then
    return jarvis_client
  end
  
  -- This would connect to the Rust binary
  -- For now, we'll use a mock implementation
  jarvis_client = {
    explain = function(code, language, context)
      return vim.schedule_wrap(function()
        M.show_explanation("This is a mock explanation for: " .. (code or "selection"))
      end)()
    end,
    
    improve = function(code, language)
      return vim.schedule_wrap(function()
        M.show_improvements("Here are some mock improvements for your code")
      end)()
    end,
    
    fix = function(code, errors, language)
      return vim.schedule_wrap(function()
        M.show_fixes("Here are some mock fixes for your errors")
      end)()
    end,
    
    generate = function(description, language, context)
      return vim.schedule_wrap(function()
        M.insert_generated_code("-- Generated code for: " .. description)
      end)()
    end,
    
    chat = function(message)
      return "Mock response to: " .. message
    end,
  }
  
  return jarvis_client
end

-- Get visual selection
local function get_visual_selection()
  local start_pos = vim.fn.getpos("'<")
  local end_pos = vim.fn.getpos("'>")
  
  if start_pos[2] == 0 or end_pos[2] == 0 then
    return nil
  end
  
  local lines = vim.api.nvim_buf_get_lines(0, start_pos[2] - 1, end_pos[2], false)
  
  if #lines == 0 then
    return nil
  end
  
  -- Handle partial line selections
  if #lines == 1 then
    lines[1] = string.sub(lines[1], start_pos[3], end_pos[3])
  else
    lines[1] = string.sub(lines[1], start_pos[3])
    lines[#lines] = string.sub(lines[#lines], 1, end_pos[3])
  end
  
  return table.concat(lines, '\n')
end

-- Get file context
local function get_file_context()
  local filetype = vim.bo.filetype
  local filename = vim.fn.expand('%:t')
  local line_count = vim.api.nvim_buf_line_count(0)
  local cursor_pos = vim.api.nvim_win_get_cursor(0)
  
  return {
    filename = filename,
    filetype = filetype,
    line_count = line_count,
    cursor_line = cursor_pos[1],
    cursor_col = cursor_pos[2],
  }
end

-- Show floating window
function M.show_floating_window(title, content, opts)
  opts = opts or {}
  local width = math.floor(vim.o.columns * (opts.width or config.floating_window.width))
  local height = math.floor(vim.o.lines * (opts.height or config.floating_window.height))
  
  local buf = vim.api.nvim_create_buf(false, true)
  
  -- Set content
  local lines = vim.split(content, '\n')
  vim.api.nvim_buf_set_lines(buf, 0, -1, false, lines)
  
  -- Create window
  local win_opts = {
    relative = 'editor',
    width = width,
    height = height,
    col = math.floor((vim.o.columns - width) / 2),
    row = math.floor((vim.o.lines - height) / 2),
    anchor = 'NW',
    style = 'minimal',
    border = config.floating_window.border,
    title = title,
    title_pos = 'center',
  }
  
  local win = vim.api.nvim_open_win(buf, true, win_opts)
  
  -- Set buffer options
  vim.api.nvim_buf_set_option(buf, 'modifiable', false)
  vim.api.nvim_buf_set_option(buf, 'filetype', 'markdown')
  
  -- Close on escape or q
  vim.keymap.set('n', '<Esc>', '<cmd>close<cr>', { buffer = buf, silent = true })
  vim.keymap.set('n', 'q', '<cmd>close<cr>', { buffer = buf, silent = true })
  
  return buf, win
end

-- Show explanation
function M.show_explanation(explanation)
  M.show_floating_window('🤖 Jarvis Explanation', explanation)
end

-- Show improvements
function M.show_improvements(improvements)
  M.show_floating_window('🚀 Jarvis Improvements', improvements)
end

-- Show fixes
function M.show_fixes(fixes)
  M.show_floating_window('🔧 Jarvis Fixes', fixes)
end

-- Insert generated code
function M.insert_generated_code(code)
  local lines = vim.split(code, '\n')
  local cursor_pos = vim.api.nvim_win_get_cursor(0)
  vim.api.nvim_buf_set_lines(0, cursor_pos[1], cursor_pos[1], false, lines)
end

-- Main functions
function M.explain()
  local selection = get_visual_selection()
  local context = get_file_context()
  
  if not selection then
    vim.notify('No text selected', vim.log.levels.WARN)
    return
  end
  
  local client = init_jarvis()
  client.explain(selection, context.filetype, vim.inspect(context))
end

function M.improve()
  local selection = get_visual_selection()
  local context = get_file_context()
  
  if not selection then
    vim.notify('No text selected', vim.log.levels.WARN)
    return
  end
  
  local client = init_jarvis()
  client.improve(selection, context.filetype)
end

function M.fix()
  local selection = get_visual_selection()
  local context = get_file_context()
  
  if not selection then
    vim.notify('No text selected', vim.log.levels.WARN)
    return
  end
  
  -- Get diagnostics
  local diagnostics = vim.diagnostic.get(0)
  local errors = {}
  for _, diag in ipairs(diagnostics) do
    table.insert(errors, diag.message)
  end
  
  local client = init_jarvis()
  client.fix(selection, errors, context.filetype)
end

function M.generate()
  local description = vim.fn.input('Generate code for: ')
  if description == '' then
    return
  end
  
  local context = get_file_context()
  local client = init_jarvis()
  client.generate(description, context.filetype, vim.inspect(context))
end

function M.explain_line()
  -- Select current line and explain
  local line = vim.api.nvim_get_current_line()
  local context = get_file_context()
  
  local client = init_jarvis()
  client.explain(line, context.filetype, vim.inspect(context))
end

-- Chat interface
local chat_buf = nil
local chat_win = nil

function M.open_chat()
  if chat_buf and vim.api.nvim_buf_is_valid(chat_buf) then
    if chat_win and vim.api.nvim_win_is_valid(chat_win) then
      vim.api.nvim_set_current_win(chat_win)
      return
    end
  end
  
  -- Create chat buffer
  chat_buf = vim.api.nvim_create_buf(false, true)
  vim.api.nvim_buf_set_name(chat_buf, 'Jarvis Chat')
  
  -- Set up chat window
  local width = math.floor(vim.o.columns * 0.8)
  local height = math.floor(vim.o.lines * 0.8)
  
  local win_opts = {
    relative = 'editor',
    width = width,
    height = height,
    col = math.floor((vim.o.columns - width) / 2),
    row = math.floor((vim.o.lines - height) / 2),
    anchor = 'NW',
    style = 'minimal',
    border = 'rounded',
    title = ' 🤖 Jarvis Chat ',
    title_pos = 'center',
  }
  
  chat_win = vim.api.nvim_open_win(chat_buf, true, win_opts)
  
  -- Initialize chat
  local initial_content = {
    '🤖 Jarvis AI Assistant',
    '═══════════════════════',
    '',
    'Welcome to Jarvis chat! Ask me anything about your code, system, or development.',
    '',
    'Commands:',
    '  • Type your message and press <Enter> to send',
    '  • Use <Esc> or :q to close this window',
    '  • Use <C-c> to cancel current input',
    '',
    '────────────────────────────────────────────────────────────────────',
    '',
    '> ',
  }
  
  vim.api.nvim_buf_set_lines(chat_buf, 0, -1, false, initial_content)
  vim.api.nvim_win_set_cursor(chat_win, {#initial_content, 2})
  
  -- Set up chat keymaps
  vim.keymap.set('i', '<CR>', function()
    local line = vim.api.nvim_get_current_line()
    if line:match('^> ') and #line > 2 then
      local message = line:sub(3)
      M.send_chat_message(message)
    else
      return '<CR>'
    end
  end, { buffer = chat_buf, expr = true })
  
  vim.keymap.set('n', '<Esc>', '<cmd>close<cr>', { buffer = chat_buf })
  vim.keymap.set('n', 'q', '<cmd>close<cr>', { buffer = chat_buf })
  
  vim.keymap.set('i', '<C-c>', function()
    vim.api.nvim_set_current_line('> ')
    vim.api.nvim_win_set_cursor(0, {vim.fn.line('.'), 2})
  end, { buffer = chat_buf })
  
  vim.cmd('startinsert')
end

function M.send_chat_message(message)
  if not chat_buf or not vim.api.nvim_buf_is_valid(chat_buf) then
    return
  end
  
  -- Add user message
  local user_line = '👤 You: ' .. message
  local line_count = vim.api.nvim_buf_line_count(chat_buf)
  vim.api.nvim_buf_set_lines(chat_buf, line_count - 1, line_count - 1, false, {user_line, ''})
  
  -- Show thinking indicator
  vim.api.nvim_buf_set_lines(chat_buf, -1, -1, false, {'🤖 Jarvis: Thinking...', ''})
  
  -- Get AI response (mock for now)
  local client = init_jarvis()
  local response = client.chat(message)
  
  -- Replace thinking indicator with actual response
  local new_line_count = vim.api.nvim_buf_line_count(chat_buf)
  vim.api.nvim_buf_set_lines(chat_buf, new_line_count - 2, new_line_count - 1, false, {'🤖 Jarvis: ' .. response, ''})
  
  -- Add new prompt
  vim.api.nvim_buf_set_lines(chat_buf, -1, -1, false, {'> '})
  
  -- Move cursor to new prompt
  local final_line_count = vim.api.nvim_buf_line_count(chat_buf)
  vim.api.nvim_win_set_cursor(chat_win, {final_line_count, 2})
  vim.cmd('startinsert')
end

-- Setup function
function M.setup(user_config)
  config = vim.tbl_deep_extend('force', default_config, user_config or {})
  
  -- Create user commands
  vim.api.nvim_create_user_command('JarvisExplain', M.explain, { desc = 'Explain selected code with Jarvis' })
  vim.api.nvim_create_user_command('JarvisImprove', M.improve, { desc = 'Get improvement suggestions from Jarvis' })
  vim.api.nvim_create_user_command('JarvisFix', M.fix, { desc = 'Fix errors with Jarvis' })
  vim.api.nvim_create_user_command('JarvisChat', M.open_chat, { desc = 'Open Jarvis chat interface' })
  vim.api.nvim_create_user_command('JarvisGenerate', function(opts)
    if opts.args and opts.args ~= '' then
      local context = get_file_context()
      local client = init_jarvis()
      client.generate(opts.args, context.filetype, vim.inspect(context))
    else
      M.generate()
    end
  end, { nargs = '*', desc = 'Generate code with Jarvis' })
  
  -- Set up keymaps
  if config.keymaps then
    local opts = { noremap = true, silent = true }
    
    -- Visual mode mappings
    if config.keymaps.explain then
      vim.keymap.set('v', config.keymaps.explain, '<cmd>JarvisExplain<cr>', 
        vim.tbl_extend('force', opts, { desc = 'Jarvis: Explain selection' }))
    end
    
    if config.keymaps.improve then
      vim.keymap.set('v', config.keymaps.improve, '<cmd>JarvisImprove<cr>', 
        vim.tbl_extend('force', opts, { desc = 'Jarvis: Improve selection' }))
    end
    
    if config.keymaps.fix then
      vim.keymap.set('v', config.keymaps.fix, '<cmd>JarvisFix<cr>', 
        vim.tbl_extend('force', opts, { desc = 'Jarvis: Fix selection' }))
    end
    
    -- Normal mode mappings
    if config.keymaps.chat then
      vim.keymap.set('n', config.keymaps.chat, '<cmd>JarvisChat<cr>', 
        vim.tbl_extend('force', opts, { desc = 'Jarvis: Open chat' }))
    end
    
    if config.keymaps.generate then
      vim.keymap.set('n', config.keymaps.generate, M.generate, 
        vim.tbl_extend('force', opts, { desc = 'Jarvis: Generate code' }))
    end
    
    if config.keymaps.line_explain then
      vim.keymap.set('n', config.keymaps.line_explain, M.explain_line, 
        vim.tbl_extend('force', opts, { desc = 'Jarvis: Explain current line' }))
    end
  end
  
  -- Set up autocommands
  local group = vim.api.nvim_create_augroup('Jarvis', { clear = true })
  
  vim.api.nvim_create_autocmd('VimLeavePre', {
    group = group,
    callback = function()
      -- Clean up resources
      if chat_buf and vim.api.nvim_buf_is_valid(chat_buf) then
        vim.api.nvim_buf_delete(chat_buf, { force = true })
      end
    end,
  })
  
  vim.notify('🤖 Jarvis plugin loaded successfully!', vim.log.levels.INFO)
end

return M
