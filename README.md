# ✨ Aura SDK (aura-sdk)

[![CI](https://github.com/Maki-Grz/aura-sdk/actions/workflows/ci.yml/badge.svg)](https://github.com/Maki-Grz/aura-sdk/actions/workflows/ci.yml)
[![Version](https://img.shields.io/badge/version-0.1.0-purple.svg)](Cargo.toml)
[![Platform](https://img.shields.io/badge/platform-Windows--ARM64-red.svg)](https://learn.microsoft.com/en-us/windows/arm/)
[![NPU](https://img.shields.io/badge/accelerator-Qualcomm--Hexagon-orange.svg)](https://www.qualcomm.com/products/technology/processors/snapdragon-x-plus)

Aura SDK is a high-performance AI inference engine for **Snapdragon X Elite / Plus** platforms.  
It supports two execution modes:

- **Native Genie Engine** – Uses the Qualcomm NPU (Hexagon) for best performance, requires models in Genie binary format.
- **Aura Engine (ORT)** – ONNX Runtime backend, **CPU only** (ideal for model prototyping or when NPU compatibility is not required).

---

## ⚙️ Prerequisites & Drivers

- **NPU Driver**: Version **30.0.140.x** or higher (check in Device Manager).
- **⚠️ CRITICAL**: Disable **Memory Integrity** in *Windows Security > Device Security > Core Isolation* and restart.
- **QAIRT SDK**: Install [Qualcomm AI Stack](https://softwarecenter.qualcomm.com/catalog/catalog-suite/Qualcomm%C2%AE%20AI%20Stack) (v2.45.40 recommended).
- **Environment variables** (optional, the SDK will try to set them automatically):
  - `QNN_SDK_ROOT` – path to QAIRT installation.
  - `ADSP_LIBRARY_PATH` – path to Hexagon unsigned libraries (e.g. `...\lib\hexagon-v73\unsigned`).

---

## 🛠️ Build

```powershell
# Build native engine only (Genie NPU mode)
cargo build --release

# Build with ONNX Runtime support (CPU mode)
cargo build --release --features aura-engine
```

---

## 🚀 Usage

### 1. Native Genie Engine (NPU – recommended)

Use pre‑converted Genie models (e.g. `phi_3_5_mini_instruct-genie-w4a16-qualcomm`).

```powershell
# Set environment (optional, the SDK tries to auto‑detect)
$env:ADSP_LIBRARY_PATH = "C:\Qualcomm\AIStack\QAIRT\2.45.40.260406\lib\hexagon-v73\unsigned"

.\target\release\aura-sdk.exe --prompt "Explain the benefits of NPU."
```

### 2. Aura Engine (ORT – CPU only)

Use any ONNX model (quantised or not). **Note**: ONNX Runtime with QNN provider is not enabled by default because most ONNX models are not compatible with the Hexagon NPU. The engine falls back to CPU.

```powershell
# Add QNN DLLs to PATH (only needed for environment setup)
$env:PATH = "C:\Qualcomm\AIStack\QAIRT\2.45.40.260406\lib\aarch64-windows-msvc;" + $env:PATH

.\target\release\aura-sdk.exe --ort --model path/to/model.onnx --prompt "Hello NPU!"
```

Exemple for running the Aura Engine with ORT on a quantised ONNX model:

Model: `hf download onnx-community/Llama-3.2-1B-Instruct-ONNX --local-dir Llama-1B-ONNX`

```powershell
cargo run --release --features aura-engine -- --ort --model Llama-1B-ONNX/onnx/model_q4.onnx --prompt "Explain NPU in 3 sentences."
```

Expect **50 tokens/second** on CPU for a 1B parameter model.

---

## 📊 Expected Performance

| Mode          | Backend | Typical TPS (Snapdragon X Plus) |
|---------------|---------|----------------------------------|
| Genie (NPU)   | Hexagon | 10-35 (depending on model size) |
| Aura (ORT)    | CPU     | 50 (1B int8)                  |

---

## 📜 License

MIT License
