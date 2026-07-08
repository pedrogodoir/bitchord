# BitChord

A peer-to-peer file sharing application inspired by BitTorrent, built on top of the Chord Distributed Hash Table (DHT) protocol.

## 🧠 Architecture Overview

The system architecture is decentralized and divided into two main layers of responsibility:

### 1. The Distributed Hash Table (Chord Protocol)
The network operates on a logical ring topology with an 8-bit identifier space (IDs from 0 to 255).
* **Nodes:** Each application instance acts as a node (`Node`) with an ID generated from its IP address. Each node maintains references to its Successor, Predecessor, and a **Finger Table** (routing table) to guarantee logarithmic search complexity.
* **Metadata Storage:** The Chord ring **does not** store the heavy files. It acts as a decentralized tracker, storing only the torrent metadata (`TorrentMeta`), mapped by the SHA-1 hash of the filename.

### 2. The File Sharing Layer
* **Seeders & Chunks:** The actual files remain on the users' disks. During upload, files are split into fixed 256 KB pieces (`CHUNK_SIZE`).
* **Automatic Seeding:** Once a node successfully downloads a file, it automatically registers itself as a seeder in the DHT, strengthening the network.
* **`.bitchord` Files:** These act like traditional `.torrent` files. They contain the vital information (block hashes, total size, filename) required for a client to request chunks and validate data integrity.

---

## 🔄 Main Execution Flows

### Discovery & Join Flow
1. A node enters the network and broadcasts a UDP Multicast ping (`PING_CHORD_JOIN`) on port 5000.
2. If an existing node is listening, it acknowledges the presence.
3. The joining node connects via TCP to the contact node and uses the `FindSuccessor` RPC to discover its correct position in the 0-255 ring.
4. Background routines (`stabilize` and `fix_fingers`) run periodically to stitch the node into the network and update routing tables.

### Upload / Seeding Flow
1. The user selects a file via the UI (`upload_file`).
2. The backend reads the file, splits it into 256 KB chunks, and calculates the SHA-1 hash for each chunk.
3. The SHA-1 hash of the *filename* dictates which ring ID is responsible for storing this metadata.
4. The system routes the request (`find_successor_rpc`) to the responsible node and sends the `TorrentMeta` via a `PutData` message. The uploader's IP is added to the `seeders` list.
5. A `.bitchord` file is generated locally, and the original node starts seeding.

### Search & Download Flow (Parallel & Load Balanced)
1. The user inputs a `.bitchord` file or a direct hash. The system extracts the target ID.
2. The network is queried (`fetch_meta`) to find the node holding the `TorrentMeta`.
3. The node initiates a **parallel download** (`download_all_chunks`) using a thread pool.
4. **Load Balancing:** Chunk requests are distributed evenly across all available seeders using a Round-Robin strategy, preventing network bottlenecks.
5. As chunks arrive concurrently, their SHA-1 hashes are recalculated and verified. Valid chunks are stitched back together in the correct order.
6. The node registers itself as a new seeder for the downloaded file.

### Fault Tolerance & Crash Recovery
* **Heartbeat Protocol:** Nodes actively monitor their immediate neighbors using rapid TCP ping checks (`is_alive`).
* If a node crashes or drops unexpectedly (e.g., forced exit), neighboring nodes detect the timeout, dynamically bypass the dead node using their Finger Tables, and heal the ring automatically.

### Graceful Leave Flow
1. When the app is normally closed (`leave_network`), the node triggers `leave_ring`.
2. **Ghost Seeder Cleanup:** It actively queries the DHT to remove its own IP from the seeder lists of all files it was currently sharing.
3. It transfers its entire metadata dictionary (`storage`) to its Successor to prevent network data loss.
4. It notifies its Predecessor and Successor to connect to each other, seamlessly patching the hole in the ring.

---

## 🔌 Interfaces (APIs)

### Node-to-Node Network Interface
All TCP communication between peers uses JSON-serialized objects modeled by the `Message` enum:
* **Chord Maintenance:** `Ping`, `Pong`, `FindSuccessor`, `GetPredecessor`, `Notify`.
* **Routing & Storage:** `PutData`, `GetData`, `TransferKeys`.
* **File Transfer:** `RequestChunk`, `ChunkData`.

### Backend-to-Frontend Interface (Tauri)
Commands exposed by Rust to the React UI:
* `get_node_info`: Returns the current node state (ID, successor, predecessor).
* `join_network` / `leave_network`: Controls entry and graceful exit from the ring.
* `upload_file`: Initiates the file publication process and returns status messages.
* `search_file`: Queries the DHT for metadata.
* `download_file` / `download_by_hash`: Starts the chunk-gathering process.

---

## 💻 Tech Stack

[![Static Badge](https://img.shields.io/badge/Tauri-000000?style=flat&logo=Tauri)](https://v2.tauri.app/)
[![Static Badge](https://img.shields.io/badge/Rust-000000?style=flat&logo=Rust)](https://rust-lang.org/)
[![Static Badge](https://img.shields.io/badge/React-000000?style=flat&logo=React)](https://react.dev/)

* **Core Language:** Rust (Memory safety, high concurrency).
* **Networking:** Standard TCP for reliable RPCs and UDP Multicast for trackerless local network discovery.
* **Concurrency:** `rayon` for data-parallelism and thread-pool management during downloads.
* **Cryptography:** SHA-1 for ID generation and chunk integrity validation.
* **Serialization:** JSON via `serde` / `serde_json`.

---

## ▶️ Building and Running

There are two ways to build BitChord, depending on your environment and operating system.

### Method 1: With Docker (Linux only / Generates `.AppImage`)
Ideal if you are on Linux and **do not** want to install the Rust and Node.js environments on your machine. Docker will handle everything and generate a standalone executable.

## 1. Build the Tauri builder image:
From the project root, run:
```bash
docker build -f Dockerfile.build -t tauri-builder .
```

## 2. Start the build container:
```bash
docker run --rm -it -v "${PWD}:/app" tauri-builder
```
You will see a prompt similar to:
```text
root@xxxxxxxx:/app#
```
This indicates that you are inside the container.

## 3. Install dependencies and build:
Inside the container, run:
```bash
npm install
npm run tauri build
```

## 4. Locate and run the application:
The generated AppImage will be located at:
```text
src-tauri/target/release/bundle/appimage/
```
Grant execution permissions and run the file:
```bash
chmod +x bitchord_0.1.0_amd64.AppImage
./bitchord_0.1.0_amd64.AppImage
```

---

### Method 2: Native (Windows or Linux with Rust installed)
Ideal if you already have the development environment set up, with `Node.js` and `Rust` installed. Tauri automatically detects the operating system and generates the corresponding executable (`.exe` for Windows or `.AppImage`/`.deb` for Linux).

## 1. Install frontend dependencies:
From the project root, open the terminal and run:
```bash
npm install
```

## 2. Build the application:
Use the NPM script that calls Tauri to generate the optimized executable:
```bash
npm run tauri build
```

## 3. Locate the final executable:
* **On Windows:** the installer will be located at:
  ```text
  src-tauri/target/release/bundle/nsis/
  ```
  The direct executable can also be found at:
  ```text
  src-tauri/target/release/
  ```
* **On Linux:** the AppImage will be located at:
  ```text
  src-tauri/target/release/bundle/appimage/
  ```
