# 🤝 CLAUDE.md — Claude/LLM/Agent Integrations for Jarvis

A running log for how Jarvis should leverage Claude and other LLMs as copilots, memory engines, and skill providers.

---

## Claude as Agent Copilot

* Claude (via API) as a first-class backend for all agent chat, code, audit, and workflow tasks
* Skill: Claude-powered code review, PR generation, doc authoring
* Skill: Summarize audit logs, suggest next steps, answer "what broke?"
* “Context window extender”: Jarvis streams memory/logs to Claude for analysis

## Multi-Model Integration

* Jarvis can switch between Claude, OpenAI, Ollama, local LLMs, Gemini, etc. on demand
* Agent decides: best model for code, chat, data science, audit
* Seamless context sharing (persistent + ephemeral)

## Memory & Chain-of-Thought

* Use Claude for large context reasoning and explanations
* Jarvis keeps persistent memory, sends relevant slices to Claude
* Claude outputs actionable plans/PRs/scripts, Jarvis executes

## Security & Trust

* All Claude actions auditable: log, sign, store on GhostChain if needed
* Claude can propose but not execute actions without Jarvis/owner approval ("human-in-the-loop")
* Claude can verify/check Jarvis’ outputs and vice versa

## Future Integrations

* Claude-code in NVim, VSCode, or browser
* Multi-agent: Claude and Jarvis coordinate, check each other, run tasks in parallel
* Claude as part of Jarvis’ team: e.g., have Claude handle code gen, Jarvis handle infra, Gemini do data science

---

*This document guides how Claude and other LLMs will empower Jarvis as a persistent, agentic supercopilot.*
