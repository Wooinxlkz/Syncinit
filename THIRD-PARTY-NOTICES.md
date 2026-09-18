# Third-Party Notices

Syncinit is built with the following open-source components. Licenses listed
below are believed accurate based on each project's standard/current
licensing at the time of writing, **not independently re-verified for this
build** — before a public release, regenerate this list with actual
tooling rather than trusting this file blind:

```bash
# Rust dependencies
cargo install cargo-license
cd src-tauri && cargo license > ../THIRD-PARTY-NOTICES-rust.txt

# npm dependencies
npx license-checker --summary
```

## Rust crates (src-tauri/Cargo.toml)

| Crate | License |
|---|---|
| tauri, tauri-build, tauri-plugin-dialog, tauri-plugin-fs, tauri-plugin-shell | MIT OR Apache-2.0 |
| serde, serde_json | MIT OR Apache-2.0 |
| zip | MIT |
| sevenz-rust | MIT OR Apache-2.0 |
| tar, flate2, xz2, zstd, bzip2, crc32fast | MIT OR Apache-2.0 |
| walkdir | Unlicense OR MIT |
| rayon | MIT OR Apache-2.0 |
| aes-gcm, argon2, sha2, zeroize (RustCrypto) | MIT OR Apache-2.0 |
| rand | MIT OR Apache-2.0 |
| thiserror, anyhow | MIT OR Apache-2.0 |
| chrono | MIT OR Apache-2.0 |
| winreg | MIT |

## npm packages (package.json)

| Package | License |
|---|---|
| react, react-dom | MIT |
| motion | MIT |
| @tauri-apps/api, @tauri-apps/plugin-dialog, @tauri-apps/plugin-fs, @tauri-apps/plugin-shell | MIT OR Apache-2.0 |
| @tauri-apps/cli | MIT OR Apache-2.0 |
| vite, @vitejs/plugin-react | MIT |
| typescript | Apache-2.0 |

## Fonts

| Font | License |
|---|---|
| Inter | SIL Open Font License 1.1 |
| JetBrains Mono | SIL Open Font License 1.1 |

Both permit bundling and use in commercial/free software without
royalties; attribution here satisfies the OFL's requirements.

## UI component origin

The PIN entry component (`src/OTPInput.tsx`) and the motion easing curve
(`src/ease.ts`) were adapted from the developer's own separate project
(Xuro) — same author, not a third-party dependency, no license concern.
