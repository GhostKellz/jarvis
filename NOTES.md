## 🔮 Jarvis: AI Copilot for Linux + Homelab
This is a seperate project!!!
Jarvis is envisioned as a local AI assistant deeply integrated with Linux systems (primarily Arch), Proxmox homelabs, and development environments like Neovim.

### Key Features:

* Built in **Rust** or possibly **Zig** (debated for performance vs safety)
* Uses **Ollama**, **Claude**, **OpenAI**, and Brave API
* Interacts with:

  * System health (Btrfs, Snapper, systemd)
  * Codebases (Rust, Zig, Shell)
  * Infrastructure tools (NGINX, Docker, WireGuard)
  * Web content/blogs for automated content updates
* Stores context and history in **zqlite** (Zig-based SQLite)
* Intended as a **CLI-first**, **LLM-driven assistant**

---
