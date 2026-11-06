<p align="center">
  <a href="README.md">English</a> · <strong>Español</strong>
</p>

<p align="center">
  <img src="assets/banner.png" width="480" alt="TXT Merge - MERGER & EDITOR">
</p>

<p align="center">
  <strong>Herramienta moderna, rápida y ligera para unir archivos de texto y edición masiva por lotes.</strong>
</p>

<p align="center">
  <a href="https://github.com/SrLink16/TXT-Merge/releases/latest"><img src="https://img.shields.io/github/v/release/SrLink16/TXT-Merge?style=flat-square&color=008080" alt="Última versión"></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/licencia-GPL--3.0-blue?style=flat-square" alt="Licencia: GPL-3.0"></a>
  <img src="https://img.shields.io/badge/plataforma-Windows%2010%2F11-blue?style=flat-square" alt="Plataforma Windows">
  <img src="https://img.shields.io/badge/hecho%20con-Rust-orange?style=flat-square&logo=rust" alt="Rust">
  <img src="https://img.shields.io/badge/GUI-egui-purple?style=flat-square" alt="egui">
  <img src="https://img.shields.io/badge/portable-sin%20instalador-success?style=flat-square" alt="Portable">
</p>

---

## ⚡ Descripción

**TXT Merge** es una aplicación de escritorio nativa desarrollada en Rust, diseñada para unir múltiples archivos de texto en un único archivo estructurado, así como aplicar modificaciones masivas (búsqueda y reemplazo con regex) con previsualización en vivo de diferencias (*diff*).

Es totalmente portátil, ultrarrápida y no requiere instalación.

---

## ✨ Características

### 📂 Combinador de Archivos (Merger)
* **Detección inteligente de texto**: Reconoce archivos de texto y filtra automáticamente binarios analizando encabezados y codificación.
* **Exportación multiformato**: Unifica archivos en `.txt`, `.md`, `.csv`, `.tsv`, `.json`, `.yaml`, `.xml`, `.html`, `.toml`, `.sql`, `.srt` y muchos más.
* **Separadores personalizables**: Agrega encabezados con nombres de archivo, rutas relativas, saltos o líneas divisorias entre cada texto.
* **Arrastrar y Soltar (Drag & Drop)**: Arrastra una carpeta para listar todos sus archivos de texto, o uno o varios archivos para trabajar solo con ellos.
* **Compatibilidad de codificación**: Detección y decodificación automática para UTF-8, UTF-16, Latin-1 y Windows-1252.

### 📝 Editor por Lotes (Batch Editor)
* **Búsqueda y reemplazo múltiple**: Aplica reglas de texto o expresiones regulares (regex) en todos los archivos a la vez.
* **Previsualización de diferencias (Diff)**: Inspecciona visualmente los cambios línea por línea antes de guardar cualquier archivo en el disco.
* **Operación segura**: Previsualización no destructiva para evitar modificaciones accidentales.

### 🎨 Interfaz Moderna
* **Temas visuales**: Modo Oscuro y Modo Claro.
* **Soporte multilingüe**: Interfaz nativa en Español e Inglés (se adapta automáticamente al idioma de tu sistema).

---

## 🔤 Variables del separador

Escribe estas etiquetas en el separador y cada una se reemplaza en cada archivo. El valor después de `:` se puede editar.

| Variable | Se reemplaza por |
|---|---|
| `<Inc Nr:1>` | Número ascendente. `<Inc Nr:10>` empieza en 10; `<Inc Nr:001>` rellena con ceros (001, 002…) |
| `<Inc NrDir:1>` | Número ascendente que vuelve a empezar en cada carpeta |
| `<Inc Alpha:A>` | Letra ascendente: A, B … Z, AA. `<Inc Alpha:c>` empieza en c en minúscula |
| `<Name>` | Nombre del archivo sin extensión |
| `<Ext>` | Extensión del archivo sin punto |
| `<FolderName:1>` | Nombre de la carpeta del archivo; `2` es la carpeta de arriba, y así |
| `<Num Files>` | Número de archivos de la lista en la misma carpeta |
| `<Num Dirs>` | Número de carpetas dentro de la carpeta del archivo |
| `<Num Items>` | Número total de archivos que se combinan |
| `<Word:1>` | Palabra N del nombre del archivo |
| `<RWord:1>` | Palabra N contando desde el final del nombre del archivo |
| `<MediaType>` | Tipo de medio (MIME) según la extensión, ej. `text/markdown` |

Ejemplo: `=== <Inc Nr:1>/<Num Items> - <Name> ===` queda como `=== 3/12 - Informe ===`.
Un valor vacío (`<Inc Nr:>`) usa el valor por defecto; uno no válido se deja tal cual.

---

## ✏️ Operaciones de edición en masa

La pestaña **Editar** aplica estos pasos a todos los archivos marcados, en este orden:

1. **Borrar líneas por número** — ej. `1-3, 10, 20-` (`20-` significa desde la línea 20 hasta el final).
2. **Borrar líneas que contengan** un texto.
   * *Borrar también la línea vacía que queda después* (activado por defecto), así al borrar un título no queda una línea en blanco arriba.
3. **Reemplazar texto** — texto normal o expresiones regulares (`$1` inserta el grupo 1), con *Coincidir mayúsculas* opcional. Un reemplazo vacío borra lo encontrado.
4. **Quitar espacios** al inicio y al final de cada línea.
5. **Quitar líneas vacías**, o **dejar como máximo una línea vacía seguida**. Las líneas con solo espacios cuentan como vacías.

Antes de aplicar, pulsa **Ver** en un archivo para ver los **Cambios** (diff estilo GitHub), el **Resultado** o el **Original** (editable).
Los resultados se guardan como **copias en una subcarpeta nueva** (`editados`, `editados (2)`…) o **sobrescribiendo los originales**. Cada archivo conserva su codificación y sus saltos de línea.

---

## 🚀 Descarga e Instalación

### Windows (Portable)
No requiere instalación:

1. Ve a la sección de [**Últimas Releases**](https://github.com/SrLink16/TXT-Merge/releases/latest).
2. Descarga **`TXT-Merge.exe`**.
3. ¡Ábrelo directamente! (Compatible con Windows 10 y 11).

---

## 🛠️ Compilar desde el código fuente

Asegúrate de tener [Rust](https://www.rust-lang.org/) instalado:

```bash
git clone https://github.com/SrLink16/TXT-Merge.git
cd TXT-Merge
cargo build --release
```

El binario compilado estará ubicado en `target/release/TXT-Merge.exe`.

---

## 📋 Historial de versiones

### v0.2.0
* Al arrastrar uno o varios archivos, la lista tiene solo esos archivos; al arrastrar una carpeta se sigue listando la carpeta entera.
* La barra superior muestra *(solo archivos arrastrados)* cuando la lista viene de archivos arrastrados.
* Arreglada la compilación desde el código fuente (Rust edition 2024).

### v0.1.0 — Initial Launch
* Combinador de archivos con exportación a varios formatos, separadores personalizados y variables.
* Editor en masa con vista previa de cambios estilo GitHub.
* Visor y editor de archivos con números de línea que conserva la codificación de cada archivo.
* Temas claro y oscuro; interfaz en inglés y español.

---

## 👤 Autor

* **[SrLink16](https://github.com/SrLink16)**

---

## 📄 Licencia

Este proyecto está bajo la licencia **GNU General Public License v3.0 (GPL-3.0)**. Consulta el archivo [LICENSE](LICENSE) para más detalles.
