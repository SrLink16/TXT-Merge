<p align="center">
  <strong>English</strong> · <a href="README_es.md">Español</a>
</p>

<p align="center">
  <img src="assets/banner.png" width="480" alt="TXT Merge - MERGER & EDITOR">
</p>

<p align="center">
  <strong>A fast, lightweight, and modern tool for merging text files and batch editing.</strong>
</p>

<p align="center">
  <a href="https://github.com/SrLink16/TXT-Merge/releases/latest"><img src="https://img.shields.io/github/v/release/SrLink16/TXT-Merge?style=flat-square&color=008080" alt="Latest Release"></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-GPL--3.0-blue?style=flat-square" alt="License: GPL-3.0"></a>
  <img src="https://img.shields.io/badge/platform-Windows%2010%2F11-blue?style=flat-square" alt="Platform Windows">
  <img src="https://img.shields.io/badge/built%20with-Rust-orange?style=flat-square&logo=rust" alt="Rust">
  <img src="https://img.shields.io/badge/GUI-egui-purple?style=flat-square" alt="egui">
  <img src="https://img.shields.io/badge/portable-no%20install-success?style=flat-square" alt="Portable">
</p>

---

## ⚡ Overview

**TXT Merge** is a native desktop application built in Rust designed to merge multiple text files into a single structured output, as well as perform batch search-and-replace modifications with instant diff previews.

It is completely portable, ultra-fast, and requires zero installation.

---

## ✨ Features

### 📂 File Merger
* **Smart Text Detection**: Automatically distinguishes valid text files from binary files by inspecting encodings and headers.
* **Multi-Format Export**: Combine files into `.txt`, `.md`, `.csv`, `.tsv`, `.json`, `.yaml`, `.xml`, `.html`, `.toml`, `.sql`, `.srt`, and more.
* **Custom Separators**: Insert headers, filenames, relative paths, or custom divider lines between files.
* **Drag & Drop**: Drop entire folders or specific files directly into the window.
* **Encoding Support**: Automatic detection and decoding for UTF-8, UTF-16, Latin-1, and Windows-1252.

### 📝 Batch Editor
* **Multi-File Search & Replace**: Apply plain text or regex find-and-replace rules across all scanned files simultaneously.
* **Live Diff View**: Visual side-by-side or inline diff inspection before committing any changes to disk.
* **Non-Destructive**: Safe previews ensure no files are overwritten unexpectedly.

### 🎨 Modern Interface
* **Themes**: Dark and Light theme modes.
* **Multi-Language GUI**: Native English and Spanish interface (automatically adapts to your OS language).

---

## 🚀 Download & Installation

### Windows (Portable)
No installation required:

1. Head over to the [**Latest Releases**](https://github.com/SrLink16/TXT-Merge/releases/latest).
2. Download **`TXT-Merge.exe`**.
3. Double-click and run! (Compatible with Windows 10 & 11).

---

## 🛠️ Building from Source

Make sure you have [Rust](https://www.rust-lang.org/) installed:

```bash
git clone https://github.com/SrLink16/TXT-Merge.git
cd TXT-Merge
cargo build --release
```

The compiled binary will be located in `target/release/TXT-Merge.exe`.

---

## 👤 Author

* **[SrLink16](https://github.com/SrLink16)**

---

## 📄 License

This project is licensed under the **GNU General Public License v3.0 (GPL-3.0)**. See the [LICENSE](LICENSE) file for details.
