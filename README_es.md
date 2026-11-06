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

## 👤 Autor

* **[SrLink16](https://github.com/SrLink16)**

---

## 📄 Licencia

Este proyecto está bajo la licencia **GNU General Public License v3.0 (GPL-3.0)**. Consulta el archivo [LICENSE](LICENSE) para más detalles.
