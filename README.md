# 🦀 qnn-bindings-rs

[![Rust](https://img.shields.io/badge/rust-stable-brightgreen.svg)](https://www.rust-lang.org)
[![Platform](https://img.shields.io/badge/platform-Windows--ARM64-blue.svg)](https://learn.microsoft.com/en-us/windows/arm/)
[![NPU](https://img.shields.io/badge/accelerator-Qualcomm--Hexagon-orange.svg)](https://www.qualcomm.com/products/technology/processors/snapdragon-x-elite)
[![License](https://img.shields.io/badge/license-MIT-lightgrey.svg)](LICENSE)

**High-performance Rust bindings and CLI runner for Qualcomm Genie/QNN inference on Snapdragon NPUs.**

Accelerate modern LLMs like **Phi-3.5 Mini Instruct** directly on your Snapdragon X Elite or X Plus (including the 8-core X1P-42-100) using the Hexagon NPU. This project provides a clean, zero-overhead Rust interface to the Qualcomm Genie SDK.

---

## 🚀 Key Features

-   **Smart Streaming**: Real-time token output with automatic ChatML/Phi-3.5 formatting and **loop protection**.
-   **Hardware Accelerated**: Targeted optimization for **Snapdragon X Plus (8-core)** and **X Elite** (v73 HTP).
-   **Performance Metrics**: Built-in tracking for TTFT (Time To First Token) and TPS (Tokens Per Second).
-   **Safety Guards**: Configurable `--max-tokens` and automatic whitespace detection to prevent infinite generation.
-   **Beginner Friendly**: Comprehensive guide from installation to first inference.

---

## 📋 Prerequisites

Before you begin, ensure your system meets these requirements:

### 1. Hardware
-   **Device**: A Windows 11 ARM64 laptop with a **Snapdragon X Elite** or **Snapdragon X Plus** SoC.
-   **Memory**: 16GB RAM recommended.

### 2. Software
-   **Rust Toolchain**: Install via [rustup.rs](https://rustup.rs/).
    -   Target: `rustup target add aarch64-pc-windows-msvc`
-   **QAIRT SDK**: Download the **Qualcomm AI Stack (QAIRT)** v2.45+ from the [Qualcomm Software Center](https://qpm.qualcomm.com/).
-   **LLVM**: Required for binding generation. Download the Windows ARM64 LLVM installer from the [LLVM releases page](https://github.com/llvm/llvm-project/releases).

---

## 🛠️ Step 1: Environment Setup

The QNN runtime needs to know where your SDK and NPU libraries are located. Open a **PowerShell** terminal and run:

```powershell
# Set your SDK path (verify your version number)
$env:QNN_SDK_ROOT = "C:\Qualcomm\AIStack\QAIRT\2.45.40.260406"

# Add SDK libraries to your Path
$env:Path = "$env:QNN_SDK_ROOT\lib\aarch64-windows-msvc;" + $env:Path

# Point to the NPU microcode (v73 is for Snapdragon X series)
$env:ADSP_LIBRARY_PATH = "$env:QNN_SDK_ROOT\lib\hexagon-v73\unsigned"
```

---

## 🧠 Step 2: Download the Model (Phi-3.5)

Qualcomm AI Hub provides optimized "Genie Bundles" for Snapdragon NPUs.

1.  Visit [Qualcomm AI Hub - Phi-3.5-mini-instruct](https://aihub.qualcomm.com/compute/models/phi_3_5_mini_instruct).
2.  Select **Snapdragon X Elite** (or X Plus) as the device.
3.  Choose the **Genie** runtime and download the bundle.
4.  Extract the ZIP into a folder named `phi_3_5_mini_instruct-genie-w4a16-qualcomm` in this project's root.

**Your folder should contain:**
-   `*.serialized.bin` (Compiled NPU binaries)
-   `tokenizer.json` (The model's vocabulary)
-   `genie_config.json` (Inference settings)
-   `htp_backend_ext_config.json` (NPU backend config)

---

## 🔨 Step 3: Build & Run

1.  **Generate Bindings & Compile**:
    ```powershell
    cargo build --release
    ```
    *Note: `build.rs` will automatically find your QNN SDK and generate the necessary Rust code.*

2.  **Launch Inference**:
    ```powershell
    # Simple prompt (automatic Phi-3.5 formatting)
    .\target\release\qnn-bindings-rs.exe --prompt "Explain the benefits of NPU with two sentences."

    # Advanced options
    .\target\release\qnn-bindings-rs.exe --prompt "Write a story." --max-tokens 1024 --verbose
    ```

The CLI will automatically wrap your prompt in the **Phi-3.5 Instruct format** (`<|user|>...<|end|>`) and handle the NPU streaming.

---

## 📊 Performance & Benchmarks

Measured on **Snapdragon X Plus (8-core)** with Phi-3.5 Mini (W4A16):

| Metric | Typical Value |
| :--- | :--- |
| **TTFT** (First Token) | ~280ms - 350ms |
| **TPS** (Generation) | ~18 - 22 tokens/s |
| **NPU Power** | Highly Efficient (< 5W) |

---

## 🔍 Troubleshooting (Common Issues)

### ❌ Error 30001 (Invalid Binary)
This is the most common error. It means the `.bin` files don't match your SDK version.
-   **Fix**: Ensure your `QNN_SDK_ROOT` version matches the version used on AI Hub. If you change the SDK version, run `cargo clean` and rebuild.
-   **Config**: Check `htp_backend_ext_config.json`. Set `"soc_model": 43` or `60`.

### ❌ "Could not find Genie.lib"
The linker cannot find the Qualcomm libraries.
-   **Fix**: Verify that `$env:QNN_SDK_ROOT\lib\aarch64-windows-msvc\Genie.lib` exists.

### ❌ Loops or Endless Text
The model doesn't know when to stop.
-   **Fix**: Our code handles this automatically, but ensure `genie_config.json` has `temp: 0.1` and you are using the provided `src/main.rs` which detects the `<|end|>` tag.

---

## 📜 License

This project is licensed under the **MIT License**.
*Qualcomm, Snapdragon, and QNN are trademarks of Qualcomm Technologies, Inc. Model files and SDKs are subject to Qualcomm's own terms and licenses.*

---

## 🏷️ Tags
`rust` `qnn` `qualcomm` `snapdragon` `npu` `llm` `phi-3.5` `windows-arm64` `hexagon` `genie-sdk`
