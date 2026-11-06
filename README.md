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
* **Drag & Drop**: Drop a folder to list all its text files, or drop one or more files to work with just those.
* **Encoding Support**: Automatic detection and decoding for UTF-8, UTF-16, Latin-1, and Windows-1252.

### 📝 Batch Editor
* **Multi-File Search & Replace**: Apply plain text or regex find-and-replace rules across all scanned files simultaneously.
* **Live Diff View**: GitHub-style inline diff (`-` removed, `+` added) before committing any changes to disk.
* **Non-Destructive**: Safe previews ensure no files are overwritten unexpectedly.

### 🎨 Modern Interface
* **Themes**: Dark and Light theme modes.
* **Multi-Language GUI**: Native English and Spanish interface (automatically adapts to your OS language).

---

## 🔤 Separator Variables

Write these tags in the separator and each one is replaced for every file. The value after `:` can be edited.

| Variable | Replaced with |
|---|---|
| `<Inc Nr:1>` | Ascending number. `<Inc Nr:10>` starts at 10; `<Inc Nr:001>` pads with zeros (001, 002…) |
| `<Inc NrDir:1>` | Ascending number that restarts in each folder |
| `<Inc Alpha:A>` | Ascending letter: A, B … Z, AA. `<Inc Alpha:c>` starts at c in lowercase |
| `<Name>` | File name without extension |
| `<Ext>` | File extension without the dot |
| `<FolderName:1>` | Name of the file's folder; `2` is the folder above it, and so on |
| `<Num Files>` | Number of listed files in the same folder |
| `<Num Dirs>` | Number of folders inside the file's folder |
| `<Num Items>` | Total number of files being merged |
| `<Word:1>` | Word N of the file name |
| `<RWord:1>` | Word N counting from the end of the file name |
| `<MediaType>` | Media (MIME) type from the extension, e.g. `text/markdown` |

Example: `=== <Inc Nr:1>/<Num Items> - <Name> ===` becomes `=== 3/12 - Report ===`.
An empty value (`<Inc Nr:>`) uses the default; an invalid one is left as written.

---

## ✏️ Batch Edit Operations

The **Edit** tab applies these steps to every checked file, in this order:

1. **Delete lines by number** — e.g. `1-3, 10, 20-` (`20-` means from line 20 to the end).
2. **Delete lines containing** a text.
   * *Also delete the empty line left after it* (on by default), so removing a title leaves no blank line at the top.
3. **Replace text** — plain text or regular expressions (`$1` inserts group 1), with an optional *Match case*. An empty replacement deletes the match.
4. **Trim spaces** at the start and end of each line.
5. **Remove empty lines**, or **keep at most one empty line in a row**. Lines with only spaces count as empty.

Before applying, press **View** on a file to see the **Changes** (GitHub-style diff), the **Result**, or the **Original** (editable).
Results are saved as **copies in a new subfolder** (`editados`, `editados (2)`…) or **overwrite the originals**. Each file keeps its encoding and line endings.

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

## 📋 Changelog

### v0.2.0
* Dropping one or more files now lists only those files; dropping a folder still lists the whole folder.
* The top bar shows *(dropped files only)* when the list comes from dropped files.
* Fixed building from source (Rust edition 2024).

### v0.1.0 — Initial Launch
* File merger with multi-format export, custom separators and separator variables.
* Batch editor with a GitHub-style diff preview.
* File viewer and editor with line numbers that keeps each file's encoding.
* Light and dark themes; English and Spanish interface.

---

## 👤 Author

* **[SrLink16](https://github.com/SrLink16)**

---

## 📄 License

This project is licensed under the **GNU General Public License v3.0 (GPL-3.0)**. See the [LICENSE](LICENSE) file for details.
