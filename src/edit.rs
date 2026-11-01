//! Bulk text edits used by the "Edit" tab: the same changes applied to many
//! files at once.

use std::fs;
use std::io;
use std::ops::RangeInclusive;
use std::path::{Path, PathBuf};

use regex::{NoExpand, Regex, RegexBuilder};

use crate::{decode_with_encoding, display_path, save_text, unique_output_path};

/// What the user switched on in the Edit tab. Steps run in field order.
#[derive(Clone, Debug, PartialEq, Hash)]
pub struct EditOptions {
    pub delete_line_numbers: bool,
    /// e.g. "1-3, 10, 20-" (20- means from line 20 to the end).
    pub line_numbers: String,
    pub delete_containing: bool,
    pub delete_text: String,
    /// When a line is deleted, also delete the empty line right after it,
    /// so no blank gap is left where it was.
    pub delete_blank_after: bool,
    pub replace: bool,
    pub find: String,
    pub replace_with: String,
    /// Shared by "delete lines containing" and "replace".
    pub match_case: bool,
    pub use_regex: bool,
    pub trim_lines: bool,
    pub remove_empty: bool,
    /// Lines with only spaces or tabs count as empty.
    pub whitespace_is_empty: bool,
    pub collapse_empty: bool,
}

impl Default for EditOptions {
    fn default() -> Self {
        Self {
            delete_line_numbers: false,
            // Starts with a valid value so switching the step on shows no error.
            line_numbers: "1".to_owned(),
            delete_containing: false,
            delete_text: String::new(),
            delete_blank_after: true,
            replace: false,
            find: String::new(),
            replace_with: String::new(),
            match_case: false,
            use_regex: false,
            trim_lines: false,
            remove_empty: false,
            whitespace_is_empty: true,
            collapse_empty: false,
        }
    }
}

impl EditOptions {
    /// True when at least one step is switched on.
    pub fn any(&self) -> bool {
        self.delete_line_numbers
            || self.delete_containing
            || self.replace
            || self.trim_lines
            || self.remove_empty
            || self.collapse_empty
    }
}

#[derive(Debug, PartialEq)]
pub enum EditError {
    /// The part of the line-number list that could not be understood.
    LineNumbers(String),
    /// The regular expression is invalid; holds the parser message.
    Pattern(String),
    EmptyDeleteText,
    EmptyFind,
}

/// Parses "1-3, 10, 20-" into line ranges (1-based, inclusive).
pub fn parse_line_numbers(spec: &str) -> Result<Vec<RangeInclusive<usize>>, EditError> {
    let bad = |part: &str| EditError::LineNumbers(part.to_owned());
    let parts: Vec<&str> = spec.split(',').map(str::trim).filter(|p| !p.is_empty()).collect();
    if parts.is_empty() {
        return Err(bad(""));
    }
    parts
        .into_iter()
        .map(|part| {
            let number = |s: &str| s.trim().parse::<usize>().ok().filter(|n| *n > 0);
            match part.split_once('-') {
                Some((from, to)) => {
                    let from = number(from).ok_or_else(|| bad(part))?;
                    let to = if to.trim().is_empty() { usize::MAX } else { number(to).ok_or_else(|| bad(part))? };
                    if to < from {
                        return Err(bad(part));
                    }
                    Ok(from..=to)
                }
                None => number(part).map(|n| n..=n).ok_or_else(|| bad(part)),
            }
        })
        .collect()
}

/// Edit steps ready to run, with patterns already compiled.
pub struct Edits {
    line_ranges: Option<Vec<RangeInclusive<usize>>>,
    delete: Option<Regex>,
    replace: Option<(Regex, String)>,
    expand_groups: bool,
    delete_blank_after: bool,
    trim_lines: bool,
    remove_empty: bool,
    collapse_empty: bool,
    whitespace_is_empty: bool,
}

impl Edits {
    pub fn compile(o: &EditOptions) -> Result<Self, EditError> {
        let matcher = |pattern: &str| {
            let pattern = if o.use_regex { pattern.to_owned() } else { regex::escape(pattern) };
            RegexBuilder::new(&pattern)
                .case_insensitive(!o.match_case)
                .multi_line(true)
                .build()
                .map_err(|e| EditError::Pattern(e.to_string()))
        };
        let line_ranges =
            if o.delete_line_numbers { Some(parse_line_numbers(&o.line_numbers)?) } else { None };
        let delete = if o.delete_containing {
            if o.delete_text.is_empty() {
                return Err(EditError::EmptyDeleteText);
            }
            Some(matcher(&o.delete_text)?)
        } else {
            None
        };
        let replace = if o.replace {
            if o.find.is_empty() {
                return Err(EditError::EmptyFind);
            }
            Some((matcher(&o.find)?, o.replace_with.clone()))
        } else {
            None
        };
        Ok(Self {
            line_ranges,
            delete,
            replace,
            expand_groups: o.use_regex,
            delete_blank_after: o.delete_blank_after,
            trim_lines: o.trim_lines,
            remove_empty: o.remove_empty,
            collapse_empty: o.collapse_empty,
            whitespace_is_empty: o.whitespace_is_empty,
        })
    }

    /// Applies every step to `text`, which must use `\n` line endings.
    /// A final line break is kept when the text had one.
    pub fn apply(&self, text: &str) -> String {
        let (body, trailing_newline) = match text.strip_suffix('\n') {
            Some(body) => (body, true),
            None => (text, false),
        };
        let original: Vec<&str> = body.split('\n').collect();
        // Line numbers refer to the original file, so mark deletions on it first.
        let by_rule: Vec<bool> = original
            .iter()
            .enumerate()
            .map(|(i, line)| {
                self.line_ranges.as_ref().is_some_and(|rs| rs.iter().any(|r| r.contains(&(i + 1))))
                    || self.delete.as_ref().is_some_and(|re| re.is_match(line))
            })
            .collect();
        let mut deleted = by_rule.clone();
        if self.delete_blank_after {
            // Only one empty line per deleted line, so paragraph gaps elsewhere stay.
            for i in 0..original.len().saturating_sub(1) {
                if by_rule[i] && original[i + 1].trim().is_empty() {
                    deleted[i + 1] = true;
                }
            }
        }
        let lines: Vec<&str> =
            original.iter().zip(&deleted).filter(|(_, d)| !**d).map(|(l, _)| *l).collect();
        if lines.is_empty() {
            return String::new();
        }

        let mut text = lines.join("\n");
        if let Some((re, with)) = &self.replace {
            text = if self.expand_groups {
                re.replace_all(&text, with.as_str()).into_owned()
            } else {
                re.replace_all(&text, NoExpand(with)).into_owned()
            };
        }

        let mut lines: Vec<&str> =
            text.split('\n').map(|l| if self.trim_lines { l.trim() } else { l }).collect();
        let is_empty = |l: &str| if self.whitespace_is_empty { l.trim().is_empty() } else { l.is_empty() };
        if self.remove_empty {
            lines.retain(|l| !is_empty(l));
        } else if self.collapse_empty {
            let mut previous_empty = false;
            lines.retain(|l| {
                let empty = is_empty(l);
                let keep = !(empty && previous_empty);
                previous_empty = empty;
                keep
            });
        }
        let mut out = lines.join("\n");
        if trailing_newline && !lines.is_empty() {
            out.push('\n');
        }
        out
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Change {
    Same,
    Removed,
    Added,
}

/// One line of a before/after comparison.
#[derive(Clone, Debug, PartialEq)]
pub struct DiffLine {
    pub change: Change,
    /// Line number in the original (1-based); `None` for added lines.
    pub old: Option<usize>,
    /// Line number in the result (1-based); `None` for removed lines.
    pub new: Option<usize>,
    pub text: String,
}

/// Line-by-line comparison of `old` and `new`, like a GitHub diff.
pub fn diff_lines(old: &str, new: &str) -> Vec<DiffLine> {
    similar::TextDiff::from_lines(old, new)
        .iter_all_changes()
        .map(|c| DiffLine {
            change: match c.tag() {
                similar::ChangeTag::Equal => Change::Same,
                similar::ChangeTag::Delete => Change::Removed,
                similar::ChangeTag::Insert => Change::Added,
            },
            old: c.old_index().map(|i| i + 1),
            new: c.new_index().map(|i| i + 1),
            text: c.value().trim_end_matches(['\n', '\r']).to_owned(),
        })
        .collect()
}

#[derive(Debug, PartialEq)]
pub enum DiffRow<'a> {
    Line(&'a DiffLine),
    /// This many unchanged lines are folded away.
    Hidden(usize),
}

/// Keeps changed lines plus `context` unchanged lines around each change,
/// folding the rest like GitHub does.
pub fn only_changes(lines: &[DiffLine], context: usize) -> Vec<DiffRow<'_>> {
    let mut keep = vec![false; lines.len()];
    for (i, line) in lines.iter().enumerate() {
        if line.change != Change::Same {
            let end = (i + context).min(lines.len() - 1);
            keep[i.saturating_sub(context)..=end].iter_mut().for_each(|k| *k = true);
        }
    }
    let mut rows = Vec::new();
    let mut hidden = 0;
    for (line, keep) in lines.iter().zip(keep) {
        if keep {
            if hidden > 0 {
                rows.push(DiffRow::Hidden(hidden));
                hidden = 0;
            }
            rows.push(DiffRow::Line(line));
        } else {
            hidden += 1;
        }
    }
    if hidden > 0 {
        rows.push(DiffRow::Hidden(hidden));
    }
    rows
}

pub struct EditReport {
    pub changed: usize,
    pub unchanged: usize,
    pub failed: Vec<String>,
    /// The new folder the copies went into, when not overwriting.
    pub copy_folder: Option<PathBuf>,
}

/// Applies `edits` to `files`. With `copy_folder`, results go into a new
/// subfolder of `root` (keeping the subfolder layout, never overwriting an
/// existing folder); otherwise changed files are overwritten in place.
/// Each file keeps its encoding and line endings.
pub fn edit_files(
    root: &Path,
    files: &[PathBuf],
    edits: &Edits,
    copy_folder: Option<&str>,
) -> io::Result<EditReport> {
    let dest = match copy_folder {
        Some(name) => {
            let dir = unique_output_path(root, name);
            fs::create_dir_all(&dir)?;
            Some(dir)
        }
        None => None,
    };
    let mut report = EditReport { changed: 0, unchanged: 0, failed: Vec::new(), copy_folder: dest.clone() };
    for file in files {
        let name = display_path(root, file);
        let Some((text, encoding)) = fs::read(file).ok().and_then(|b| decode_with_encoding(&b)) else {
            report.failed.push(name);
            continue;
        };
        let crlf = text.contains("\r\n");
        let original = text.replace("\r\n", "\n");
        let edited = edits.apply(&original);
        let changed = edited != original;
        let out = if crlf { edited.replace('\n', "\r\n") } else { edited };

        let target = match &dest {
            Some(dir) => dir.join(file.strip_prefix(root).unwrap_or(Path::new(file.file_name().unwrap_or_default()))),
            None => file.clone(),
        };
        if dest.is_some() || changed {
            let written = target
                .parent()
                .map_or(Ok(()), fs::create_dir_all)
                .and_then(|_| save_text(&target, &out, encoding));
            if written.is_err() {
                report.failed.push(name);
                continue;
            }
        }
        if changed {
            report.changed += 1;
        } else {
            report.unchanged += 1;
        }
    }
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(o: EditOptions, text: &str) -> String {
        Edits::compile(&o).unwrap().apply(text)
    }

    #[test]
    fn parses_line_number_lists() {
        assert_eq!(parse_line_numbers("1-3, 10, 20-").unwrap(), vec![1..=3, 10..=10, 20..=usize::MAX]);
        assert_eq!(parse_line_numbers(" 5 ").unwrap(), vec![5..=5]);
        assert_eq!(parse_line_numbers("3-1"), Err(EditError::LineNumbers("3-1".into())));
        assert_eq!(parse_line_numbers("0"), Err(EditError::LineNumbers("0".into())));
        assert_eq!(parse_line_numbers("a, 2"), Err(EditError::LineNumbers("a".into())));
        assert_eq!(parse_line_numbers(" , "), Err(EditError::LineNumbers("".into())));
    }

    #[test]
    fn deletes_lines_by_number() {
        let o = EditOptions { delete_line_numbers: true, line_numbers: "1, 3-4, 6-".into(), ..Default::default() };
        assert_eq!(run(o, "a\nb\nc\nd\ne\nf\ng\n"), "b\ne\n");
    }

    #[test]
    fn deletes_lines_containing_text_ignoring_case_by_default() {
        let o = EditOptions { delete_containing: true, delete_text: "file-".into(), ..Default::default() };
        assert_eq!(run(o.clone(), "# File-abc\nhola\nfile-x\nadiós\n"), "hola\nadiós\n");
        let exact = EditOptions { match_case: true, ..o };
        assert_eq!(run(exact, "# File-abc\nfile-x\n"), "# File-abc\n");
    }

    #[test]
    fn replaces_text_literally_or_with_regex() {
        let o = EditOptions { replace: true, find: "a.b".into(), replace_with: "$0!".into(), ..Default::default() };
        assert_eq!(run(o, "A.B axb"), "$0! axb");
        let re = EditOptions {
            replace: true,
            find: r"(\d+)-(\d+)".into(),
            replace_with: "$2/$1".into(),
            use_regex: true,
            ..Default::default()
        };
        assert_eq!(run(re, "10-20 y 3-4"), "20/10 y 4/3");
    }

    #[test]
    fn trims_lines_and_removes_empty_ones() {
        let trim = EditOptions { trim_lines: true, ..Default::default() };
        assert_eq!(run(trim, "  a  \n\tb\n"), "a\nb\n");
        let empty = EditOptions { remove_empty: true, ..Default::default() };
        assert_eq!(run(empty.clone(), "a\n\n   \nb\n\n"), "a\nb\n");
        let strict = EditOptions { whitespace_is_empty: false, ..empty };
        assert_eq!(run(strict, "a\n\n   \nb\n"), "a\n   \nb\n");
    }

    #[test]
    fn collapses_runs_of_empty_lines_to_one() {
        let o = EditOptions { collapse_empty: true, ..Default::default() };
        assert_eq!(run(o, "a\n\n\n \nb\n\nc\n"), "a\n\nb\n\nc\n");
    }

    #[test]
    fn steps_run_in_order_and_keep_final_newline() {
        // Line numbers refer to the original file; replace runs before emptiness checks.
        let o = EditOptions {
            delete_line_numbers: true,
            line_numbers: "1".into(),
            replace: true,
            find: "x".into(),
            replace_with: "".into(),
            remove_empty: true,
            ..Default::default()
        };
        assert_eq!(run(o.clone(), "title\nx\nkeep\n"), "keep\n");
        assert_eq!(run(o, "title\nx\nkeep"), "keep");
    }

    #[test]
    fn deleting_every_line_leaves_an_empty_file() {
        let o = EditOptions { delete_line_numbers: true, line_numbers: "1-".into(), ..Default::default() };
        assert_eq!(run(o, "a\nb\n"), "");
    }

    #[test]
    fn invalid_settings_are_reported() {
        let bad_regex = EditOptions { replace: true, find: "(".into(), use_regex: true, ..Default::default() };
        assert!(matches!(Edits::compile(&bad_regex), Err(EditError::Pattern(_))));
        let no_find = EditOptions { replace: true, ..Default::default() };
        assert!(matches!(Edits::compile(&no_find), Err(EditError::EmptyFind)));
        let no_text = EditOptions { delete_containing: true, ..Default::default() };
        assert!(matches!(Edits::compile(&no_text), Err(EditError::EmptyDeleteText)));
        assert!(!EditOptions::default().any());
    }

    #[test]
    fn deleting_a_line_also_deletes_the_empty_line_after_it() {
        let o = EditOptions { delete_line_numbers: true, line_numbers: "1".into(), ..Default::default() };
        assert_eq!(run(o.clone(), "# title\n\nCortas una cebolla.\n\nNo es que pique.\n"), "Cortas una cebolla.\n\nNo es que pique.\n");
        // Only the one line right after; a run of empty lines keeps the rest.
        assert_eq!(run(o.clone(), "# title\n\n\nbody\n"), "\nbody\n");
        // Same for lines deleted because they contain a text.
        let containing = EditOptions { delete_containing: true, delete_text: "borrar".into(), ..Default::default() };
        assert_eq!(run(containing, "a\n\nborrar esto\n   \nb\n"), "a\n\nb\n");
        // Switched off, the empty line stays.
        let keep = EditOptions { delete_blank_after: false, ..o };
        assert_eq!(run(keep, "# title\n\nbody\n"), "\nbody\n");
    }

    #[test]
    fn delete_by_number_works_right_away_with_its_default_of_line_1() {
        let o = EditOptions { delete_line_numbers: true, ..Default::default() };
        assert_eq!(run(o, "# title\nbody\n"), "body\n");
    }

    #[test]
    fn diff_marks_removed_and_added_lines_with_both_numbers() {
        let diff = diff_lines("# title\nuno\ndos\n", "uno\nDOS\n");
        let summary: Vec<_> = diff.iter().map(|d| (d.change, d.old, d.new, d.text.as_str())).collect();
        assert_eq!(
            summary,
            vec![
                (Change::Removed, Some(1), None, "# title"),
                (Change::Same, Some(2), Some(1), "uno"),
                (Change::Removed, Some(3), None, "dos"),
                (Change::Added, None, Some(2), "DOS"),
            ]
        );
        assert!(diff_lines("a\nb\n", "a\nb\n").iter().all(|d| d.change == Change::Same));
    }

    #[test]
    fn only_changes_folds_unchanged_lines_outside_the_context() {
        let old: String = (1..=10).map(|n| format!("{n}\n")).collect();
        let new = old.replace("5\n", "");
        let diff = diff_lines(&old, &new);
        let rows = only_changes(&diff, 1);
        let shown: Vec<String> = rows
            .iter()
            .map(|r| match r {
                DiffRow::Line(l) => format!("{:?} {}", l.change, l.text),
                DiffRow::Hidden(n) => format!("hidden {n}"),
            })
            .collect();
        assert_eq!(shown, vec!["hidden 3", "Same 4", "Removed 5", "Same 6", "hidden 4"]);
        assert_eq!(only_changes(&diff_lines("a\n", "a\n"), 3), vec![DiffRow::Hidden(1)]);
    }

    #[test]
    fn overwrite_mode_rewrites_only_changed_files_keeping_crlf_and_ansi() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        fs::write(root.join("a.txt"), b"caf\xE9\r\n\r\nfin\r\n").unwrap();
        fs::write(root.join("b.txt"), "sin cambios\n").unwrap();
        fs::write(root.join("c.bin"), b"\0\x01").unwrap();
        let edits = Edits::compile(&EditOptions { remove_empty: true, ..Default::default() }).unwrap();
        let files = vec![root.join("a.txt"), root.join("b.txt"), root.join("c.bin")];

        let report = edit_files(root, &files, &edits, None).unwrap();

        assert_eq!((report.changed, report.unchanged), (1, 1));
        assert_eq!(report.failed, vec!["c.bin"]);
        assert!(report.copy_folder.is_none());
        assert_eq!(fs::read(root.join("a.txt")).unwrap(), b"caf\xE9\r\nfin\r\n");
        assert_eq!(fs::read_to_string(root.join("b.txt")).unwrap(), "sin cambios\n");
    }

    #[test]
    fn copy_mode_writes_every_file_into_a_new_folder_and_leaves_originals() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        fs::create_dir(root.join("sub")).unwrap();
        fs::create_dir(root.join("editados")).unwrap();
        fs::write(root.join("a.txt"), "a\n\nb\n").unwrap();
        fs::write(root.join("sub/c.txt"), "c\n").unwrap();
        let edits = Edits::compile(&EditOptions { remove_empty: true, ..Default::default() }).unwrap();
        let files = vec![root.join("a.txt"), root.join("sub/c.txt")];

        let report = edit_files(root, &files, &edits, Some("editados")).unwrap();

        let out = root.join("editados (2)");
        assert_eq!(report.copy_folder.as_deref(), Some(out.as_path()));
        assert_eq!((report.changed, report.unchanged), (1, 1));
        assert_eq!(fs::read_to_string(out.join("a.txt")).unwrap(), "a\nb\n");
        assert_eq!(fs::read_to_string(out.join("sub/c.txt")).unwrap(), "c\n");
        assert_eq!(fs::read_to_string(root.join("a.txt")).unwrap(), "a\n\nb\n");
    }
}
