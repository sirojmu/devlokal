# devlokal

A free, lightweight, bandwidth-friendly local web development environment (nginx + PHP + MariaDB),
designed for users in Indonesia: resumable checksum-verified downloads, offline install from local
archives, low memory use, Indonesian-language messages (English via `DEVLOKAL_LANG=en`).

**Status: pre-release.** The core and CLI are tested on Linux only; Windows, macOS, MariaDB and the
Tauri desktop app are untested. See [docs/ARSITEKTUR.md](docs/ARSITEKTUR.md) (in Indonesian) for the
test-status table. The component manifest intentionally ships without checksums; fill them from
official sources before use.

```bash
cargo build --release
devlokal import nginx-1.26.2 nginx.zip --strip 1
devlokal import php-8.3.12 php.zip
devlokal new shop && devlokal sync && devlokal up
```

MIT licensed. nginx, PHP and MariaDB are not included and have their own licenses.
