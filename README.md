# Vigie

---

[![CI](https://github.com/curiousmindflow/vigie/actions/workflows/ci.yml/badge.svg)](https://github.com/curiousmindflow/vigie/actions/workflows/ci.yml)
[![License](https://img.shields.io/badge/license-Apache--2.0%2FMIT-blue)](https://github.com/curiousmindflow/vigie/LICENSE-APACHE)

<!-- [![Vigie banner](.github/images/banner.png)(https://github.com/curiousmindflow/vigie/)] -->

<p align="center">
  <img src=".github/images/banner.png" alt="Vigie" width="50%"/>
</p>

## Introduction

---

`Vigie` is a SWIM protocol implementation in Rust

## About SWIM

...

## Why Vigie ?

...

**Sans-io** means the protocol core operates on messages and state machines without directly performing I/O operations. Instead, it returns "effects" that the realization layer executes.

## Project status

⚠️ **Experimental / Work in Progress**

## Technology Stack

Complete implementation in Rust.

- **Core:** Pure Rust, sans-io, lock-free state machines
- **Realization:** Tokio async runtime, Kameo actor framework
- **Testing:** Unit tests (rstest), property-based testing (proptest), mutation testing (cargo-mutants), fuzzing (cargo-fuzz)

## Changelog

See [CHANGELOG.md](CHANGELOG.md).

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md).

## License

Apache 2.0 / MIT
