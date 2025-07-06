# TODO.md – Jarvis-NV Roadmap

## ✅ Core Status

* [x] CLI compiles successfully
* [x] Async runtime functional
* [x] Build + check successful
* [x] Config system initialized
* [x] Ready for Docker containerization

---

## 🔌 1. GPU Integration (CUDA/NVML Monitoring)

> 🎯 Goal: Use `nvidia-smi` or NVML bindings to pull GPU stats in real-time

* [ ] Add `nvml-wrapper` or `cuda-sys` crate
* [ ] Build a `GpuMonitor` struct:

  * [ ] Polls GPU usage, temperature, VRAM, power
  * [ ] Exposes gRPC/REST endpoint (optional)
* [ ] Log or emit Prometheus/OpenMetrics-compatible output

---

## 🔗 2. GhostChain + ZVM Node Integration

> 🎯 Goal: Make `jarvis-nv` monitor/control ghostd or ghostnode

* [ ] Connect to `ghostd` RPC or WebSocket interface
* [ ] Pull:

  * [ ] Block height
  * [ ] Mempool state
  * [ ] Peer connections
* [ ] Enable:

  * [ ] Node restart / soft reload
  * [ ] Hot reload of node config
  * [ ] CLI-triggered commands

---

## 🧠 3. AI Agent + LLM Integration

> 🎯 Goal: Enable GPU-powered LLM ops via Ollama, Claude, or vLLM

* [ ] Launch Ollama or HF models
* [ ] Handle prompt routing via `jarvis-core`
* [ ] Add task hooks:

  * [ ] Blockchain analytics
  * [ ] Diagnostic helpers
  * [ ] Remediation planners

---

## 📊 4. Observability: Metrics + Logging

> 🎯 Goal: Prometheus-exportable metrics and logs

* [ ] Add `prometheus_client` crate
* [ ] Expose `/metrics` HTTP endpoint (QUIC-compatible)
* [ ] Export:

  * [ ] GPU stats
  * [ ] Node telemetry
  * [ ] LLM task events

---

## 🧪 5. Testnet Node Orchestrator Mode

> 🎯 Goal: Let `jarvis-nv` deploy and manage blockchain testnets

* [ ] Launch nodes via:

  * [ ] Docker Compose
  * [ ] Podman
  * [ ] Systemd or Proxmox API
* [ ] Healthcheck loop
* [ ] Snapshot rotation + rollback

---

## 📁 Suggested Structure

```
jarvis-nv/
├── src/
│   ├── main.rs
│   ├── gpu/monitor.rs
│   ├── node/rpc.rs
│   ├── ai/ollama.rs
│   ├── metrics.rs
│   ├── config.rs
│   └── agent.rs
├── Dockerfile
├── README.md
└── JARVIS_NV_TODO.md
```

---

## ✨ Future Ideas

* [ ] GPU-based malicious peer detection
* [ ] Live network map w/ real-time stats
* [ ] zk-SNARK proof acceleration via GPU
* [ ] GhostPlane testnet flood tools
* [ ] GPU workload scheduler across VMs

---

**Next step?** Pick any of the above, and we’ll scaffold it.
