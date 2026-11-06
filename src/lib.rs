//! Core of TXT Merge: finding text files, decoding them and merging them
//! into one output file. The GUI in `main.rs` only drives these functions.

pub mod edit;

use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use walkdir::WalkDir;

/// Formats the combined text can be exported as (the output extension).
pub const EXPORT_EXTENSIONS: &[&str] = &[
    "txt", "md", "csv", "tsv", "log", "json", "xml", "html", "yaml", "toml", "ini", "sql", "srt",
    "tex", "nfo",
];

/// Extensions that are never text, so they are skipped without being opened.
const BINARY_EXTENSIONS: &[&str] = &[
    "exe", "dll", "sys", "msi", "bin", "iso", "img", "zip", "rar", "7z", "gz", "tar", "cab",
    "pdf", "doc", "docx", "xls", "xlsx", "ppt", "pptx", "odt", "ods", "png", "jpg", "jpeg",
    "gif", "bmp", "ico", "webp", "tif", "tiff", "psd", "mp3", "wav", "flac", "ogg", "mp4", "avi",
    "mkv", "mov", "wmv", "ttf", "otf", "woff", "woff2", "db", "sqlite", "lnk", "class", "pyc",
    "o", "obj", "lib", "pdb", "jar", "apk",
];

/// Looks at the first 8 KB of the file to decide whether it holds text,
/// whatever its extension.
pub fn is_text_file(path: &Path) -> bool {
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or_default();
    if BINARY_EXTENSIONS.iter().any(|b| b.eq_ignore_ascii_case(ext)) {
        return false;
    }
    let Ok(mut file) = fs::File::open(path) else { return false };
    let mut head = vec![0u8; 8192];
    let Ok(n) = io::Read::read(&mut file, &mut head) else { return false };
    looks_like_text(&head[..n])
}

fn looks_like_text(head: &[u8]) -> bool {
    if head.starts_with(&[0xFF, 0xFE]) || head.starts_with(&[0xFE, 0xFF]) {
        return true;
    }
    if head.contains(&0) {
        return false;
    }
    // Text has almost no control characters besides tab, newlines and form feed.
    let control = head
        .iter()
        .filter(|&&b| b < 0x20 && !matches!(b, b'\t' | b'\n' | b'\r' | 0x0C | 0x1B))
        .count();
    control * 100 <= head.len()
}

/// Separators shipped with the original program, plus a few more.
pub const PRESET_SEPARATORS: &[&str] = &[
    "-*-*-*-*-*-*-*-*-*-*-*-*-*-*-*-*-*-*-*-*-*-*-*-*-*-*-*-*-*-*-*-*-*-*-*-*",
    "************************************************************************",
    "========================================================================",
    "------------------------------------------------------------------------",
    "++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++",
    "########################################################################",
    "<---------------------------------------------------------------------->",
    "---",
];

pub const ERROR_LINE: &str = "[Error] - File could not be read (binary, locked or no access)";

#[derive(Clone, Debug, PartialEq)]
pub struct CombineOptions {
    /// Do not write the separator line under each file name.
    pub no_separator: bool,
    /// Do not write the file name above each segment.
    pub no_filename: bool,
    /// Join segments without line breaks between them (continuous text).
    pub no_returns: bool,
    /// With `no_returns`, put a single space between segments.
    pub add_space: bool,
    pub separator: String,
}

impl Default for CombineOptions {
    fn default() -> Self {
        Self {
            no_separator: false,
            no_filename: false,
            no_returns: false,
            add_space: false,
            separator: PRESET_SEPARATORS[0].to_owned(),
        }
    }
}

/// Turns user input such as " *.ADOC " into `Some("adoc")`.
pub fn normalize_extension(input: &str) -> Option<String> {
    let ext = input.trim().trim_start_matches('*').trim_start_matches('.').trim().to_lowercase();
    (!ext.is_empty() && !ext.contains(|c: char| c.is_whitespace() || "\\/:*?\"<>|.".contains(c)))
        .then_some(ext)
}

/// Lists every text file under `root`, whatever its extension, sorted by its
/// path relative to `root`. `exclude` lets the scan skip files this program
/// wrote itself.
pub fn scan(
    root: &Path,
    subfolders: bool,
    exclude: &dyn Fn(&Path) -> bool,
) -> Vec<PathBuf> {
    let depth = if subfolders { usize::MAX } else { 1 };
    let mut files: Vec<PathBuf> = WalkDir::new(root)
        .max_depth(depth)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_file())
        .map(|e| e.into_path())
        .filter(|p| !exclude(p) && is_text_file(p))
        .collect();
    files.sort_by_key(|p| display_path(root, p).to_lowercase());
    files
}

/// Path shown in the list and written as the file header.
pub fn display_path(root: &Path, path: &Path) -> String {
    path.strip_prefix(root).unwrap_or(path).display().to_string()
}

/// What a drag-and-drop onto the window means.
#[derive(Debug, PartialEq)]
pub enum Dropped {
    /// A folder was dropped: list everything in it.
    Folder(PathBuf),
    /// Only files were dropped: list just these. `root` is the folder that
    /// holds all of them, where results are saved.
    Files { root: PathBuf, files: Vec<PathBuf> },
}

/// A dropped folder wins over dropped files; with only files, the list is
/// limited to them.
pub fn classify_drop(paths: &[PathBuf]) -> Option<Dropped> {
    if let Some(dir) = paths.iter().find(|p| p.is_dir()) {
        return Some(Dropped::Folder(dir.clone()));
    }
    let files: Vec<PathBuf> = paths.iter().filter(|p| p.is_file()).cloned().collect();
    let root = common_folder(&files)?;
    Some(Dropped::Files { root, files })
}

/// Deepest folder containing every file; `None` when there are no files or
/// they share no folder (different drives).
pub fn common_folder(files: &[PathBuf]) -> Option<PathBuf> {
    let mut candidate = files.first()?.parent()?.to_path_buf();
    while !files.iter().all(|f| f.starts_with(&candidate)) {
        candidate = candidate.parent()?.to_path_buf();
    }
    Some(candidate)
}

/// The text files among `files` that still exist, without repeats, sorted
/// like a folder scan.
pub fn text_files_only(root: &Path, files: &[PathBuf]) -> Vec<PathBuf> {
    let mut kept: Vec<PathBuf> = files.iter().filter(|p| p.is_file() && is_text_file(p)).cloned().collect();
    kept.sort_by_key(|p| display_path(root, p).to_lowercase());
    kept.dedup();
    kept
}

/// How a text file's bytes were encoded, so an edited file is saved back the same way.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TextEncoding {
    Utf8,
    Utf8Bom,
    Utf16Le,
    Utf16Be,
    Windows1252,
}

/// Decodes a text file: UTF-8 (with or without BOM), UTF-16 with BOM, or
/// Windows-1252 as a fallback for old ANSI files. Files with NUL bytes and
/// no UTF-16 BOM are treated as binary and rejected.
pub fn decode_with_encoding(bytes: &[u8]) -> Option<(String, TextEncoding)> {
    if let Some(rest) = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]) {
        return Some((String::from_utf8_lossy(rest).into_owned(), TextEncoding::Utf8Bom));
    }
    if bytes.starts_with(&[0xFF, 0xFE]) {
        let text = encoding_rs::UTF_16LE.decode_with_bom_removal(bytes).0.into_owned();
        return Some((text, TextEncoding::Utf16Le));
    }
    if bytes.starts_with(&[0xFE, 0xFF]) {
        let text = encoding_rs::UTF_16BE.decode_with_bom_removal(bytes).0.into_owned();
        return Some((text, TextEncoding::Utf16Be));
    }
    if bytes.contains(&0) {
        return None;
    }
    match std::str::from_utf8(bytes) {
        Ok(s) => Some((s.to_owned(), TextEncoding::Utf8)),
        Err(_) => Some((
            encoding_rs::WINDOWS_1252.decode(bytes).0.into_owned(),
            TextEncoding::Windows1252,
        )),
    }
}

pub fn decode_text(bytes: &[u8]) -> Option<String> {
    decode_with_encoding(bytes).map(|(text, _)| text)
}

/// Encodes `text` back into `encoding`. ANSI cannot hold every character
/// (emoji, Chinese...); such text is written as UTF-8 with BOM instead, and
/// the encoding actually used is returned.
pub fn encode_text(text: &str, encoding: TextEncoding) -> (Vec<u8>, TextEncoding) {
    let with_prefix = |prefix: &[u8], body: Vec<u8>| [prefix.to_vec(), body].concat();
    match encoding {
        TextEncoding::Utf8 => (text.as_bytes().to_vec(), encoding),
        TextEncoding::Utf8Bom => (with_prefix(&[0xEF, 0xBB, 0xBF], text.as_bytes().to_vec()), encoding),
        TextEncoding::Utf16Le => {
            let body = text.encode_utf16().flat_map(u16::to_le_bytes).collect();
            (with_prefix(&[0xFF, 0xFE], body), encoding)
        }
        TextEncoding::Utf16Be => {
            let body = text.encode_utf16().flat_map(u16::to_be_bytes).collect();
            (with_prefix(&[0xFE, 0xFF], body), encoding)
        }
        TextEncoding::Windows1252 => {
            let (bytes, _, unmappable) = encoding_rs::WINDOWS_1252.encode(text);
            if unmappable {
                encode_text(text, TextEncoding::Utf8Bom)
            } else {
                (bytes.into_owned(), encoding)
            }
        }
    }
}

/// Writes edited text over `path` in `encoding`; returns the encoding used.
pub fn save_text(path: &Path, text: &str, encoding: TextEncoding) -> io::Result<TextEncoding> {
    let (bytes, used) = encode_text(text, encoding);
    fs::write(path, bytes)?;
    Ok(used)
}

/// Text loaded for the viewer.
#[derive(Debug, PartialEq)]
pub struct FileText {
    pub text: String,
    pub encoding: TextEncoding,
    /// The file is longer than what was loaded.
    pub truncated: bool,
}

/// Reads at most `max_bytes` of a file for showing it on screen.
/// `None` if it cannot be read as text.
pub fn read_preview(path: &Path, max_bytes: usize) -> Option<FileText> {
    let file = fs::File::open(path).ok()?;
    let mut bytes = Vec::new();
    io::Read::read_to_end(&mut io::Read::take(file, max_bytes as u64 + 1), &mut bytes).ok()?;
    let truncated = bytes.len() > max_bytes;
    if truncated {
        bytes.truncate(max_bytes);
        // Do not let a character cut in half turn the whole text into ANSI.
        if let Err(e) = std::str::from_utf8(&bytes) {
            if e.error_len().is_none() {
                bytes.truncate(e.valid_up_to());
            }
        }
    }
    decode_with_encoding(&bytes).map(|(text, encoding)| FileText { text, encoding, truncated })
}

/// One file ready to be merged; `content` is `None` when it could not be read.
pub struct Segment {
    /// Path shown in the header.
    pub name: String,
    /// Real path, used by the separator variables.
    pub path: PathBuf,
    /// Number of folders inside this file's folder (for `<Num Dirs>`).
    pub sibling_dirs: usize,
    pub content: Option<String>,
}

/// Tags that can be written in the separator. Each is replaced per file.
/// The value after `:` can be edited (start number, start letter, level...).
pub const SEPARATOR_VARIABLES: &[&str] = &[
    "<Inc Nr:1>",
    "<Inc NrDir:1>",
    "<Inc Alpha:A>",
    "<Name>",
    "<Ext>",
    "<FolderName:1>",
    "<Num Files>",
    "<Num Dirs>",
    "<Num Items>",
    "<Word:1>",
    "<RWord:1>",
    "<MediaType>",
];

/// Values the separator variables of one file are filled with.
pub struct VarContext<'a> {
    pub path: &'a Path,
    /// Position in the whole list, starting at 1.
    pub index: usize,
    /// Position among the listed files of the same folder, starting at 1.
    pub dir_index: usize,
    /// Listed files in the same folder.
    pub dir_files: usize,
    /// Folders inside the file's folder.
    pub sibling_dirs: usize,
    /// Files in the whole list.
    pub items: usize,
}

/// 1 -> A, 26 -> Z, 27 -> AA.
fn alpha(mut n: usize) -> String {
    let mut letters = Vec::new();
    while n > 0 {
        n -= 1;
        letters.push(b'A' + (n % 26) as u8);
        n /= 26;
    }
    letters.reverse();
    String::from_utf8(letters).expect("ASCII letters")
}

/// A -> 1, Z -> 26, AA -> 27; `None` for anything but letters.
fn alpha_to_number(letters: &str) -> Option<usize> {
    if letters.is_empty() || !letters.chars().all(|c| c.is_ascii_alphabetic()) {
        return None;
    }
    letters
        .to_ascii_uppercase()
        .bytes()
        .try_fold(0usize, |n, b| n.checked_mul(26)?.checked_add((b - b'A' + 1) as usize))
}

/// `start + position - 1`. A start written with leading zeros (`007`) keeps
/// that width: 007, 008, 009...
fn counter(arg: Option<&str>, position: usize) -> Option<String> {
    let arg = arg.unwrap_or("1");
    let start: usize = arg.parse().ok()?;
    let value = start.checked_add(position - 1)?;
    let width = if arg.len() > 1 && arg.starts_with('0') { arg.len() } else { 0 };
    Some(format!("{value:0width$}"))
}

/// Media (MIME) type guessed from the extension; text files default to text/plain.
pub fn media_type(path: &Path) -> &'static str {
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or_default().to_lowercase();
    match ext.as_str() {
        "md" | "markdown" => "text/markdown",
        "csv" => "text/csv",
        "tsv" => "text/tab-separated-values",
        "html" | "htm" => "text/html",
        "css" => "text/css",
        "js" | "mjs" => "text/javascript",
        "json" => "application/json",
        "xml" => "application/xml",
        "svg" => "image/svg+xml",
        "yaml" | "yml" => "application/yaml",
        "toml" => "application/toml",
        "sql" => "application/sql",
        "rtf" => "application/rtf",
        "tex" => "application/x-tex",
        "srt" => "application/x-subrip",
        "sh" => "application/x-sh",
        "php" => "application/x-httpd-php",
        "py" => "text/x-python",
        "rs" => "text/x-rust",
        "java" => "text/x-java",
        "c" | "h" => "text/x-c",
        "cpp" | "cc" | "cxx" | "hpp" => "text/x-c++",
        "cs" => "text/x-csharp",
        "go" => "text/x-go",
        _ => "text/plain",
    }
}

fn resolve_variable(token: &str, ctx: &VarContext) -> Option<String> {
    // An empty value (`<Inc Nr:>`) means the default, like no value at all.
    let (key, arg) = match token.split_once(':') {
        Some((k, a)) => (k, Some(a.trim()).filter(|a| !a.is_empty())),
        None => (token, None),
    };
    let key: String = key.chars().filter(|c| !c.is_whitespace()).collect::<String>().to_lowercase();
    // For variables whose value is a level or word position (1 or more).
    let position = || -> Option<usize> {
        match arg {
            None => Some(1),
            Some(a) => a.parse().ok().filter(|n| *n > 0),
        }
    };
    let stem = ctx.path.file_stem().and_then(|s| s.to_str()).unwrap_or_default();
    let word = |n: usize, from_end: bool| {
        let words: Vec<&str> = stem.split(|c: char| !c.is_alphanumeric()).filter(|w| !w.is_empty()).collect();
        let pick = if from_end { words.len().checked_sub(n) } else { Some(n - 1) };
        pick.and_then(|i| words.get(i)).map(|w| w.to_string()).unwrap_or_default()
    };
    let value = match key.as_str() {
        "incnr" => counter(arg, ctx.index)?,
        "incnrdir" => counter(arg, ctx.dir_index)?,
        "incalpha" => {
            let start_letters = arg.unwrap_or("A");
            let letters = alpha(alpha_to_number(start_letters)? + ctx.index - 1);
            if start_letters.chars().all(|c| c.is_ascii_lowercase()) {
                letters.to_ascii_lowercase()
            } else {
                letters
            }
        }
        "word" => word(position()?, false),
        "rword" => word(position()?, true),
        // 1 is the file's own folder, 2 the folder above it, and so on.
        "foldername" => ctx
            .path
            .ancestors()
            .nth(position()?)
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .unwrap_or_default()
            .to_owned(),
        _ if arg.is_some() => return None,
        "name" => stem.to_owned(),
        "ext" => ctx.path.extension().and_then(|e| e.to_str()).unwrap_or_default().to_owned(),
        "numfiles" => ctx.dir_files.to_string(),
        "numdirs" => ctx.sibling_dirs.to_string(),
        "numitems" => ctx.items.to_string(),
        "mediatype" => media_type(ctx.path).to_owned(),
        _ => return None,
    };
    Some(value)
}

/// Replaces `<Inc Nr>`, `<Name>`... in `template`. Unknown tags such as the
/// arrows of `<----->` are kept as written.
pub fn expand_variables(template: &str, ctx: &VarContext) -> String {
    let mut out = String::new();
    let mut rest = template;
    while let Some(start) = rest.find('<') {
        out.push_str(&rest[..start]);
        let tail = &rest[start..];
        let resolved = tail
            .find('>')
            .and_then(|end| resolve_variable(&tail[1..end], ctx).map(|v| (v, end)));
        match resolved {
            Some((value, end)) => {
                out.push_str(&value);
                rest = &tail[end + 1..];
            }
            None => {
                out.push('<');
                rest = &tail[1..];
            }
        }
    }
    out.push_str(rest);
    out
}

/// Builds the merged text. Output uses CRLF line endings.
pub fn render(segments: &[Segment], opts: &CombineOptions) -> String {
    let joiner = match (opts.no_returns, opts.add_space) {
        (false, _) => "\r\n\r\n",
        (true, true) => " ",
        (true, false) => "",
    };
    let mut dir_totals: HashMap<Option<&Path>, usize> = HashMap::new();
    for seg in segments {
        *dir_totals.entry(seg.path.parent()).or_default() += 1;
    }
    let mut dir_seen: HashMap<Option<&Path>, usize> = HashMap::new();
    let mut out = String::new();
    for (i, seg) in segments.iter().enumerate() {
        let dir = seg.path.parent();
        let dir_index = {
            let seen = dir_seen.entry(dir).or_default();
            *seen += 1;
            *seen
        };
        let ctx = VarContext {
            path: &seg.path,
            index: i + 1,
            dir_index,
            dir_files: dir_totals[&dir],
            sibling_dirs: seg.sibling_dirs,
            items: segments.len(),
        };
        let mut header = String::new();
        if !opts.no_filename {
            header.push_str(&seg.name);
            header.push_str("\r\n");
        }
        if !opts.no_separator {
            header.push_str(&expand_variables(&opts.separator, &ctx));
            header.push_str("\r\n");
        }
        if i > 0 {
            // A header always starts on its own line, even in continuous mode.
            if !header.is_empty() && opts.no_returns {
                out.push_str("\r\n");
            } else {
                out.push_str(joiner);
            }
        }
        out.push_str(&header);
        match &seg.content {
            Some(text) => out.push_str(&normalize_newlines(text.trim_end_matches(['\r', '\n']))),
            None => out.push_str(ERROR_LINE),
        }
    }
    out.push_str("\r\n");
    out
}

fn normalize_newlines(text: &str) -> String {
    text.replace("\r\n", "\n").replace('\r', "\n").replace('\n', "\r\n")
}

/// Returns `dir/name`, or `dir/stem (2).ext`, `(3)`... if that file exists.
pub fn unique_output_path(dir: &Path, name: &str) -> PathBuf {
    let first = dir.join(name);
    if !first.exists() {
        return first;
    }
    let p = Path::new(name);
    let stem = p.file_stem().and_then(|s| s.to_str()).unwrap_or("combined");
    let ext = p.extension().and_then(|s| s.to_str());
    (2..)
        .map(|n| match ext {
            Some(e) => dir.join(format!("{stem} ({n}).{e}")),
            None => dir.join(format!("{stem} ({n})")),
        })
        .find(|c| !c.exists())
        .expect("unbounded range always yields a free name")
}

fn count_dirs(dir: &Path) -> usize {
    fs::read_dir(dir)
        .map(|entries| entries.filter_map(Result::ok).filter(|e| e.path().is_dir()).count())
        .unwrap_or(0)
}

pub struct CombineReport {
    pub output: PathBuf,
    pub merged: usize,
    pub skipped: Vec<String>,
}

/// Reads `files`, merges them and writes the result into `root` without
/// overwriting an existing file.
pub fn combine_to_folder(
    root: &Path,
    files: &[PathBuf],
    output_name: &str,
    opts: &CombineOptions,
) -> io::Result<CombineReport> {
    let mut skipped = Vec::new();
    let mut dir_counts: HashMap<PathBuf, usize> = HashMap::new();
    let segments: Vec<Segment> = files
        .iter()
        .map(|f| {
            let name = display_path(root, f);
            let content = fs::read(f).ok().and_then(|b| decode_text(&b));
            if content.is_none() {
                skipped.push(name.clone());
            }
            let parent = f.parent().unwrap_or(root).to_path_buf();
            let sibling_dirs = *dir_counts.entry(parent).or_insert_with_key(|p| count_dirs(p));
            Segment { name, path: f.clone(), sibling_dirs, content }
        })
        .collect();
    let output = unique_output_path(root, output_name);
    fs::write(&output, render(&segments, opts))?;
    Ok(CombineReport { output, merged: files.len() - skipped.len(), skipped })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seg(name: &str, text: &str) -> Segment {
        Segment { name: name.into(), path: name.into(), sibling_dirs: 0, content: Some(text.into()) }
    }

    fn ctx(path: &Path) -> VarContext<'_> {
        VarContext { path, index: 28, dir_index: 3, dir_files: 5, sibling_dirs: 2, items: 40 }
    }

    #[test]
    fn every_separator_variable_is_replaced() {
        let path = Path::new("C:/docs/Notas/Informe final 2024.v2.md");
        let c = ctx(path);
        let cases = [
            ("<Inc Nr>", "28"),
            ("<Inc NrDir>", "3"),
            ("<Inc Alpha>", "AB"),
            ("<Name>", "Informe final 2024.v2"),
            ("<Ext>", "md"),
            ("<FolderName>", "Notas"),
            ("<Num Files>", "5"),
            ("<Num Dirs>", "2"),
            ("<Num Items>", "40"),
            ("<Word:1>", "Informe"),
            ("<Word:3>", "2024"),
            ("<Word>", "Informe"),
            ("<RWord:1>", "v2"),
            ("<RWord:2>", "2024"),
            ("<Word:9>", ""),
            ("<MediaType>", "text/markdown"),
        ];
        for (template, expected) in cases {
            assert_eq!(expand_variables(template, &c), expected, "{template}");
        }
    }

    #[test]
    fn editable_values_change_the_start_or_level() {
        // index 28, dir_index 3 in `ctx`.
        let c = ctx(Path::new("C:/docs/Notas/Mayo/b.txt"));
        let cases = [
            ("<Inc Nr:1>", "28"),
            ("<Inc Nr:10>", "37"),
            ("<Inc Nr:0>", "27"),
            ("<Inc Nr:001>", "028"),
            ("<Inc Nr:>", "28"),
            ("<Inc Alpha: >", "AB"),
            ("<FolderName:>", "Mayo"),
            ("<Inc NrDir:1>", "3"),
            ("<Inc NrDir:100>", "102"),
            ("<Inc Alpha:A>", "AB"),
            ("<Inc Alpha:C>", "AD"),
            ("<Inc Alpha:a>", "ab"),
            ("<FolderName:1>", "Mayo"),
            ("<FolderName:2>", "Notas"),
            ("<FolderName:9>", ""),
            ("<Word:2>", ""),
        ];
        for (template, expected) in cases {
            assert_eq!(expand_variables(template, &c), expected, "{template}");
        }
        // Invalid values leave the tag as written.
        for bad in ["<Inc Nr:x>", "<Inc Alpha:1>", "<FolderName:0>", "<Ext:2>", "<MediaType:1>"] {
            assert_eq!(expand_variables(bad, &c), bad);
        }
    }

    #[test]
    fn start_values_count_from_the_first_file() {
        let o = CombineOptions {
            no_filename: true,
            separator: "<Inc Nr:10> <Inc Alpha:C>".into(),
            ..Default::default()
        };
        let segs = [seg("a.txt", "1"), seg("b.txt", "2")];
        assert_eq!(render(&segs, &o), "10 C\r\n1\r\n\r\n11 D\r\n2\r\n");
    }

    #[test]
    fn read_preview_limits_size_without_breaking_characters() {
        let dir = tempfile::tempdir().unwrap();
        let small = dir.path().join("a.txt");
        fs::write(&small, "hola ñandú").unwrap();
        let read = read_preview(&small, 1000).unwrap();
        assert_eq!((read.text.as_str(), read.truncated), ("hola ñandú", false));
        assert_eq!(read.encoding, TextEncoding::Utf8);

        // "ñ" is two bytes; a limit of 6 would cut it in half.
        let cut = dir.path().join("b.txt");
        fs::write(&cut, "hola ñandú").unwrap();
        let read = read_preview(&cut, 6).unwrap();
        assert_eq!((read.text.as_str(), read.truncated), ("hola ", true));

        let bin = dir.path().join("c.dat");
        fs::write(&bin, b"\0\x01\x02").unwrap();
        assert_eq!(read_preview(&bin, 1000), None);
        assert_eq!(read_preview(&dir.path().join("missing.txt"), 1000), None);
    }

    #[test]
    fn saving_keeps_the_original_encoding() {
        let dir = tempfile::tempdir().unwrap();
        let cases: [(&[u8], TextEncoding); 5] = [
            (b"caf\xC3\xA9", TextEncoding::Utf8),
            (b"\xEF\xBB\xBFcaf\xC3\xA9", TextEncoding::Utf8Bom),
            (b"\xFF\xFEc\0a\0f\0\xE9\0", TextEncoding::Utf16Le),
            (b"\xFE\xFF\0c\0a\0f\0\xE9", TextEncoding::Utf16Be),
            (b"caf\xE9", TextEncoding::Windows1252),
        ];
        for (i, (original, encoding)) in cases.into_iter().enumerate() {
            let path = dir.path().join(format!("{i}.txt"));
            fs::write(&path, original).unwrap();
            let read = read_preview(&path, 1000).unwrap();
            assert_eq!((read.text.as_str(), read.encoding), ("café", encoding));

            let used = save_text(&path, "café ñ", read.encoding).unwrap();
            assert_eq!(used, encoding);
            let again = read_preview(&path, 1000).unwrap();
            assert_eq!((again.text.as_str(), again.encoding), ("café ñ", encoding));
        }
    }

    #[test]
    fn ansi_text_with_emoji_is_saved_as_utf8_bom() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("a.txt");
        assert_eq!(save_text(&path, "hola 🙂", TextEncoding::Windows1252).unwrap(), TextEncoding::Utf8Bom);
        assert_eq!(fs::read(&path).unwrap(), "\u{FEFF}hola 🙂".as_bytes());
    }

    #[test]
    fn media_type_follows_the_extension() {
        assert_eq!(media_type(Path::new("a.md")), "text/markdown");
        assert_eq!(media_type(Path::new("a.JSON")), "application/json");
        assert_eq!(media_type(Path::new("a.weird")), "text/plain");
        assert_eq!(media_type(Path::new("noext")), "text/plain");
    }

    #[test]
    fn variables_mix_with_text_and_unknown_tags_stay() {
        let c = ctx(Path::new("C:/a/b.txt"));
        assert_eq!(expand_variables("== <Inc Nr>/<Num Items> - <name> ==", &c), "== 28/40 - b ==");
        assert_eq!(expand_variables("<---->", &c), "<---->");
        assert_eq!(expand_variables("<<Name>>", &c), "<b>");
        assert_eq!(expand_variables("<Word:0> <Name:2> <Inc", &c), "<Word:0> <Name:2> <Inc");
    }

    #[test]
    fn alpha_counts_like_spreadsheet_columns() {
        assert_eq!(alpha(1), "A");
        assert_eq!(alpha(26), "Z");
        assert_eq!(alpha(27), "AA");
        assert_eq!(alpha(52), "AZ");
        assert_eq!(alpha(703), "AAA");
    }

    #[test]
    fn render_numbers_files_overall_and_per_folder() {
        let o = CombineOptions {
            no_filename: true,
            separator: "<Inc Nr>.<Inc NrDir> <FolderName>/<Name> (<Num Files> of <Num Items>)".into(),
            ..Default::default()
        };
        let segs = [seg("x/a.txt", "1"), seg("x/b.txt", "2"), seg("y/c.txt", "3")];
        assert_eq!(
            render(&segs, &o),
            "1.1 x/a (2 of 3)\r\n1\r\n\r\n2.2 x/b (2 of 3)\r\n2\r\n\r\n3.1 y/c (1 of 3)\r\n3\r\n"
        );
    }

    fn opts() -> CombineOptions {
        CombineOptions { separator: "---".into(), ..Default::default() }
    }

    #[test]
    fn default_writes_name_separator_and_blank_line_between_files() {
        let out = render(&[seg("a.txt", "one\n"), seg("b.md", "two")], &opts());
        assert_eq!(out, "a.txt\r\n---\r\none\r\n\r\nb.md\r\n---\r\ntwo\r\n");
    }

    #[test]
    fn no_separator_drops_only_the_separator_line() {
        let o = CombineOptions { no_separator: true, ..opts() };
        let out = render(&[seg("a.txt", "one"), seg("b.txt", "two")], &o);
        assert_eq!(out, "a.txt\r\none\r\n\r\nb.txt\r\ntwo\r\n");
    }

    #[test]
    fn no_filename_drops_only_the_name_line() {
        let o = CombineOptions { no_filename: true, ..opts() };
        let out = render(&[seg("a.txt", "one"), seg("b.txt", "two")], &o);
        assert_eq!(out, "---\r\none\r\n\r\n---\r\ntwo\r\n");
    }

    #[test]
    fn no_headers_and_no_returns_gives_continuous_text() {
        let o = CombineOptions { no_filename: true, no_separator: true, no_returns: true, ..opts() };
        let out = render(&[seg("a", "one\r\n"), seg("b", "two\n"), seg("c", "three")], &o);
        assert_eq!(out, "onetwothree\r\n");
    }

    #[test]
    fn add_space_puts_one_space_between_continuous_segments() {
        let o = CombineOptions {
            no_filename: true,
            no_separator: true,
            no_returns: true,
            add_space: true,
            ..opts()
        };
        let out = render(&[seg("a", "one\n"), seg("b", "two")], &o);
        assert_eq!(out, "one two\r\n");
    }

    #[test]
    fn add_space_has_no_effect_without_no_returns() {
        let o = CombineOptions { no_filename: true, no_separator: true, add_space: true, ..opts() };
        let out = render(&[seg("a", "one"), seg("b", "two")], &o);
        assert_eq!(out, "one\r\n\r\ntwo\r\n");
    }

    #[test]
    fn no_returns_with_headers_keeps_header_on_its_own_line() {
        let o = CombineOptions { no_separator: true, no_returns: true, ..opts() };
        let out = render(&[seg("a", "one\n"), seg("b", "two")], &o);
        assert_eq!(out, "a\r\none\r\nb\r\ntwo\r\n");
    }

    #[test]
    fn unreadable_file_writes_error_line() {
        let missing = Segment { name: "x.bin".into(), path: "x.bin".into(), sibling_dirs: 0, content: None };
        let out = render(&[missing], &opts());
        assert_eq!(out, format!("x.bin\r\n---\r\n{ERROR_LINE}\r\n"));
    }

    #[test]
    fn decodes_utf8_bom_utf16_and_ansi_and_rejects_binary() {
        assert_eq!(decode_text(b"\xEF\xBB\xBFhola").as_deref(), Some("hola"));
        assert_eq!(decode_text(b"\xFF\xFEh\0i\0").as_deref(), Some("hi"));
        assert_eq!(decode_text(b"caf\xE9").as_deref(), Some("café"));
        assert_eq!(decode_text("ñandú".as_bytes()).as_deref(), Some("ñandú"));
        assert_eq!(decode_text(b"MZ\0\0\x90"), None);
    }

    #[test]
    fn normalize_extension_accepts_dots_and_globs_only_for_one_extension() {
        assert_eq!(normalize_extension(" *.ADOC ").as_deref(), Some("adoc"));
        assert_eq!(normalize_extension("nfo").as_deref(), Some("nfo"));
        assert_eq!(normalize_extension("tex, nfo"), None);
        assert_eq!(normalize_extension("  "), None);
    }

    #[test]
    fn dropping_files_lists_only_those_files() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        fs::create_dir(root.join("sub")).unwrap();
        for f in ["a.txt", "b.md", "other.txt", "sub/c.txt"] {
            fs::write(root.join(f), "texto").unwrap();
        }
        fs::write(root.join("img.png"), b"\x89PNG\0\0").unwrap();

        // One file: only that file, results saved next to it.
        let one = classify_drop(&[root.join("a.txt")]).unwrap();
        assert_eq!(one, Dropped::Files { root: root.to_path_buf(), files: vec![root.join("a.txt")] });

        // Files from different subfolders share their closest common folder.
        let many = vec![root.join("sub").join("c.txt"), root.join("b.md"), root.join("img.png"), root.join("b.md")];
        let Some(Dropped::Files { root: common, files }) = classify_drop(&many) else { panic!("expected files") };
        assert_eq!(common, root);
        let listed: Vec<_> = text_files_only(&common, &files).iter().map(|p| display_path(&common, p)).collect();
        assert_eq!(listed, vec!["b.md", &format!("sub{}c.txt", std::path::MAIN_SEPARATOR)]);
    }

    #[test]
    fn dropping_a_folder_lists_the_whole_folder() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        fs::create_dir(root.join("sub")).unwrap();
        fs::write(root.join("a.txt"), "x").unwrap();
        assert_eq!(classify_drop(&[root.join("sub")]), Some(Dropped::Folder(root.join("sub"))));
        // A folder dropped together with files still means "the whole folder".
        assert_eq!(
            classify_drop(&[root.join("a.txt"), root.join("sub")]),
            Some(Dropped::Folder(root.join("sub")))
        );
        assert_eq!(classify_drop(&[]), None);
        assert_eq!(classify_drop(&[root.join("missing.txt")]), None);
    }

    #[test]
    fn scan_finds_text_files_of_any_extension_and_skips_binary() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        fs::create_dir(root.join("sub")).unwrap();
        for f in ["b.markdown", "a.MD", "c.weird", "noext", "combined.md", "sub/d.txt"] {
            fs::write(root.join(f), "plain text\r\n").unwrap();
        }
        fs::write(root.join("e.png"), b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR").unwrap();
        fs::write(root.join("f.pdf"), "%PDF-1.7 looks like text").unwrap();
        fs::write(root.join("g.dat"), b"\x01\x02\x03\x04\x05\x06abc").unwrap();
        fs::write(root.join("h.txt"), b"\xFF\xFEh\0i\0").unwrap();
        let skip = |p: &Path| p.file_name().is_some_and(|n| n == "combined.md");
        let names = |v: Vec<PathBuf>| v.iter().map(|p| display_path(root, p)).collect::<Vec<_>>();

        assert_eq!(
            names(scan(root, false, &skip)),
            vec!["a.MD", "b.markdown", "c.weird", "h.txt", "noext"]
        );
        let deep = names(scan(root, true, &skip));
        assert_eq!(deep.len(), 6);
        assert!(deep.iter().any(|n| n.ends_with("d.txt")));
    }

    #[test]
    fn combine_writes_into_the_folder_without_overwriting() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        fs::write(root.join("a.txt"), "one").unwrap();
        fs::write(root.join("b.bin"), b"\0\x01").unwrap();
        fs::write(root.join("combined.txt"), "old").unwrap();
        let files = vec![root.join("a.txt"), root.join("b.bin")];

        let report = combine_to_folder(root, &files, "combined.txt", &opts()).unwrap();

        assert_eq!(report.output, root.join("combined (2).txt"));
        assert_eq!(report.merged, 1);
        assert_eq!(report.skipped, vec!["b.bin"]);
        assert_eq!(fs::read_to_string(root.join("combined.txt")).unwrap(), "old");
        let written = fs::read_to_string(&report.output).unwrap();
        assert!(written.starts_with("a.txt\r\n---\r\none\r\n\r\nb.bin\r\n---\r\n[Error]"));
    }
}
