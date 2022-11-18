use camino::{Utf8Path as Path, Utf8PathBuf as PathBuf};
use serde::Deserialize;
use toml::Value;

use super::{CmdSpec, Format, Metadata};
use crate::error::*;
use crate::util::PathBufExt;

#[derive(Deserialize, Debug)]
pub struct Output {
    pub file: PathBuf,
    pub template: Option<PathBuf>,

    #[serde(default)]
    pub format: Format,

    #[serde(rename = "process")]
    pub post_process: Option<CmdSpec>,
    #[serde(rename = "process_win")]
    pub post_process_win: Option<CmdSpec>,
    /// *Instead* of executing `process` or `process_win`, use the build-in Tectonic library to create Pdf.
    ///
    /// If true, changes format to pdf, changes output file extension to `.pdf` and removes
    /// `post_process` and `post_process_win`
    ///
    /// (This is a temporary and hacky solution to make the same configuration files also work with
    /// bard without tectonic support - it will just use the `process` setting)
    #[serde(rename = "process_into_pdf")]
    pub use_tectonic: Option<bool>,

    #[serde(flatten)]
    pub metadata: Metadata,
}

impl Output {
    pub fn resolve(&mut self, dir_templates: &Path, dir_output: &Path) -> Result<()> {
        if let Some(template) = self.template.as_mut() {
            template.resolve(dir_templates);
        }
        self.file.resolve(dir_output);

        if !matches!(self.format, Format::Auto) {
            return Ok(());
        }

        let ext = self.file.extension().map(str::to_lowercase);

        self.format = match ext.as_deref() {
            Some("html") => Format::Html,
            Some("tex") => Format::Tex,
            Some("pdf") => Format::Pdf,
            Some("xml") => Format::Xml,
            Some("json") => Format::Json,
            _ => bail!(
                "Unknown or unsupported format of output file: {}\nHint: Specify format with  \
                 'format = ...'",
                self.file
            ),
        };

        // Hack - if use_tectonic is enabled, change format to pdf, output extension to .pdf and
        // disable post process to compile the file with tectonic instead of post_process
        if self.use_tectonic == Some(true){
            self.format = Format::Pdf;
            self.file.set_extension("pdf");
            self.post_process = None;
            self.post_process_win = None;
        }

        Ok(())
    }

    pub fn output_filename(&self) -> &str {
        self.file.file_name().expect("OutputSpec: Invalid filename")
    }

    pub fn template_path(&self) -> Option<&Path> {
        match self.format {
            Format::Html | Format::Tex | Format::Pdf | Format::Hovorka => self.template.as_deref(),
            Format::Json | Format::Xml => None,
            Format::Auto => Format::no_auto(),
        }
    }

    pub fn post_process(&self) -> Option<&CmdSpec> {
        if cfg!(windows) && self.post_process_win.is_some() {
            return self.post_process_win.as_ref();
        }

        self.post_process.as_ref()
    }

    pub fn template_filename(&self) -> String {
        self.template
            .as_ref()
            .map(|p| p.to_string())
            .unwrap_or_else(|| String::from("<builtin>"))
    }

    pub fn dpi(&self) -> f64 {
        const DEFAULT: f64 = 144.0;

        self.metadata
            .get("dpi")
            .and_then(|value| match value {
                Value::Integer(i) => Some(*i as f64),
                Value::Float(f) => Some(*f),
                _ => None,
            })
            .unwrap_or(DEFAULT)
    }
}
