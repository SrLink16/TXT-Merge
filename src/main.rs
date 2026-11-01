#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod theme;

use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver};
use std::thread;

use eframe::egui;
use txt_merge::edit::{
    Change, DiffLine, DiffRow, EditError, EditOptions, EditReport, Edits, diff_lines, edit_files,
    only_changes,
};
use txt_merge::{
    CombineOptions, CombineReport, EXPORT_EXTENSIONS, PRESET_SEPARATORS, SEPARATOR_VARIABLES,
    Segment, TextEncoding, combine_to_folder, display_path, normalize_extension, read_preview,
    render, save_text, scan,
};

use theme::{Palette, Theme};

const APP_NAME: &str = "TXT Merge";

#[derive(Clone, Copy, PartialEq, Debug)]
enum Tab {
    Combine,
    Edit,
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum Lang {
    En,
    Es,
}

impl Lang {
    fn system() -> Self {
        match sys_locale::get_locale() {
            Some(l) if l.to_lowercase().starts_with("es") => Lang::Es,
            _ => Lang::En,
        }
    }

    /// Picks the text for this language.
    fn t(self, en: &'static str, es: &'static str) -> &'static str {
        match self {
            Lang::En => en,
            Lang::Es => es,
        }
    }

    fn name(self) -> &'static str {
        match self {
            Lang::En => "English",
            Lang::Es => "Español",
        }
    }

    fn key(self) -> &'static str {
        match self {
            Lang::En => "en",
            Lang::Es => "es",
        }
    }

    fn from_key(key: &str) -> Option<Self> {
        match key.trim() {
            "en" => Some(Lang::En),
            "es" => Some(Lang::Es),
            _ => None,
        }
    }
}

/// Appearance and language, the only choices kept between sessions.
#[derive(Clone, Copy, PartialEq, Debug)]
struct Prefs {
    theme: Theme,
    lang: Lang,
}

impl Prefs {
    /// Reads `key=value` lines; unknown or missing values keep `fallback`.
    fn parse(text: &str, fallback: Prefs) -> Prefs {
        let mut prefs = fallback;
        for line in text.lines() {
            match line.split_once('=') {
                Some(("theme", v)) => prefs.theme = Theme::from_key(v).unwrap_or(prefs.theme),
                Some(("lang", v)) => prefs.lang = Lang::from_key(v).unwrap_or(prefs.lang),
                _ => {}
            }
        }
        prefs
    }

    fn to_text(self) -> String {
        format!("theme={}\nlang={}\n", self.theme.key(), self.lang.key())
    }

    fn path() -> Option<PathBuf> {
        std::env::var_os("APPDATA").map(|dir| PathBuf::from(dir).join(APP_NAME).join("settings.txt"))
    }

    fn load() -> Prefs {
        let fallback = Prefs { theme: Theme::Dark, lang: Lang::system() };
        let text = Self::path().and_then(|p| std::fs::read_to_string(p).ok()).unwrap_or_default();
        Self::parse(&text, fallback)
    }

    /// Best effort: if it cannot be written, the app simply starts with the
    /// defaults next time.
    fn save(self) {
        let Some(path) = Self::path() else { return };
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let _ = std::fs::write(path, self.to_text());
    }
}

#[derive(Clone, PartialEq)]
struct Settings {
    lang: Lang,
    folder: Option<PathBuf>,
    /// Extension the combined file is exported with.
    export_extension: String,
    /// When true, `custom_extension` is used instead of `export_extension`.
    use_custom: bool,
    custom_extension: String,
    subfolders: bool,
    no_separator: bool,
    no_filename: bool,
    no_returns: bool,
    add_space: bool,
    separator: String,
    /// Output file name without extension.
    output_stem: String,
    tab: Tab,
    edit: EditOptions,
    /// Save edited files as copies in a new subfolder instead of overwriting.
    edit_copies: bool,
    edit_folder: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            tab: Tab::Combine,
            edit: EditOptions::default(),
            edit_copies: true,
            edit_folder: "editados".to_owned(),
            lang: Lang::system(),
            folder: None,
            export_extension: "txt".to_owned(),
            use_custom: false,
            custom_extension: String::new(),
            subfolders: false,
            no_separator: false,
            no_filename: false,
            no_returns: false,
            add_space: false,
            separator: PRESET_SEPARATORS[0].to_owned(),
            output_stem: "combined".to_owned(),
        }
    }
}

impl Settings {
    /// The export extension, or `None` while the custom field is empty or invalid.
    fn extension(&self) -> Option<String> {
        if self.use_custom {
            normalize_extension(&self.custom_extension)
        } else {
            Some(self.export_extension.clone())
        }
    }

    fn output_name(&self) -> Option<String> {
        let stem = match self.output_stem.trim() {
            "" => "combined",
            s => s,
        };
        self.extension().map(|ext| format!("{stem}.{ext}"))
    }

    fn options(&self) -> CombineOptions {
        CombineOptions {
            no_separator: self.no_separator,
            no_filename: self.no_filename,
            no_returns: self.no_returns,
            add_space: self.add_space,
            separator: self.separator.clone(),
        }
    }

    /// Name of the subfolder edited copies go into.
    fn edit_folder_name(&self) -> String {
        match self.edit_folder.trim() {
            "" => "editados".to_owned(),
            name => name.to_owned(),
        }
    }

    /// Everything that changes which files a scan finds.
    fn scan_key(&self) -> String {
        format!(
            "{:?}|{}|{:?}|{}",
            self.folder,
            self.subfolders,
            self.output_name(),
            self.edit_folder_name()
        )
    }
}

/// True for files inside `editados`, `editados (2)`... directly under `root`,
/// so edited copies are not listed again.
fn is_in_edit_folder(root: &Path, path: &Path, folder: &str) -> bool {
    let rel = path.strip_prefix(root).unwrap_or(path);
    let mut parts = rel.components();
    match (parts.next(), parts.next()) {
        (Some(first), Some(_)) => is_previous_output(Path::new(first.as_os_str()), folder),
        _ => false,
    }
}

/// Human message for a problem in the Edit tab settings.
fn edit_error_text(l: Lang, e: &EditError) -> String {
    match e {
        EditError::LineNumbers(part) if part.is_empty() => {
            l.t("Type the line numbers to delete, like 1-3, 10.", "Escribe los números de línea a borrar, como 1-3, 10.").to_owned()
        }
        EditError::LineNumbers(part) => match l {
            Lang::En => format!("Line numbers not understood: \"{part}\". Use 1-3, 10, 20-"),
            Lang::Es => format!("No se entienden los números de línea: \"{part}\". Usa 1-3, 10, 20-"),
        },
        EditError::Pattern(msg) => {
            format!("{} {msg}", l.t("Invalid regular expression:", "Expresión regular no válida:"))
        }
        EditError::EmptyDeleteText => {
            l.t("Type the text of the lines to delete.", "Escribe el texto de las líneas a borrar.").to_owned()
        }
        EditError::EmptyFind => l.t("Type the text to find.", "Escribe el texto a buscar.").to_owned(),
    }
}

/// True for `combined.*` and `combined (2).*`, `combined (3).*`... whatever
/// the extension, so earlier results (even ones exported as another type)
/// are never merged into a new one.
fn is_previous_output(path: &Path, output_name: &str) -> bool {
    let Some(stem) = path.file_stem().and_then(|n| n.to_str()) else { return false };
    let out = Path::new(output_name).file_stem().and_then(|s| s.to_str()).unwrap_or_default();
    let (stem, out) = (stem.to_lowercase(), out.to_lowercase());
    if stem == out {
        return true;
    }
    let Some(rest) = stem.strip_prefix(&format!("{out} (")) else { return false };
    let Some(num) = rest.strip_suffix(')') else { return false };
    !num.is_empty() && num.chars().all(|c| c.is_ascii_digit())
}

enum Status {
    Idle,
    Done(CombineReport),
    Edited(EditReport),
    /// The I/O error message from writing the output file.
    Error(String),
}

struct FileEntry {
    path: PathBuf,
    name: String,
    selected: bool,
}

/// Largest part of a file shown in the viewer, so huge files stay fast.
const PREVIEW_MAX_BYTES: usize = 512 * 1024;

/// Window width that leaves room for the list and the viewer side by side.
const WIDTH_WITH_VIEWER: f32 = 1320.0;

/// What to do once the user decides about unsaved changes in the viewer.
enum Pending {
    Close,
    Open(PathBuf, String),
    /// Empty the program's list and forget the folder (files on disk stay).
    Clear,
}

struct Preview {
    path: PathBuf,
    name: String,
    /// Text being edited, with `\n` line endings. `None` when unreadable.
    text: Option<String>,
    encoding: TextEncoding,
    /// The file used `\r\n`; restored when saving.
    crlf: bool,
    /// Only part of the file was loaded, so editing is disabled.
    truncated: bool,
    dirty: bool,
    /// Result of the last save: (is_error, message).
    message: Option<(bool, String)>,
    pending: Option<Pending>,
}

impl Preview {
    fn load(path: PathBuf, name: String) -> Self {
        let read = read_preview(&path, PREVIEW_MAX_BYTES);
        let crlf = read.as_ref().is_some_and(|r| r.text.contains("\r\n"));
        Self {
            encoding: read.as_ref().map_or(TextEncoding::Utf8, |r| r.encoding),
            truncated: read.as_ref().is_some_and(|r| r.truncated),
            text: read.map(|r| r.text.replace("\r\n", "\n")),
            crlf,
            path,
            name,
            dirty: false,
            message: None,
            pending: None,
        }
    }

    /// Writes the edited text back to the file. Returns true on success.
    fn save(&mut self, l: Lang) -> bool {
        let Some(text) = &self.text else { return false };
        if self.truncated {
            return false;
        }
        let out = if self.crlf { text.replace('\n', "\r\n") } else { text.clone() };
        match save_text(&self.path, &out, self.encoding) {
            Ok(used) => {
                let note = if used != self.encoding {
                    l.t(
                        "Saved as UTF-8: the text has characters ANSI cannot store.",
                        "Guardado como UTF-8: el texto tiene caracteres que ANSI no admite.",
                    )
                } else {
                    l.t("Saved.", "Guardado.")
                };
                self.encoding = used;
                self.dirty = false;
                self.message = Some((false, note.to_owned()));
                true
            }
            Err(e) => {
                let prefix = l.t("Could not save:", "No se pudo guardar:");
                self.message = Some((true, format!("{prefix} {e}")));
                false
            }
        }
    }
}

enum ViewerAction {
    Save,
    Close,
    SaveAndContinue,
    Discard,
    Cancel,
}

struct App {
    settings: Settings,
    files: Vec<FileEntry>,
    preview: Option<Preview>,
    list_filter: String,
    scanned_key: Option<String>,
    scan_rx: Option<Receiver<(String, Vec<PathBuf>)>>,
    combine_rx: Option<Receiver<std::io::Result<CombineReport>>>,
    edit_rx: Option<Receiver<std::io::Result<EditReport>>>,
    status: Status,
    /// What the viewer shows in the Edit tab.
    view: ViewMode,
    /// The Changes view shows every line; when off, unchanged stretches are folded.
    show_full_file: bool,
    edit_preview: Option<EditPreview>,
    theme: Theme,
    /// Theme whose look is currently installed in egui.
    applied_theme: Option<Theme>,
    settings_open: bool,
    logo_light: egui::TextureHandle,
    logo_dark: egui::TextureHandle,
}

/// Loads a raw RGBA square image bundled in the exe.
fn load_logo(ctx: &egui::Context, name: &str, rgba: &[u8], side: usize) -> egui::TextureHandle {
    let image = egui::ColorImage::from_rgba_unmultiplied([side, side], rgba);
    ctx.load_texture(name, image, egui::TextureOptions::LINEAR)
}

#[derive(Clone, Copy, PartialEq)]
enum ViewMode {
    Changes,
    Result,
    Original,
}

/// Bulk edit applied to the file in the viewer, kept until the text or the
/// edit settings change.
struct EditPreview {
    key: u64,
    result: Result<String, EditError>,
    diff: Vec<DiffLine>,
}

impl App {
    fn new(ctx: &egui::Context) -> Self {
        // Every session starts fresh: no folder or options are remembered,
        // only the appearance and language.
        let prefs = Prefs::load();
        Self {
            settings: Settings { lang: prefs.lang, ..Settings::default() },
            theme: prefs.theme,
            applied_theme: None,
            settings_open: false,
            logo_light: load_logo(ctx, "logo-light", include_bytes!("../assets/logo_light_96.rgba"), 96),
            logo_dark: load_logo(ctx, "logo-dark", include_bytes!("../assets/logo_dark_96.rgba"), 96),
            files: Vec::new(),
            preview: None,
            list_filter: String::new(),
            scanned_key: None,
            scan_rx: None,
            combine_rx: None,
            edit_rx: None,
            status: Status::Idle,
            view: ViewMode::Changes,
            show_full_file: true,
            edit_preview: None,
        }
    }

    fn pick_folder(&mut self) {
        let title = self.settings.lang.t(
            "Select the folder with your text files",
            "Selecciona la carpeta con tus archivos de texto",
        );
        let mut dialog = rfd::FileDialog::new().set_title(title);
        if let Some(dir) = self.settings.folder.as_ref().filter(|d| d.is_dir()) {
            dialog = dialog.set_directory(dir);
        }
        if let Some(dir) = dialog.pick_folder() {
            self.settings.folder = Some(dir);
            self.status = Status::Idle;
        }
    }

    fn start_scan_if_needed(&mut self) {
        let key = self.settings.scan_key();
        if self.scanned_key.as_ref() == Some(&key) {
            return;
        }
        self.scanned_key = Some(key.clone());
        let Some(root) = self.settings.folder.clone().filter(|d| d.is_dir()) else {
            self.files.clear();
            return;
        };
        let output_name = self.settings.output_name();
        let edit_folder = self.settings.edit_folder_name();
        let subfolders = self.settings.subfolders;
        let (tx, rx) = mpsc::channel();
        thread::spawn(move || {
            let exclude = |p: &Path| {
                output_name.as_deref().is_some_and(|n| is_previous_output(p, n))
                    || is_in_edit_folder(&root, p, &edit_folder)
            };
            let found = scan(&root, subfolders, &exclude);
            let _ = tx.send((key, found));
        });
        self.scan_rx = Some(rx);
    }

    fn poll_workers(&mut self) {
        if let Some(rx) = &self.scan_rx {
            if let Ok((key, found)) = rx.try_recv() {
                self.scan_rx = None;
                // Ignore a result that belongs to settings the user already changed.
                if self.scanned_key.as_ref() == Some(&key) {
                    let root = self.settings.folder.clone().unwrap_or_default();
                    self.files = found
                        .into_iter()
                        .map(|p| FileEntry { name: display_path(&root, &p), path: p, selected: true })
                        .collect();
                    let still_listed = |p: &Preview| self.files.iter().any(|f| f.path == p.path);
                    if !self.preview.as_ref().is_some_and(still_listed) {
                        self.preview = None;
                    }
                }
            }
        }
        if let Some(rx) = &self.combine_rx {
            if let Ok(result) = rx.try_recv() {
                self.combine_rx = None;
                self.status = match result {
                    Ok(report) => Status::Done(report),
                    Err(e) => Status::Error(e.to_string()),
                };
            }
        }
        if let Some(rx) = &self.edit_rx {
            if let Ok(result) = rx.try_recv() {
                self.edit_rx = None;
                self.status = match result {
                    Ok(report) => Status::Edited(report),
                    Err(e) => Status::Error(e.to_string()),
                };
                // Originals may have been rewritten: show their new content.
                if let Some(p) = self.preview.as_ref().filter(|p| !p.dirty) {
                    self.preview = Some(Preview::load(p.path.clone(), p.name.clone()));
                }
            }
        }
    }

    fn apply_edits(&mut self) {
        let Some(root) = self.settings.folder.clone() else { return };
        let Ok(edits) = Edits::compile(&self.settings.edit) else { return };
        let files: Vec<PathBuf> =
            self.files.iter().filter(|f| f.selected).map(|f| f.path.clone()).collect();
        let copy_folder = self.settings.edit_copies.then(|| self.settings.edit_folder_name());
        let (tx, rx) = mpsc::channel();
        thread::spawn(move || {
            let _ = tx.send(edit_files(&root, &files, &edits, copy_folder.as_deref()));
        });
        self.edit_rx = Some(rx);
        self.status = Status::Idle;
    }

    /// Recomputes the edited version of the viewer's text and its diff, only
    /// when the text or the edit settings changed.
    fn refresh_edit_preview(&mut self) {
        use std::hash::{Hash, Hasher};
        let Some(text) = self.preview.as_ref().and_then(|p| p.text.as_ref()) else { return };
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        (text, &self.settings.edit).hash(&mut hasher);
        let key = hasher.finish();
        if self.edit_preview.as_ref().map(|c| c.key) == Some(key) {
            return;
        }
        let result = Edits::compile(&self.settings.edit).map(|e| e.apply(text));
        let diff = result.as_ref().map(|edited| diff_lines(text, edited)).unwrap_or_default();
        self.edit_preview = Some(EditPreview { key, result, diff });
    }

    fn combine(&mut self) {
        let (Some(root), Some(name)) = (self.settings.folder.clone(), self.settings.output_name())
        else {
            return;
        };
        let files: Vec<PathBuf> =
            self.files.iter().filter(|f| f.selected).map(|f| f.path.clone()).collect();
        let opts = self.settings.options();
        let (tx, rx) = mpsc::channel();
        thread::spawn(move || {
            let _ = tx.send(combine_to_folder(&root, &files, &name, &opts));
        });
        self.combine_rx = Some(rx);
        self.status = Status::Idle;
    }

    fn busy(&self) -> bool {
        self.scan_rx.is_some() || self.combine_rx.is_some() || self.edit_rx.is_some()
    }

    fn edit_panel(&mut self, ui: &mut egui::Ui) {
        let l = self.settings.lang;
        let s = &mut self.settings;
        let e = &mut s.edit;
        let field = |ui: &mut egui::Ui, value: &mut String, hint: &str, enabled: bool| {
            ui.add_enabled(
                enabled,
                egui::TextEdit::singleline(value).hint_text(hint).desired_width(f32::INFINITY),
            );
        };

        ui.checkbox(&mut self.show_full_file, l.t("Show the whole file text", "Mostrar todo el texto del archivo"))
            .on_hover_text(l.t(
                "In the viewer's Changes view. Uncheck it to hide unchanged parts and see only the changes.",
                "En la vista Cambios del visor. Desmárcalo para ocultar lo que no cambia y ver solo los cambios.",
            ));
        ui.separator();

        ui.heading(l.t("Bulk edit", "Edición en masa"));
        ui.small(l.t(
            "Applied to every checked file in the list, in this order.",
            "Se aplica a todos los archivos marcados en la lista, en este orden.",
        ));
        ui.add_space(4.0);

        ui.checkbox(&mut e.delete_line_numbers, l.t("Delete lines by number", "Borrar líneas por número"));
        field(ui, &mut e.line_numbers, l.t("e.g. 1-3, 10, 20-", "ej. 1-3, 10, 20-"), e.delete_line_numbers);
        ui.add_space(4.0);

        ui.checkbox(&mut e.delete_containing, l.t("Delete lines containing", "Borrar líneas que contengan"));
        field(ui, &mut e.delete_text, l.t("text", "texto"), e.delete_containing);
        ui.add_enabled_ui(e.delete_line_numbers || e.delete_containing, |ui| {
            ui.checkbox(
                &mut e.delete_blank_after,
                l.t("Also delete the empty line left after it", "Borrar también la línea vacía que queda después"),
            )
            .on_hover_text(l.t(
                "So deleting a title does not leave a blank line at the top.",
                "Así, al borrar un título no queda una línea en blanco arriba.",
            ));
        });
        ui.add_space(4.0);

        ui.checkbox(&mut e.replace, l.t("Replace text", "Reemplazar texto"));
        field(ui, &mut e.find, l.t("Find", "Buscar"), e.replace);
        field(ui, &mut e.replace_with, l.t("Replace with (empty deletes it)", "Reemplazar por (vacío lo borra)"), e.replace);
        ui.add_enabled_ui(e.replace || e.delete_containing, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.checkbox(&mut e.match_case, l.t("Match case", "Coincidir mayúsculas"));
                ui.checkbox(&mut e.use_regex, l.t("Regular expression", "Expresión regular"))
                    .on_hover_text(l.t(
                        "Patterns like \\d+ or ^Title. In the replacement, $1 inserts group 1.",
                        "Patrones como \\d+ o ^Título. En el reemplazo, $1 inserta el grupo 1.",
                    ));
            });
        });
        ui.add_space(4.0);

        ui.checkbox(&mut e.trim_lines, l.t("Trim spaces at start and end of lines", "Quitar espacios al inicio y final de cada línea"));
        ui.checkbox(&mut e.remove_empty, l.t("Remove empty lines", "Quitar líneas vacías"));
        ui.add_enabled_ui(!e.remove_empty, |ui| {
            ui.checkbox(&mut e.collapse_empty, l.t("Keep at most one empty line in a row", "Dejar como máximo una línea vacía seguida"));
        });
        ui.add_enabled_ui(e.remove_empty || e.collapse_empty, |ui| {
            ui.checkbox(&mut e.whitespace_is_empty, l.t("Lines with only spaces count as empty", "Contar líneas con solo espacios como vacías"));
        });

        if let Err(err) = Edits::compile(e) {
            ui.colored_label(ui.visuals().error_fg_color, edit_error_text(l, &err));
        }

        ui.separator();
        ui.heading(l.t("Save to", "Guardar en"));
        ui.radio_value(&mut s.edit_copies, true, l.t("Copies in a new subfolder", "Copias en una subcarpeta nueva"));
        ui.add_enabled_ui(s.edit_copies, |ui| {
            ui.add(egui::TextEdit::singleline(&mut s.edit_folder).hint_text("editados").desired_width(f32::INFINITY));
            ui.small(l.t(
                "Originals are not touched. If the folder exists, (2), (3)... is added.",
                "Los originales no se tocan. Si la carpeta existe, se añade (2), (3)...",
            ));
        });
        ui.radio_value(&mut s.edit_copies, false, l.t("Overwrite the original files", "Sobrescribir los archivos originales"));
        if !s.edit_copies {
            ui.colored_label(
                ui.visuals().warn_fg_color,
                l.t("Changes go straight into your files and cannot be undone.", "Los cambios van directo a tus archivos y no se pueden deshacer."),
            );
        }
        ui.checkbox(&mut s.subfolders, l.t("Include subfolders", "Incluir subcarpetas"));

        ui.separator();
        ui.small(l.t(
            "Tip: press \"View\" on a file to see the result before applying.",
            "Consejo: pulsa \"Ver\" en un archivo para ver el resultado antes de aplicar.",
        ));
    }

    fn options_panel(&mut self, ui: &mut egui::Ui) {
        let s = &mut self.settings;
        let l = s.lang;

        ui.heading(l.t("Export as", "Exportar como"));
        ui.small(l.t(
            "Every text file in the folder is read. This only sets the combined file's type.",
            "Se leen todos los archivos de texto de la carpeta. Esto solo define el tipo del archivo combinado.",
        ));
        egui::Grid::new("types").num_columns(4).spacing([10.0, 2.0]).show(ui, |ui| {
            for (i, ext) in EXPORT_EXTENSIONS.iter().enumerate() {
                let on = !s.use_custom && s.export_extension == *ext;
                if ui.radio(on, format!(".{ext}")).clicked() {
                    s.export_extension = ext.to_string();
                    s.use_custom = false;
                }
                if i % 4 == 3 {
                    ui.end_row();
                }
            }
        });
        ui.horizontal(|ui| {
            if ui.radio(s.use_custom, l.t("Other:", "Otro:")).clicked() {
                s.use_custom = true;
            }
            let field = ui.add(
                egui::TextEdit::singleline(&mut s.custom_extension)
                    .hint_text(l.t("e.g. adoc", "ej. adoc"))
                    .desired_width(f32::INFINITY),
            );
            if field.gained_focus() || field.changed() {
                s.use_custom = true;
            }
        });
        if s.use_custom && s.extension().is_none() {
            ui.colored_label(
                ui.visuals().error_fg_color,
                l.t("Type one extension, like adoc.", "Escribe una extensión, como adoc."),
            );
        }

        ui.label(l.t("Output file name:", "Nombre del archivo final:"));
        ui.horizontal(|ui| {
            let ext = s.extension().map(|e| format!(".{e}")).unwrap_or_else(|| ".?".to_owned());
            let width = ui.available_width() - 60.0;
            ui.add(
                egui::TextEdit::singleline(&mut s.output_stem)
                    .hint_text("combined")
                    .desired_width(width),
            );
            ui.strong(ext);
        });
        ui.small(l.t(
            "Saved inside the selected folder. If the name is taken, (2), (3)... is added.",
            "Se guarda en la carpeta seleccionada. Si el nombre existe, se añade (2), (3)...",
        ));
        ui.checkbox(&mut s.subfolders, l.t("Include subfolders", "Incluir subcarpetas"))
            .on_hover_text(l.t(
                "Also read every folder inside the selected one.",
                "Lee también todas las carpetas dentro de la seleccionada.",
            ));

        ui.separator();
        ui.heading(l.t("Output format", "Formato del resultado"));
        ui.checkbox(&mut s.no_filename, l.t("No Filename", "Sin nombre de archivo"))
            .on_hover_text(l.t(
                "Do not write the file name above each file's text.",
                "No escribe el nombre del archivo encima de su texto.",
            ));
        ui.checkbox(&mut s.no_separator, l.t("No Separator", "Sin separador"))
            .on_hover_text(l.t(
                "Do not write the separator line under each file name.",
                "No escribe la línea separadora debajo de cada nombre.",
            ));
        ui.checkbox(&mut s.no_returns, l.t("No Carriage Returns", "Sin saltos de línea"))
            .on_hover_text(l.t(
                "Join the files as one continuous text, without line breaks between them.",
                "Une los archivos como un texto continuo, sin saltos de línea entre ellos.",
            ));
        ui.add_enabled_ui(s.no_returns, |ui| {
            ui.checkbox(&mut s.add_space, l.t("Add Space Char", "Añadir espacio"))
                .on_hover_text(l.t(
                    "Put a space between the joined files.",
                    "Pone un espacio entre los archivos unidos.",
                ))
                .on_disabled_hover_text(l.t(
                    "Only used together with \"No Carriage Returns\".",
                    "Solo se usa junto con \"Sin saltos de línea\".",
                ));
        });

        ui.add_enabled_ui(!s.no_separator, |ui| {
            ui.label(l.t("Separator:", "Separador:"));
            egui::ComboBox::from_id_salt("sep")
                .width(ui.available_width())
                .selected_text(truncate(&s.separator, 32))
                .show_ui(ui, |ui| {
                    for sep in PRESET_SEPARATORS {
                        ui.selectable_value(&mut s.separator, sep.to_string(), truncate(sep, 40));
                    }
                });
            ui.add(
                egui::TextEdit::singleline(&mut s.separator)
                    .hint_text(l.t("or type your own", "o escribe el tuyo"))
                    .desired_width(f32::INFINITY),
            );
            ui.small(l.t(
                "Click a variable to add it to the separator. The value after \":\" can be edited.",
                "Haz clic en una variable para añadirla al separador. El valor después de \":\" se puede editar.",
            ));
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = [4.0, 4.0].into();
                for var in SEPARATOR_VARIABLES {
                    let button = ui
                        .small_button(egui::RichText::new(*var).monospace())
                        .on_hover_text(variable_help(l, var));
                    if button.clicked() {
                        s.separator.push_str(var);
                    }
                }
            });
        });

        ui.separator();
        ui.label(egui::RichText::new(l.t("Preview", "Vista previa")).strong());
        let sample = [
            Segment {
                name: "notes.txt".into(),
                path: "docs/notes.txt".into(),
                sibling_dirs: 0,
                content: Some(l.t("First file.", "Primer archivo.").into()),
            },
            Segment {
                name: "readme.md".into(),
                path: "docs/readme.md".into(),
                sibling_dirs: 0,
                content: Some(l.t("Second file.", "Segundo archivo.").into()),
            },
        ];
        let preview = render(&sample, &s.options()).replace("\r\n", "\n");
        egui::Frame::group(ui.style()).show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(egui::RichText::new(preview.trim_end()).monospace().size(11.0));
        });
    }

    fn file_list(&mut self, ui: &mut egui::Ui) {
        let l = self.settings.lang;
        if self.settings.folder.is_none() {
            let dragging = ui.ctx().input(|i| !i.raw.hovered_files.is_empty());
            ui.add_space(((ui.available_height() - 190.0) / 2.0 - 20.0).max(12.0));
            let zone = ui.vertical_centered(|ui| {
                theme::drop_zone(
                    ui,
                    "📂",
                    l.t("Drop a folder with your .txt files here", "Suelta aquí una carpeta con tus .txt"),
                    l.t("or click to choose one", "o haz clic para elegirla"),
                    dragging,
                )
            });
            if zone.inner.clicked() {
                self.pick_folder();
            }
            return;
        }
        let selected = self.files.iter().filter(|f| f.selected).count();
        // Wraps onto two lines when the column is narrow instead of spilling over.
        ui.horizontal_wrapped(|ui| {
            if self.scan_rx.is_some() {
                ui.spinner();
                ui.label(l.t("Searching files...", "Buscando archivos..."));
            } else {
                let text = match l {
                    Lang::En => format!("{selected} of {} files selected", self.files.len()),
                    Lang::Es => format!("{selected} de {} archivos seleccionados", self.files.len()),
                };
                ui.strong(text);
            }
            ui.add_space(12.0);
            ui.add(
                egui::TextEdit::singleline(&mut self.list_filter)
                    .hint_text(l.t("Filter list...", "Filtrar lista..."))
                    .desired_width(160.0),
            );
            if ui.button(l.t("Select all", "Todos")).clicked() {
                self.files.iter_mut().for_each(|f| f.selected = true);
            }
            if ui.button(l.t("Select none", "Ninguno")).clicked() {
                self.files.iter_mut().for_each(|f| f.selected = false);
            }
        });
        ui.separator();

        if self.files.is_empty() && self.scan_rx.is_none() {
            ui.centered_and_justified(|ui| {
                ui.label(l.t(
                    "No text files here. Try including subfolders.",
                    "No hay archivos de texto aquí. Prueba incluyendo subcarpetas.",
                ));
            });
            return;
        }
        let needle = self.list_filter.to_lowercase();
        let viewing = self.preview.as_ref().map(|p| p.path.clone());
        let mut to_view = None;
        egui::ScrollArea::vertical().id_salt("list").auto_shrink(false).show(ui, |ui| {
            for f in self.files.iter_mut() {
                if needle.is_empty() || f.name.to_lowercase().contains(&needle) {
                    ui.horizontal(|ui| {
                        let active = viewing.as_ref() == Some(&f.path);
                        let view = ui
                            .selectable_label(active, l.t("View", "Ver"))
                            .on_hover_text(l.t("Show what this file contains", "Ver qué contiene este archivo"));
                        if view.clicked() {
                            to_view = Some((f.path.clone(), f.name.clone()));
                        }
                        ui.checkbox(&mut f.selected, "");
                        // Cut to the column width; the full name shows on hover.
                        let name = ui.add(egui::Label::new(&f.name).truncate().sense(egui::Sense::click()));
                        if name.clicked() {
                            f.selected = !f.selected;
                        }
                        name.on_hover_text(&f.name);
                    });
                }
            }
        });
        if let Some((path, name)) = to_view {
            self.request(ui.ctx(), Pending::Open(path, name));
        }
    }

    /// Opens a file or closes the viewer, first asking about unsaved changes.
    fn request(&mut self, ctx: &egui::Context, what: Pending) {
        if let Some(p) = &mut self.preview {
            if p.dirty {
                p.pending = Some(what);
                return;
            }
        }
        self.apply(ctx, what);
    }

    fn apply(&mut self, ctx: &egui::Context, what: Pending) {
        match what {
            Pending::Close => self.preview = None,
            Pending::Open(path, name) => {
                if self.preview.is_none() {
                    widen_for_viewer(ctx);
                }
                self.preview = Some(Preview::load(path, name));
            }
            Pending::Clear => {
                self.preview = None;
                self.settings.folder = None;
                self.files.clear();
                self.list_filter.clear();
                self.scanned_key = None;
                self.scan_rx = None;
                self.status = Status::Idle;
            }
        }
    }

    fn preview_panel(&mut self, ui: &mut egui::Ui) {
        let l = self.settings.lang;
        let in_edit_tab = self.settings.tab == Tab::Edit;
        let previewing = in_edit_tab && self.view != ViewMode::Original;
        if previewing {
            self.refresh_edit_preview();
        }
        let any_edit = self.settings.edit.any();
        let edited = if previewing { self.edit_preview.as_ref() } else { None };
        let view = &mut self.view;
        let show_full_file = self.show_full_file;
        let Some(p) = &mut self.preview else { return };
        let editable = p.text.is_some() && !p.truncated;
        let mut action = None;

        // Long names have no spaces to wrap at, so they are cut to fit the column.
        ui.add(egui::Label::new(egui::RichText::new(&p.name).strong()).truncate())
            .on_hover_text(&p.name);
        ui.horizontal(|ui| {
            let save = egui::Button::new(l.t("Save", "Guardar"));
            if ui
                .add_enabled(editable && p.dirty, save)
                .on_hover_text(l.t("Save changes to this file (Ctrl+S)", "Guardar cambios en este archivo (Ctrl+S)"))
                .clicked()
            {
                action = Some(ViewerAction::Save);
            }
            if ui.button(l.t("Close", "Cerrar")).clicked() {
                action = Some(ViewerAction::Close);
            }
            if p.dirty {
                ui.label(l.t("(modified)", "(modificado)"));
            }
        });
        if p.pending.is_some() {
            egui::Frame::group(ui.style()).show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.colored_label(
                    ui.visuals().warn_fg_color,
                    l.t("This file has unsaved changes.", "Este archivo tiene cambios sin guardar."),
                );
                ui.horizontal(|ui| {
                    if ui.button(l.t("Save", "Guardar")).clicked() {
                        action = Some(ViewerAction::SaveAndContinue);
                    }
                    if ui.button(l.t("Discard", "Descartar")).clicked() {
                        action = Some(ViewerAction::Discard);
                    }
                    if ui.button(l.t("Cancel", "Cancelar")).clicked() {
                        action = Some(ViewerAction::Cancel);
                    }
                });
            });
        }
        if p.truncated {
            ui.small(l.t(
                "Large file: only the first 512 KB are shown, so it cannot be edited here.",
                "Archivo grande: solo se muestran los primeros 512 KB, por eso no se puede editar aquí.",
            ));
        }
        if let Some((is_error, msg)) = &p.message {
            let color = if *is_error { ui.visuals().error_fg_color } else { ui.visuals().weak_text_color() };
            ui.colored_label(color, msg);
        }
        if in_edit_tab && p.text.is_some() {
            ui.horizontal(|ui| {
                ui.selectable_value(view, ViewMode::Changes, l.t("Changes", "Cambios")).on_hover_text(l.t(
                    "Removed lines in red (-), added lines in green (+)",
                    "Líneas borradas en rojo (-), líneas añadidas en verde (+)",
                ));
                ui.selectable_value(view, ViewMode::Result, l.t("Result", "Resultado")).on_hover_text(l.t(
                    "How this file will look after the bulk edit (read only)",
                    "Cómo quedará este archivo tras la edición en masa (solo lectura)",
                ));
                ui.selectable_value(view, ViewMode::Original, l.t("Original (editable)", "Original (editable)"));
            });
        }
        ui.separator();

        let truncated = p.truncated;
        match (edited, &mut p.text) {
            (Some(EditPreview { result: Err(e), .. }), _) => {
                ui.colored_label(ui.visuals().error_fg_color, edit_error_text(l, e));
            }
            (Some(EditPreview { result: Ok(_), diff, .. }), Some(_)) if *view == ViewMode::Changes => {
                let added = diff.iter().filter(|d| d.change == Change::Added).count();
                let removed = diff.iter().filter(|d| d.change == Change::Removed).count();
                ui.horizontal(|ui| {
                    if !any_edit {
                        ui.small(l.t("No edit is switched on yet.", "Todavía no hay ninguna edición activada."));
                    } else if added + removed == 0 {
                        ui.small(l.t("No changes in this file.", "Sin cambios en este archivo."));
                    } else {
                        ui.colored_label(DIFF_GREEN, egui::RichText::new(format!("+{added}")).strong());
                        ui.colored_label(DIFF_RED, egui::RichText::new(format!("-{removed}")).strong());
                        ui.small(l.t("lines", "líneas"));
                    }
                });
                let rows: Vec<DiffRow> =
                    if show_full_file { diff.iter().map(DiffRow::Line).collect() } else { only_changes(diff, 3) };
                diff_view(ui, &rows, l);
            }
            (Some(EditPreview { result: Ok(edited), .. }), Some(original)) => {
                let before = original.split('\n').count();
                let after = if edited.is_empty() { 0 } else { edited.split('\n').count() };
                let summary = match l {
                    Lang::En => format!("Lines: {before} before, {after} after"),
                    Lang::Es => format!("Líneas: {before} antes, {after} después"),
                };
                ui.small(summary);
                code_view(ui, &mut edited.as_str(), "viewer-result");
            }
            (_, Some(text)) => {
                let output = if truncated {
                    code_view(ui, &mut text.as_str(), "viewer")
                } else {
                    code_view(ui, text, "viewer")
                };
                if output.response.changed() {
                    p.dirty = true;
                    p.message = None;
                }
            }
            (_, None) => {
                ui.colored_label(
                    ui.visuals().error_fg_color,
                    l.t(
                        "This file could not be read (binary, locked or no access).",
                        "No se pudo leer este archivo (binario, en uso o sin permiso).",
                    ),
                );
            }
        }

        let ctx = ui.ctx().clone();
        match action {
            None => {}
            Some(ViewerAction::Save) => {
                p.save(l);
            }
            Some(ViewerAction::Close) => self.request(&ctx, Pending::Close),
            Some(ViewerAction::SaveAndContinue) => {
                if p.save(l) {
                    if let Some(next) = p.pending.take() {
                        self.apply(&ctx, next);
                    }
                }
            }
            Some(ViewerAction::Discard) => {
                if let Some(next) = p.pending.take() {
                    self.apply(&ctx, next);
                }
            }
            Some(ViewerAction::Cancel) => p.pending = None,
        }
    }

    fn status_bar(&mut self, ui: &mut egui::Ui) {
        let l = self.settings.lang;
        let selected = self.files.iter().filter(|f| f.selected).count();
        let big = |ui: &mut egui::Ui, enabled: bool, text: String| {
            theme::primary_button(ui, text, enabled, egui::vec2(200.0, 36.0))
        };
        match self.settings.tab {
            Tab::Combine => {
                let can_combine = selected > 0 && !self.busy() && self.settings.output_name().is_some();
                let label = match l {
                    Lang::En => format!("Combine {selected} files"),
                    Lang::Es => format!("Combinar {selected} archivos"),
                };
                if big(ui, can_combine, label).clicked() {
                    self.combine();
                }
            }
            Tab::Edit => {
                let valid = self.settings.edit.any() && Edits::compile(&self.settings.edit).is_ok();
                let can_apply = selected > 0 && !self.busy() && valid;
                let label = match (l, self.settings.edit_copies) {
                    (Lang::En, true) => format!("Apply to {selected} files"),
                    (Lang::Es, true) => format!("Aplicar a {selected} archivos"),
                    (Lang::En, false) => format!("Apply and overwrite {selected} files"),
                    (Lang::Es, false) => format!("Aplicar y sobrescribir {selected} archivos"),
                };
                let button = big(ui, can_apply, label);
                if !self.settings.edit.any() {
                    button.on_hover_text(l.t("Switch on at least one edit.", "Activa al menos una edición."));
                } else if button.clicked() {
                    self.apply_edits();
                }
            }
        }
        if self.combine_rx.is_some() {
            ui.spinner();
            ui.label(l.t("Combining...", "Combinando..."));
        }
        if self.edit_rx.is_some() {
            ui.spinner();
            ui.label(l.t("Editing files...", "Editando archivos..."));
        }
        match &self.status {
            Status::Idle => {}
            Status::Error(e) => {
                let prefix = l.t("Could not write the files:", "No se pudieron guardar los archivos:");
                ui.colored_label(ui.visuals().error_fg_color, format!("{prefix} {e}"));
            }
            Status::Edited(r) => {
                let mut text = match l {
                    Lang::En => format!("{} files changed, {} without changes", r.changed, r.unchanged),
                    Lang::Es => format!("{} archivos modificados, {} sin cambios", r.changed, r.unchanged),
                };
                if !r.failed.is_empty() {
                    text.push_str(&match l {
                        Lang::En => format!(", {} could not be read or saved", r.failed.len()),
                        Lang::Es => format!(", {} no se pudieron leer o guardar", r.failed.len()),
                    });
                }
                if let Some(dir) = &r.copy_folder {
                    text.push_str(&match l {
                        Lang::En => format!(" — saved in {}", dir.display()),
                        Lang::Es => format!(" — guardados en {}", dir.display()),
                    });
                }
                ui.label(text).on_hover_text(r.failed.join("\n"));
                if let Some(dir) = r.copy_folder.clone() {
                    if ui.button(l.t("Open folder", "Abrir carpeta")).clicked() {
                        open_in_explorer(&dir, false);
                    }
                }
            }
            Status::Done(r) => {
                let mut text = match l {
                    Lang::En => format!("Saved {} files to {}", r.merged, r.output.display()),
                    Lang::Es => format!("{} archivos guardados en {}", r.merged, r.output.display()),
                };
                if !r.skipped.is_empty() {
                    text.push_str(&match l {
                        Lang::En => format!(" ({} skipped, marked [Error])", r.skipped.len()),
                        Lang::Es => format!(" ({} omitidos, marcados con [Error])", r.skipped.len()),
                    });
                }
                ui.label(text).on_hover_text(r.skipped.join("\n"));
                let output = r.output.clone();
                if ui.button(l.t("Open file", "Abrir archivo")).clicked() {
                    open_in_explorer(&output, false);
                }
                if ui.button(l.t("Show in folder", "Mostrar en carpeta")).clicked() {
                    open_in_explorer(&output, true);
                }
            }
        }
    }
}

fn variable_help(l: Lang, var: &str) -> &'static str {
    match var {
        "<Inc Nr:1>" => l.t(
            "Ascending number. Change 1 to start from another number; 001 pads with zeros",
            "Número ascendente. Cambia el 1 para empezar en otro número; 001 rellena con ceros",
        ),
        "<Inc NrDir:1>" => l.t(
            "Ascending number per folder. Change 1 to start from another number",
            "Número ascendente por carpeta. Cambia el 1 para empezar en otro número",
        ),
        "<Inc Alpha:A>" => l.t(
            "Ascending letter (A, B... Z, AA). Change A to start from another letter; lowercase a gives a, b, c",
            "Letra en orden ascendente (A, B... Z, AA). Cambia la A para empezar en otra letra; en minúscula da a, b, c",
        ),
        "<Name>" => l.t("File name without extension", "Nombre de archivo sin extensión"),
        "<Ext>" => l.t("File extension without dot", "Extensión de archivo sin punto"),
        "<FolderName:1>" => l.t(
            "Folder name. 1 is the file's folder, 2 the one above it, and so on",
            "Nombre de carpeta. 1 es la carpeta del archivo, 2 la de arriba, y así",
        ),
        "<Num Files>" => l.t(
            "Total listed files in the file's folder",
            "Número total de archivos de la lista en su carpeta",
        ),
        "<Num Dirs>" => l.t("Total folders inside the file's folder", "Número total de carpetas en la carpeta"),
        "<Num Items>" => l.t("Total files in the list", "Número total de archivos en la lista"),
        "<Word:1>" => l.t(
            "Word N of the file name (change 1 to pick another)",
            "Palabra N del nombre de archivo (cambia el 1 para elegir otra)",
        ),
        "<RWord:1>" => l.t(
            "Word N counting from the end of the file name",
            "Palabra N contando desde el final del nombre de archivo",
        ),
        "<MediaType>" => l.t(
            "Media (MIME) type from the extension, e.g. text/markdown",
            "Tipo de medio (MIME) según la extensión, ej. text/markdown",
        ),
        _ => "",
    }
}

const DIFF_RED: egui::Color32 = theme::ERROR;
const DIFF_GREEN: egui::Color32 = theme::SUCCESS;

/// GitHub-style diff: the line number in the original file, a -/+ sign and a
/// tinted row for every removed or added line. Added lines have no original
/// number. Only visible rows are laid out.
fn diff_view(ui: &mut egui::Ui, rows: &[DiffRow], l: Lang) {
    let font = egui::TextStyle::Monospace.resolve(ui.style());
    let digit = ui.fonts(|f| f.glyph_width(&font, '0'));
    let row_height = theme::EDITOR_LINE_HEIGHT;
    let biggest = rows
        .iter()
        .filter_map(|r| match r {
            DiffRow::Line(d) => d.old,
            DiffRow::Hidden(_) => None,
        })
        .max()
        .unwrap_or(1);
    let number_width = digit * biggest.to_string().len().max(2) as f32 + 12.0;
    let text_left = number_width + digit * 2.0 + 8.0;
    let weak = ui.visuals().weak_text_color();
    let normal = ui.visuals().text_color();
    let faint = ui.visuals().faint_bg_color;

    ui.scope(|ui| {
        ui.spacing_mut().item_spacing.y = 0.0;
        egui::ScrollArea::both().id_salt("viewer-diff").auto_shrink(false).show_rows(
            ui,
            row_height,
            rows.len(),
            |ui, range| {
                for row in &rows[range] {
                    let (text, line) = match row {
                        DiffRow::Line(d) => (d.text.clone(), Some(*d)),
                        DiffRow::Hidden(n) => {
                            let text = match l {
                                Lang::En => format!("@@ {n} unchanged lines @@"),
                                Lang::Es => format!("@@ {n} líneas sin cambios @@"),
                            };
                            (text, None)
                        }
                    };
                    let color = if line.is_some() { normal } else { weak };
                    let galley = ui.painter().layout_no_wrap(text, font.clone(), color);
                    let width = (text_left + galley.size().x + 8.0).max(ui.available_width());
                    let (rect, _) = ui.allocate_exact_size(egui::vec2(width, row_height), egui::Sense::hover());
                    let painter = ui.painter();
                    let (tint, sign) = match line.map(|d| d.change) {
                        Some(Change::Removed) => (Some(DIFF_RED.gamma_multiply(0.22)), Some(("-", DIFF_RED))),
                        Some(Change::Added) => (Some(DIFF_GREEN.gamma_multiply(0.22)), Some(("+", DIFF_GREEN))),
                        Some(Change::Same) => (None, None),
                        None => (Some(faint), None),
                    };
                    if let Some(tint) = tint {
                        painter.rect_filled(rect, 0.0, tint);
                    }
                    let y = rect.top() + (row_height - galley.size().y) / 2.0;
                    if let Some(n) = line.and_then(|d| d.old) {
                        painter.text(
                            egui::pos2(rect.left() + number_width - 6.0, y),
                            egui::Align2::RIGHT_TOP,
                            n.to_string(),
                            font.clone(),
                            weak,
                        );
                    }
                    if let Some((sign, color)) = sign {
                        painter.text(
                            egui::pos2(rect.left() + number_width + 2.0, y),
                            egui::Align2::LEFT_TOP,
                            sign,
                            font.clone(),
                            color,
                        );
                    }
                    painter.galley(egui::pos2(rect.left() + text_left, y), galley, color);
                }
            },
        );
    });
}

/// A scrollable code editor with line numbers on the left.
fn code_view(
    ui: &mut egui::Ui,
    buffer: &mut dyn egui::TextBuffer,
    id: &str,
) -> egui::text_edit::TextEditOutput {
    let line_count = buffer.as_str().split('\n').count();
    egui::ScrollArea::vertical()
        .id_salt(id)
        .auto_shrink(false)
        .show(ui, |ui| {
            let font = egui::TextStyle::Monospace.resolve(ui.style());
            let digit = ui.fonts(|f| f.glyph_width(&font, '0'));
            let gutter = digit * line_count.to_string().len().max(2) as f32 + 14.0;
            // Taller lines (1.6) make long joined texts easier to read.
            let mut layouter = |ui: &egui::Ui, text: &str, wrap_width: f32| {
                let mut job =
                    egui::text::LayoutJob::simple(text.to_owned(), font.clone(), ui.visuals().text_color(), wrap_width);
                job.sections.iter_mut().for_each(|s| s.format.line_height = Some(theme::EDITOR_LINE_HEIGHT));
                ui.fonts(|f| f.layout_job(job))
            };
            let output = ui
                .horizontal_top(|ui| {
                    ui.add_space(gutter);
                    egui::TextEdit::multiline(buffer)
                        .id_salt(id)
                        .code_editor()
                        .layouter(&mut layouter)
                        .desired_width(f32::INFINITY)
                        .desired_rows(30)
                        .show(ui)
                })
                .inner;
            paint_line_numbers(ui, &output, gutter, &font);
            output
        })
        .inner
}

/// For each laid-out row, the line number to show beside it (1-based), or
/// `None` for the continuation rows of a wrapped line.
fn row_line_numbers(rows_end_with_newline: impl IntoIterator<Item = bool>) -> Vec<Option<usize>> {
    let mut line = 1;
    let mut starts_line = true;
    rows_end_with_newline
        .into_iter()
        .map(|ends_line| {
            let label = starts_line.then_some(line);
            starts_line = ends_line;
            if ends_line {
                line += 1;
            }
            label
        })
        .collect()
}

/// Draws the line numbers in the space left of the editor, aligned with the
/// rows of the text so wrapped lines keep a single number.
fn paint_line_numbers(
    ui: &egui::Ui,
    output: &egui::text_edit::TextEditOutput,
    gutter: f32,
    font: &egui::FontId,
) {
    let editor = output.response.rect;
    let right = editor.left() - ui.spacing().item_spacing.x;
    let area = egui::Rect::from_min_max(
        egui::pos2(right - gutter, editor.top()),
        egui::pos2(right, editor.bottom()),
    );
    let painter = ui.painter();
    painter.rect_filled(area, 2.0, ui.visuals().faint_bg_color);
    let current = output.cursor_range.map(|c| c.primary.pcursor.paragraph + 1);
    let numbers = row_line_numbers(output.galley.rows.iter().map(|r| r.ends_with_newline));
    for (row, number) in output.galley.rows.iter().zip(numbers) {
        let Some(number) = number else { continue };
        let color = if current == Some(number) {
            ui.visuals().strong_text_color()
        } else {
            ui.visuals().weak_text_color()
        };
        painter.text(
            egui::pos2(right - 6.0, output.galley_pos.y + row.rect.top()),
            egui::Align2::RIGHT_TOP,
            number.to_string(),
            font.clone(),
            color,
        );
    }
}

/// Makes the window wide enough for two columns, unless it is maximized.
fn widen_for_viewer(ctx: &egui::Context) {
    let (inner, maximized) =
        ctx.input(|i| (i.viewport().inner_rect, i.viewport().maximized.unwrap_or(false)));
    if let Some(rect) = inner.filter(|r| !maximized && r.width() < WIDTH_WITH_VIEWER) {
        ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(
            WIDTH_WITH_VIEWER,
            rect.height(),
        )));
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_owned()
    } else {
        format!("{}...", s.chars().take(max).collect::<String>())
    }
}

fn open_in_explorer(path: &Path, select: bool) {
    let mut cmd = std::process::Command::new("explorer");
    if select {
        cmd.arg(format!("/select,{}", path.display()));
    } else {
        cmd.arg(path);
    }
    let _ = cmd.spawn();
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let dropped: Vec<PathBuf> =
            ctx.input(|i| i.raw.dropped_files.iter().filter_map(|f| f.path.clone()).collect());
        if let Some(p) = dropped.into_iter().next() {
            let dir = if p.is_dir() { Some(p) } else { p.parent().map(Path::to_path_buf) };
            if dir.is_some() {
                self.settings.folder = dir;
                self.status = Status::Idle;
            }
        }

        self.poll_workers();
        self.start_scan_if_needed();
        if self.busy() {
            ctx.request_repaint();
        }
        if self.applied_theme != Some(self.theme) {
            theme::apply(ctx, self.theme);
            self.applied_theme = Some(self.theme);
        }
        let l = self.settings.lang;
        let palette = Palette::of(self.theme == Theme::Dark);
        let bar = egui::Frame::none()
            .fill(palette.bg_editor)
            .inner_margin(egui::Margin::symmetric(14.0, 8.0))
            .stroke(egui::Stroke::new(1.0_f32, palette.border));

        egui::TopBottomPanel::top("header").frame(bar).show(ctx, |ui| {
            ui.horizontal(|ui| {
                let logo = if self.theme == Theme::Dark { &self.logo_dark } else { &self.logo_light };
                theme::wordmark(ui, logo);
                ui.add_space(24.0);
                let mut tab = self.settings.tab;
                let tabs = [(Tab::Combine, l.t("Merge", "Combinar")), (Tab::Edit, l.t("Edit", "Editar"))];
                if theme::segmented(ui, "tabs", &mut tab, &tabs, 110.0) {
                    self.settings.tab = tab;
                    self.status = Status::Idle;
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if theme::icon_button(ui, "⚙", l.t("Settings", "Configuración")).clicked() {
                        self.settings_open = true;
                    }
                });
            });
        });

        let folder_bar = egui::Frame::none()
            .fill(palette.bg_app)
            .inner_margin(egui::Margin { left: 14.0, right: 14.0, top: 10.0, bottom: 4.0 });
        let mut clear_list = false;
        egui::TopBottomPanel::top("folder").frame(folder_bar).show_separator_line(false).show(ctx, |ui| {
            ui.horizontal(|ui| {
                let pick = egui::Button::new(
                    egui::RichText::new(l.t("📂  Choose folder...", "📂  Elegir carpeta...")).strong(),
                )
                .min_size([150.0, 30.0].into());
                if ui.add(pick).clicked() {
                    self.pick_folder();
                }
                match &self.settings.folder {
                    Some(d) => {
                        ui.label(egui::RichText::new(d.display().to_string()).strong());
                        let open = ui
                            .small_button(l.t("Open", "Abrir"))
                            .on_hover_text(l.t("Open this folder in Explorer", "Abrir esta carpeta en el Explorador"));
                        if open.clicked() {
                            open_in_explorer(d, false);
                        }
                        if ui.small_button(l.t("Refresh", "Actualizar")).clicked() {
                            self.scanned_key = None;
                        }
                        let clear = ui.small_button(l.t("Clear list", "Limpiar lista")).on_hover_text(l.t(
                            "Remove these files from the list to process others. Nothing is deleted from disk.",
                            "Quita estos archivos de la lista para procesar otros. No se borra nada del disco.",
                        ));
                        clear_list = clear.clicked();
                    }
                    None => {
                        ui.label(
                            egui::RichText::new(l.t("No folder selected", "Ninguna carpeta seleccionada"))
                                .color(palette.text_weak),
                        );
                    }
                }
            });
        });
        if clear_list {
            // Goes through `request` so unsaved changes in the viewer are asked about first.
            self.request(ctx, Pending::Clear);
        }

        egui::TopBottomPanel::bottom("actions").frame(bar).show(ctx, |ui| {
            ui.horizontal(|ui| self.status_bar(ui));
        });

        let sidebar = egui::Frame::none()
            .fill(palette.bg_sidebar)
            .inner_margin(egui::Margin::symmetric(14.0, 10.0))
            .stroke(egui::Stroke::new(1.0_f32, palette.border));
        egui::SidePanel::left("options").resizable(true).default_width(320.0).frame(sidebar).show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                match self.settings.tab {
                    Tab::Combine => self.options_panel(ui),
                    Tab::Edit => self.edit_panel(ui),
                }
            });
        });

        let save_shortcut = egui::KeyboardShortcut::new(egui::Modifiers::COMMAND, egui::Key::S);
        if ctx.input_mut(|i| i.consume_shortcut(&save_shortcut)) {
            if let Some(p) = self.preview.as_mut().filter(|p| p.dirty) {
                p.save(l);
            }
        }

        let central = egui::Frame::none().fill(palette.bg_app).inner_margin(14.0);
        egui::CentralPanel::default().frame(central).show(ctx, |ui| {
            if self.settings.folder.is_none() {
                self.file_list(ui);
            } else if self.preview.is_some() {
                // The list and the viewer each get their own column.
                ui.columns(2, |cols| {
                    theme::card(&cols[0]).show(&mut cols[0], |ui| {
                        ui.set_min_size(ui.available_size());
                        self.file_list(ui);
                    });
                    theme::card(&cols[1]).show(&mut cols[1], |ui| {
                        ui.set_min_size(ui.available_size());
                        self.preview_panel(ui);
                    });
                });
            } else {
                theme::card(ui).show(ui, |ui| {
                    ui.set_min_size(ui.available_size());
                    self.file_list(ui);
                });
            }
        });

        self.settings_window(ctx);
    }
}

impl App {
    /// Pop-up over a dimmed window to change the theme and the language.
    /// Closes with the ✕, Esc or a click outside it.
    fn settings_window(&mut self, ctx: &egui::Context) {
        if !self.settings_open {
            return;
        }
        let l = self.settings.lang;
        let mut close = ctx.input(|i| i.key_pressed(egui::Key::Escape));

        let screen = ctx.screen_rect();
        egui::Area::new(egui::Id::new("settings-backdrop"))
            .order(egui::Order::Middle)
            .fixed_pos(screen.min)
            .show(ctx, |ui| {
                let (rect, response) = ui.allocate_exact_size(screen.size(), egui::Sense::click());
                ui.painter().rect_filled(rect, 0.0, egui::Color32::from_black_alpha(120));
                close |= response.clicked();
            });

        let mut prefs = Prefs { theme: self.theme, lang: l };
        let frame = egui::Frame::window(&ctx.style()).inner_margin(22.0);
        egui::Window::new("settings")
            .title_bar(false)
            .order(egui::Order::Foreground)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .resizable(false)
            .collapsible(false)
            .fixed_size([300.0, 0.0])
            .frame(frame)
            .show(ctx, |ui| {
                let p = Palette::current(ui);
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(l.t("Settings", "Configuración")).font(theme::semibold(18.0)));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        close |= theme::icon_button(ui, "✕", l.t("Close (Esc)", "Cerrar (Esc)")).clicked();
                    });
                });
                ui.add_space(10.0);

                ui.label(egui::RichText::new(l.t("Appearance", "Apariencia")).font(theme::semibold(13.0)));
                let themes = [(Theme::Light, l.t("☀  Light", "☀  Claro")), (Theme::Dark, l.t("🌙  Dark", "🌙  Oscuro"))];
                theme::segmented(ui, "theme", &mut prefs.theme, &themes, 147.0);
                ui.add_space(12.0);

                ui.label(egui::RichText::new(l.t("Language", "Idioma")).font(theme::semibold(13.0)));
                let langs = [(Lang::Es, Lang::Es.name()), (Lang::En, Lang::En.name())];
                theme::segmented(ui, "lang", &mut prefs.lang, &langs, 147.0);
                ui.add_space(16.0);

                ui.separator();
                ui.add_space(4.0);
                ui.label(
                    egui::RichText::new(format!("{APP_NAME}  ·  v{}", env!("CARGO_PKG_VERSION")))
                        .small()
                        .color(p.text_weak),
                );
                ui.label(
                    egui::RichText::new(l.t(
                        "Merge, edit and export your .txt files in one step.",
                        "Combina, edita y exporta tus archivos .txt en un solo paso.",
                    ))
                    .small()
                    .color(p.text_weak),
                );
            });

        if prefs != (Prefs { theme: self.theme, lang: l }) {
            self.theme = prefs.theme;
            self.settings.lang = prefs.lang;
            prefs.save();
        }
        if close {
            self.settings_open = false;
        }
    }
}

fn main() -> eframe::Result {
    let icon = egui::IconData {
        rgba: include_bytes!("../assets/icon_256.rgba").to_vec(),
        width: 256,
        height: 256,
    };
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title(APP_NAME)
            .with_app_id(APP_NAME)
            .with_icon(std::sync::Arc::new(icon))
            .with_inner_size([1000.0, 680.0])
            .with_min_inner_size([760.0, 500.0])
            .with_drag_and_drop(true),
        ..Default::default()
    };
    eframe::run_native(
        APP_NAME,
        options,
        Box::new(|cc| {
            theme::install_fonts(&cc.egui_ctx);
            Ok(Box::new(App::new(&cc.egui_ctx)))
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn previous_outputs_are_recognized() {
        let n = "combined.txt";
        assert!(is_previous_output(Path::new("C:/x/combined.txt"), n));
        assert!(is_previous_output(Path::new("C:/x/Combined (12).TXT"), n));
        assert!(!is_previous_output(Path::new("C:/x/combined (a).txt"), n));
        assert!(is_previous_output(Path::new("C:/x/combined.md"), n));
        assert!(is_previous_output(Path::new("C:/x/combined (2).md"), n));
        assert!(!is_previous_output(Path::new("C:/x/combined notes.md"), n));
        assert!(!is_previous_output(Path::new("C:/x/notes.txt"), n));
    }

    #[test]
    fn output_name_takes_the_export_extension() {
        let mut s = Settings { export_extension: "md".into(), output_stem: "notes".into(), ..Default::default() };
        assert_eq!(s.output_name().as_deref(), Some("notes.md"));

        s.output_stem = "  ".into();
        assert_eq!(s.output_name().as_deref(), Some("combined.md"));

        s.use_custom = true;
        s.custom_extension = ".ADOC".into();
        assert_eq!(s.output_name().as_deref(), Some("combined.adoc"));

        s.custom_extension = String::new();
        assert_eq!(s.output_name(), None);
    }

    #[test]
    fn exporting_as_md_still_reads_txt_files_but_not_old_md_outputs() {
        let dir = tempfile::tempdir().unwrap();
        for f in ["x.md", "y.txt", "z.csv", "combined.md", "combined (2).md"] {
            std::fs::write(dir.path().join(f), "hola").unwrap();
        }
        let s = Settings { export_extension: "md".into(), ..Default::default() };
        let name = s.output_name().unwrap();
        let found = scan(dir.path(), false, &|p| is_previous_output(p, &name));
        let names: Vec<_> = found.iter().map(|p| display_path(dir.path(), p)).collect();
        assert_eq!(names, vec!["x.md", "y.txt", "z.csv"]);
    }

    #[test]
    fn every_variable_has_help_in_both_languages() {
        for var in SEPARATOR_VARIABLES {
            assert!(!variable_help(Lang::En, var).is_empty(), "{var}");
            assert!(!variable_help(Lang::Es, var).is_empty(), "{var}");
        }
    }

    #[test]
    fn viewer_saves_edits_keeping_crlf_and_encoding() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("a.txt");
        std::fs::write(&path, b"uno\r\ndos caf\xE9\r\n").unwrap();

        let mut p = Preview::load(path.clone(), "a.txt".into());
        assert_eq!(p.text.as_deref(), Some("uno\ndos café\n"));
        p.text.as_mut().unwrap().push_str("tres\n");
        p.dirty = true;

        assert!(p.save(Lang::Es));
        assert!(!p.dirty);
        assert_eq!(std::fs::read(&path).unwrap(), b"uno\r\ndos caf\xE9\r\ntres\r\n");
    }

    #[test]
    fn viewer_does_not_save_a_partly_loaded_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("big.txt");
        let content = "x".repeat(PREVIEW_MAX_BYTES + 10);
        std::fs::write(&path, &content).unwrap();

        let mut p = Preview::load(path.clone(), "big.txt".into());
        assert!(p.truncated);
        assert!(!p.save(Lang::En));
        assert_eq!(std::fs::read_to_string(&path).unwrap(), content);
    }

    #[test]
    fn wrapped_rows_share_their_line_number() {
        // Line 1 fits one row, line 2 wraps over three rows, line 3 is the last.
        let rows = [true, false, false, true, false];
        assert_eq!(row_line_numbers(rows), vec![Some(1), Some(2), None, None, Some(3)]);
        // A trailing newline leaves an empty last line that still gets its number.
        assert_eq!(row_line_numbers([true, true, false]), vec![Some(1), Some(2), Some(3)]);
    }

    #[test]
    fn edited_copies_are_not_listed_again() {
        let root = Path::new("C:/docs");
        assert!(is_in_edit_folder(root, Path::new("C:/docs/editados/a.txt"), "editados"));
        assert!(is_in_edit_folder(root, Path::new("C:/docs/Editados (3)/sub/a.txt"), "editados"));
        assert!(!is_in_edit_folder(root, Path::new("C:/docs/editados.txt"), "editados"));
        assert!(!is_in_edit_folder(root, Path::new("C:/docs/notas/editados/a.txt"), "editados"));
        assert!(!is_in_edit_folder(root, Path::new("C:/docs/a.txt"), "editados"));
    }

    #[test]
    fn every_edit_error_has_a_message_in_both_languages() {
        let errors = [
            EditError::LineNumbers(String::new()),
            EditError::LineNumbers("x".into()),
            EditError::Pattern("bad".into()),
            EditError::EmptyDeleteText,
            EditError::EmptyFind,
        ];
        for e in &errors {
            assert!(!edit_error_text(Lang::En, e).is_empty());
            assert!(!edit_error_text(Lang::Es, e).is_empty());
        }
    }

    #[test]
    fn prefs_round_trip_and_ignore_unknown_values() {
        let fallback = Prefs { theme: Theme::Light, lang: Lang::En };
        let dark_es = Prefs { theme: Theme::Dark, lang: Lang::Es };
        assert_eq!(Prefs::parse(&dark_es.to_text(), fallback), dark_es);
        assert_eq!(Prefs::parse("", fallback), fallback);
        assert_eq!(Prefs::parse("theme=purple\nlang=fr\nother=1", fallback), fallback);
        assert_eq!(Prefs::parse("lang=es", fallback), Prefs { theme: Theme::Light, lang: Lang::Es });
    }

    #[test]
    fn language_picks_the_matching_text() {
        assert_eq!(Lang::Es.t("Open", "Abrir"), "Abrir");
        assert_eq!(Lang::En.t("Open", "Abrir"), "Open");
    }
}
