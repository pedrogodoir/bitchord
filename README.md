# BitChord

A peer-to-peer file sharing application inspired by BitTorrent, built on top of the Chord Distributed Hash Table (DHT) protocol.

## 💻 Tech Stack

[![Static Badge](https://img.shields.io/badge/Tauri-000000?style=flat&logo=Tauri)](https://v2.tauri.app/)
[![Static Badge](https://img.shields.io/badge/Rust-000000?style=flat&logo=Rust)](https://rust-lang.org/)
[![Static Badge](https://img.shields.io/badge/React-000000?style=flat&logo=React)](https://react.dev/)

### Why Tauri?

Tauri is a framework for building cross-platform desktop applications using Rust as the backend and modern web technologies such as React for the frontend. It renders the user interface through the operating system's native WebView, resulting in lightweight, secure, and high-performance applications. It is often compared to Electron, but typically produces significantly smaller binaries and lower memory usage.

## ▶️ Building and Running

Before you begin, make sure that **Docker** is installed and running on your machine.

### 1. Build the Tauri builder image

From the project root, run:

```bash
docker build -f Dockerfile.build -t tauri-builder .
```

### 2. Start the build container

```bash
docker run --rm -it -v "${PWD}:/app" tauri-builder
```

If everything worked correctly, you should see a prompt similar to:

```text
root@xxxxxxxx:/app#
```

### 3. Install dependencies and build the application

```bash
npm install
cargo tauri build
```

### 4. Run the generated application

The generated AppImage will be located at:

```text
src-tauri/target/release/bundle/appimage/
```

Give permission:

```bash
chmod +x bitchord_0.1.0_amd64.AppImage
```

Then run it:

```bash
./bitchord_0.1.0_amd64.AppImage
```
