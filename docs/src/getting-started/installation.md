# Installation

## Prerequisites

- **Rust toolchain** — Perry is built with Cargo. Install via [rustup](https://rustup.rs/):
  ```bash
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
  ```

- **System linker** — Perry uses your system's C compiler to link:
  - **macOS**: Xcode Command Line Tools (`xcode-select --install`)
  - **Linux**: `gcc` or `clang` (`apt install build-essential`)
  - **Windows**: MSVC via Visual Studio Build Tools

## Install Perry

### From Source (recommended)

```bash
git clone https://github.com/skelpo/perry.git
cd perry
cargo build --release
```

The binary is at `target/release/perry`. Add it to your PATH:

```bash
# Add to ~/.zshrc or ~/.bashrc
export PATH="/path/to/perry/target/release:$PATH"
```

### Self-Update

Once installed, Perry can update itself:

```bash
perry update
```

This downloads the latest release and atomically replaces the binary.

## Verify Installation

```bash
perry doctor
```

This checks your installation, shows the current version, and reports if an update is available.

```bash
perry --version
```

## Platform-Specific Setup

### macOS

No additional setup needed. Perry uses the system `cc` linker and AppKit for UI apps.

For iOS development, install Xcode (not just Command Line Tools) for the iOS SDK and simulator.

### Linux

Install GTK4 development libraries for UI apps:

```bash
# Ubuntu/Debian
sudo apt install libgtk-4-dev

# Fedora
sudo dnf install gtk4-devel
```

### Windows

Install Visual Studio Build Tools with the "Desktop development with C++" workload.

## What's Next

- [Write your first program](hello-world.md)
- [Build a native app](first-app.md)
