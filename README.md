# Aetherial Zero-Footprint (`aetherial-zero-footprint`)

[![License: AGPL v3](https://badgen.net/badge/License/AGPLv3/blue)](./LICENSE)
[![Commercial License](https://badgen.net/badge/Commercial/Available/purple)](./COMMERCIAL_LICENSE.md)
[![Core: Rust](https://badgen.net/badge/Core/Rust/orange)](https://rust-lang.org)
[![Bindings: Python | Node | WASM](https://badgen.net/badge/Bindings/Python%20%7C%20Node%20%7C%20WASM/green)](#quick-start-usage)
[![Sponsor](https://badgen.net/badge/Sponsor/GitHub/black?icon=github)](https://github.com)


**Deterministic zero-swap RAM locking and cryptographic NVMe scratchpad shredding for serverless functions, multi-tenant workers, and ephemeral compute environments.**

---

## Table of Contents
1. [Why This Project Exists](#why-this-project-exists)
2. [How It Works (Core Primitives)](#how-it-works-core-primitives)
3. [Real-World Use Cases](#real-world-use-cases)
4. [Build & Installation](#build--installation)
5. [Quick-Start Usage (Rust, Python, Node.js, WASM)](#quick-start-usage)
6. [Security Boundaries & Threat Model](#security-boundaries--threat-model)
7. [Licensing & Commercial Use](#licensing--commercial-use)
8. [Support & Sponsorships](#support--sponsorships)

---

## Why This Project Exists

Modern cloud-native backends rely heavily on ephemeral compute: AWS Lambda, Google Cloud Run, Cloudflare Workers, and high-density multi-tenant worker pools. During a single request, applications routinely decrypt tenant payloads, verify JWTs, handle PII, and write temporary files to `/tmp`—all within milliseconds.

When the request completes, developers assume `free()`, garbage collection, or `rm` / `unlink()` deletes the secret. **At the OS and hardware level, it does not:**

* **OS Swap Leakage:** Under memory pressure, operating systems page heap allocations out to disk swap—leaving decrypted keys on persistent storage.
* **Compiler Dead-Store Elimination:** Standard `memset(0)` calls right before deallocation are frequently stripped out by compiler optimizations (`LLVM`/`GCC`) because the buffer is never read again.
* **Warm Container & NVMe Scratchpad Artifacts:** Cloud providers reuse "warm" microVMs across requests. Calling `unlink()` on `/tmp` removes only the directory pointer, leaving raw data blocks intact on local NVMe scratch disks.

**`aetherial-zero-footprint`** eliminates these silent leaks through a zero-dependency, memory-safe **Rust** core with native FFI bindings for **Python**, **Node.js**, and **WebAssembly**.

---

## How It Works (Core Primitives)

```text
┌─────────────────────────────────────────────────────────────────────────┐
│                    EPHEMERAL EXECUTION LIFECYCLE                        │
│                                                                         │
│  1. INGEST   ──► Copy secret into RAM & zeroize caller's source slice   │
│  2. PIN      ──► mlock (Unix) / VirtualLock (Windows) blocks OS swap    │
│  3. EXECUTE  ──► Process secrets inside strict lexical RAII scope       │
│  4. WIPE     ──► Volatile zeroize + memory fence + munlock on Drop      │
│  5. SHRED    ──► CSPRNG + zero overwrite + hardware fsync on /tmp files │
└─────────────────────────────────────────────────────────────────────────┘
```

1. **OS-Level Physical RAM Pinning (`LockedBuffer`):** Calls `libc::mlock` on Linux/macOS and `VirtualLock` on Windows to pin memory pages in physical RAM, forbidding the kernel from paging sensitive buffers to swap.
2. **Compiler-Proof Volatile Zeroization (`zeroize`):** Uses volatile writes and compiler memory barriers inside Rust's `Drop` trait—guaranteeing every byte is overwritten with `0x00` even if an exception or thread panic occurs.
3. **Multi-Pass NVMe Scratchpad Shredding (`EphemeralFile`):** Intercepts scope exit to overwrite temporary files in-place with cryptographic randomness (`CSPRNG`), followed by a deterministic zero pass and hardware `fsync()` before unlinking.

---

## Real-World Use Cases

* **Multi-Tenant Serverless Functions (AWS Lambda / Cloud Run):** Prevent cross-tenant memory and `/tmp` data leakage when cloud providers reuse warm execution containers across different customers.
* **KMS & Customer-Managed Keys (BYOK) Handling:** Pin decrypted Data Encryption Keys (DEKs) in non-swappable RAM for the exact duration of a cryptographic operation, then wipe immediately.
* **AI Agents & PII Processing Pipelines:** Safely buffer sensitive documents, healthcare records (HIPAA), or financial payloads during local inference/ETL without leaving disk or swap artifacts.
* **GDPR & Right-to-be-Forgotten Compliance:** Enforce deterministic runtime cleanup for temporary user exports, billing batches, and session caches.
* **Edge & Browser Workers (WebAssembly):** Explicitly zeroize linear WASM memory buffers (`WasmEphemeralBuffer`) before releasing allocations back to the V8 host runtime.

---

## Build & Installation

### Prerequisites
* **Rust Toolchain (`1.75+`):** Install via [rustup.rs](https://rustup.rs)
* **Optional FFI Build Tools:** `maturin` (Python), `@napi-rs/cli` (Node.js), `wasm-pack` (WebAssembly)

### 1. Native Rust Crate
```bash
cargo build --release
cargo test
```

### 2. Python Wheel (`PyO3` + `maturin`)
```bash
pip install maturin
maturin build --release --features python
pip install target/wheels/aetherial_zero_footprint-*.whl
```

### 3. Node.js / TypeScript Native Addon (`NAPI-RS`)
```bash
npm install -g @napi-rs/cli
napi build --platform --release --features node
```

### 4. WebAssembly / Cloudflare Workers (`wasm-pack`)
```bash
cargo install wasm-pack
wasm-pack build --target bundler --release -- --no-default-features --features wasm
```

---

## Quick-Start Usage

### Rust (RAII Scope Guard)
```rust
use aetherial_zero_footprint::{LockedBuffer, EphemeralFile};
use std::io::Write;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Pin secret in physical RAM (no swap) and wipe caller's input slice
    let mut raw_key = b"sensitive_256bit_tenant_secret".to_vec();
    {
        let locked = LockedBuffer::from_secret_slice(&mut raw_key)?;
        assert_eq!(locked.len(), 30);
        // Use `locked` safely inside this block...
    } // `locked` is deterministically zeroized and unlocked here

    // 2. Auto-shred scratchpad file on scope exit (2 passes: CSPRNG + zeros + fsync)
    {
        let scratch = EphemeralFile::new("/tmp/tenant_payload.tmp", 2);
        let mut file = std::fs::File::create(scratch.path())?;
        file.write_all(b"ephemeral_tenant_records")?;
        file.sync_all()?;
    } // File is overwritten, synced to hardware, and unlinked automatically

    Ok(())
}
```

### Python (Context Manager)
```python
from aetherial_zero_footprint import PyEphemeralBuffer, shred_file

# Secret is pinned in physical RAM and wiped synchronously on __exit__
with PyEphemeralBuffer(b"customer_kms_derived_key") as vault:
    secret_bytes = vault.read_bytes()
    # Execute sensitive operation...

# Overwrite temporary file in-place (random + zeros + fsync) and unlink
shred_file("/tmp/session_export.csv", passes=2)
```

### Node.js / TypeScript
```typescript
import { NodeEphemeralBuffer, shredScratchpadFile } from './index';

const vault = new NodeEphemeralBuffer(Buffer.from('tenant-signing-secret'));
try {
  const secret = vault.read();
  // Execute sensitive operation...
} finally {
  // Deterministic volatile zeroize + munlock
  vault.destroy();
}

// Multi-pass overwrite + fsync + unlink
shredScratchpadFile('/tmp/ephemeral_batch.bin', 2);
```

### WebAssembly (Cloudflare Workers / Edge)
```javascript
import { WasmEphemeralBuffer } from './pkg/aetherial_zero_footprint.js';

const encoder = new TextEncoder();
const vault = new WasmEphemeralBuffer(encoder.encode('edge-session-token'));

try {
  const bytes = vault.read();
  // Use token...
} finally {
  // Explicitly zeroizes linear WASM memory before freeing
  vault.destroy();
}
```

---

## Security Boundaries & Threat Model

Good security engineering requires stating architectural boundaries clearly:

* **Host-Language Heap Copies:** Calling `.read_bytes()` in Python or `.read()` in Node.js allocates a copy inside the host language's garbage-collected heap (`pymalloc` / V8). Keep sensitive processing inside Rust whenever possible, or minimize the lifetime of host copies.
* **Kernel & Hypervisor Privileges:** `mlock` / `VirtualLock` prevents user-space swap leaks, not privileged root or hypervisor memory inspection. For hardware-level host isolation, pair with Confidential Computing (`AMD SEV` / `Intel TDX`).
* **NVMe Wear-Leveling:** Modern SSD controllers remap physical flash blocks via Copy-on-Write wear-leveling. Multi-pass overwriting + `fsync` eliminates logical and warm-container recovery, while full-disk encryption handles physical media end-of-life.

---

## Licensing & Commercial Use

`aetherial-zero-footprint` is dual-licensed:

* **Open Source (`AGPL-3.0-only`):** Free for open-source projects, academic research, and personal non-commercial testing. Under AGPLv3, if you run this library over a network (including commercial SaaS backends or serverless APIs), you must release the complete source code of your application under AGPLv3.
* **Commercial License (Proprietary / Closed-Source SaaS):** Allows you to embed `aetherial-zero-footprint` inside proprietary products and commercial SaaS platforms **without** open-sourcing your codebase. See [`COMMERCIAL_LICENSE.md`](./COMMERCIAL_LICENSE.md) or contact `licensing@aetherial.online`.

---

## Support & Sponsorships

* ❤️ **Sponsor on GitHub:** Click the **Sponsor** button at the top of this repository (`https://github.com`) — Commercial Sponsor tiers automatically include a proprietary license.
* 🚀 **Need Full-Stack Scale-to-Zero Database Isolation?** `aetherial-zero-footprint` was extracted from **[Aetherial](https://aetherial.online)** — our stateless, zero-knowledge storage fabric that hydrates physically isolated, `AES-256-GCM` encrypted tenant databases in `<2ms` (`0 MB` idle RAM) and supports `1ms` GDPR crypto-shredding.
