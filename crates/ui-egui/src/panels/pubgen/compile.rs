//! Publication compiler: one `pubgen_project.json` manifest compiles a whole publication —
//! covers, front matter, chapters, global lists, running headers and back cover — into one
//! `.designcraft` document (and optionally a PDF).
//!
//! Manifest paths resolve against the manifest's own folder. Nothing here mutates a document
//! until the plan validates; saving is the second-to-last step, so a failed compile never
//! replaces a previous build.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

use designcraft_doc::{Selection, TextSel};
use egui::{RichText, Ui};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::{
    PubgenState, TOC_NARRATIVE_STYLE, TocPlan, apply_excel_cover, execute_convert_excel, execute_generate_back_cover, execute_generate_front_cover,
    execute_generate_inside_cover, execute_generate_pages, generate_toc, is_chart_type, is_divider_type, is_ulasan_type, load_card1_csv,
    toc_kind_title, toc_story_page,
};
use crate::DesignApp;
use crate::theme::Tokens;
use crate::widgets::{FIELD_H, outline_button, section};

/// A publication at most this big; the engine's book batch allows 4096, a publication does not.
const MAX_CHAPTERS: usize = 64;
const MAX_TITLE_CHARS: usize = 512;
const MAX_PATH_CHARS: usize = 4096;

const DEFAULT_MARGIN_TOP_MM: f64 = 20.20;
const DEFAULT_MARGIN_BOTTOM_MM: f64 = 20.0;
const DEFAULT_MARGIN_INSIDE_MM: f64 = 19.0;
const DEFAULT_MARGIN_OUTSIDE_MM: f64 = 15.0;
const DEFAULT_BLEED_MM: f64 = 3.0;

/// Which pages a header/footer preset covers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum HeaderScope {
    Body,
    All,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default, rename_all = "camelCase")]
pub(crate) struct ProjectManifest {
    pub version: u32,
    pub publication_title: String,
    pub bilingual: bool,
    #[serde(default)]
    pub profile: Option<String>,
    pub output: OutputSpec,
    pub document: DocumentSpec,
    pub cover: CoverSpec,
    pub front_matter: FrontMatterSpec,
    pub chapters: Vec<ChapterSpec>,
    pub pagination: PaginationSpec,
    pub toc: TocSpec,
    pub header_footer: HeaderFooterSpec,
    pub build: BuildSpec,
}

impl Default for ProjectManifest {
    fn default() -> Self {
        Self {
            version: 1,
            publication_title: String::new(),
            bilingual: false,
            profile: None,
            output: OutputSpec::default(),
            document: DocumentSpec::default(),
            cover: CoverSpec::default(),
            front_matter: FrontMatterSpec::default(),
            chapters: Vec::new(),
            pagination: PaginationSpec::default(),
            toc: TocSpec::default(),
            header_footer: HeaderFooterSpec::default(),
            build: BuildSpec::default(),
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(default, rename_all = "camelCase")]
pub(crate) struct OutputSpec {
    /// Compiled document path; empty = `<manifest stem>.designcraft` beside the manifest.
    pub document: String,
    /// PDF path when `build.export_pdf`; empty = the document path with `.pdf`.
    pub pdf: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default, rename_all = "camelCase")]
pub(crate) struct DocumentSpec {
    pub preset: String,
    pub facing_pages: bool,
    pub bleed_mm: f64,
    pub margins_mm: MarginSpec,
}

impl Default for DocumentSpec {
    fn default() -> Self {
        Self { preset: "A4".into(), facing_pages: true, bleed_mm: DEFAULT_BLEED_MM, margins_mm: MarginSpec::default() }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(default, rename_all = "camelCase")]
pub(crate) struct MarginSpec {
    pub top: f64,
    pub bottom: f64,
    pub inside: f64,
    pub outside: f64,
}

impl Default for MarginSpec {
    fn default() -> Self {
        Self { top: DEFAULT_MARGIN_TOP_MM, bottom: DEFAULT_MARGIN_BOTTOM_MM, inside: DEFAULT_MARGIN_INSIDE_MM, outside: DEFAULT_MARGIN_OUTSIDE_MM }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default, rename_all = "camelCase")]
pub(crate) struct CoverSpec {
    /// Workbook carrying the `COVER` sheet; empty = the first chapter workbook with one.
    pub source: String,
    pub style: usize,
    pub theme_preset: usize,
    pub include_inside: bool,
    pub include_back: bool,
}

impl Default for CoverSpec {
    fn default() -> Self {
        Self { source: String::new(), style: 0, theme_preset: 0, include_inside: true, include_back: true }
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(default, rename_all = "camelCase")]
pub(crate) struct FrontMatterSpec {
    /// Dedicated front-matter workbook; empty = the leading narrative rows of chapter 1.
    pub workbook: String,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(default, rename_all = "camelCase")]
pub(crate) struct ChapterSpec {
    pub chapter: u32,
    pub path: String,
    pub title: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default, rename_all = "camelCase")]
pub(crate) struct PaginationSpec {
    pub front_matter_format: String,
    pub body_format: String,
    pub restart_at_body: bool,
}

impl Default for PaginationSpec {
    fn default() -> Self {
        Self { front_matter_format: "lowerRoman".into(), body_format: "arabic".into(), restart_at_body: true }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default, rename_all = "camelCase")]
pub(crate) struct TocSpec {
    /// `afterFrontMatter` or `end`.
    pub placement: String,
    /// Any of `contents`, `tables`, `charts`, in generation order.
    pub lists: Vec<String>,
    pub title_bilingual: bool,
    pub hanging_indent: bool,
}

impl Default for TocSpec {
    fn default() -> Self {
        Self {
            placement: "afterFrontMatter".into(),
            lists: vec!["contents".into(), "tables".into(), "charts".into()],
            title_bilingual: true,
            hanging_indent: true,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default, rename_all = "camelCase")]
pub(crate) struct HeaderFooterSpec {
    pub preset: String,
    pub scope: String,
}

impl Default for HeaderFooterSpec {
    fn default() -> Self {
        Self { preset: "book".into(), scope: "body".into() }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default, rename_all = "camelCase")]
pub(crate) struct BuildSpec {
    pub keep_intermediate: bool,
    pub export_pdf: bool,
}

impl Default for BuildSpec {
    fn default() -> Self {
        Self { keep_intermediate: true, export_pdf: false }
    }
}

/// Where a chapter's pages come from. The manifest accepts workbooks only; catalogs exist so the
/// pipeline can be tested without binary XLSX fixtures.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum ChapterSource {
    Workbook(PathBuf),
    /// Pre-converted CSVs, for tests and tools; the manifest itself names workbooks only.
    #[cfg_attr(not(test), allow(dead_code))]
    Catalog {
        catalog: PathBuf,
        tables: Vec<PathBuf>,
    },
}

impl ChapterSource {
    fn workbook(&self) -> &Path {
        match self {
            Self::Workbook(path) => path,
            Self::Catalog { catalog, .. } => catalog,
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct ChapterPlan {
    pub chapter: u32,
    pub title: String,
    pub source: ChapterSource,
}

impl ChapterPlan {
    pub(crate) fn label(&self) -> String {
        if self.title.trim().is_empty() { self.source.workbook().display().to_string() } else { self.title.clone() }
    }
}

/// One unit of work; the job shows it and the pipeline dispatches on it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum CompileStep {
    NewDocument,
    MasterSetup,
    FrontCover,
    InsideCover,
    NormalizeSeedPage,
    FrontMatter,
    TocPlaceholders,
    Chapter(usize),
    BackCover,
    TocFinalize,
    Sections,
    RunningHeaders,
    SaveDocument,
    ExportPdf,
}

impl CompileStep {
    pub(crate) fn label(&self, plan: &CompilePlan) -> String {
        match self {
            Self::NewDocument => "New document".into(),
            Self::MasterSetup => "Master pages".into(),
            Self::FrontCover => "Front cover".into(),
            Self::InsideCover => "Inside cover".into(),
            Self::NormalizeSeedPage => "Normalize pages".into(),
            Self::FrontMatter => "Front matter".into(),
            Self::TocPlaceholders => "List placeholders".into(),
            Self::Chapter(i) => {
                plan.chapters.get(*i).map_or_else(|| format!("Chapter {}", i + 1), |c| format!("Chapter {} — {}", c.chapter, c.label()))
            }
            Self::BackCover => "Back cover".into(),
            Self::TocFinalize => "Contents lists".into(),
            Self::Sections => "Page numbering".into(),
            Self::RunningHeaders => "Running headers".into(),
            Self::SaveDocument => "Save".into(),
            Self::ExportPdf => "Export PDF".into(),
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct CompilePlan {
    pub manifest: ProjectManifest,
    pub manifest_path: PathBuf,
    pub output_document: PathBuf,
    pub output_pdf: Option<PathBuf>,
    pub chapters: Vec<ChapterPlan>,
    pub warnings: Vec<String>,
}

impl CompilePlan {
    /// Read and validate a manifest.
    pub(crate) fn load(path: &Path) -> Result<Self, String> {
        let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
        let manifest: ProjectManifest = serde_json::from_str(&text).map_err(|e| format!("{}: not a publication manifest: {e}", path.display()))?;
        Self::from_manifest(path, manifest)
    }

    /// Validate a parsed manifest and resolve every path against its folder.
    pub(crate) fn from_manifest(manifest_path: &Path, manifest: ProjectManifest) -> Result<Self, String> {
        let dir = manifest_path.parent().filter(|p| !p.as_os_str().is_empty()).unwrap_or_else(|| Path::new("."));
        let mut warnings = Vec::new();

        let mut manifest = manifest;
        if let Some(ref prof_id) = manifest.profile {
            let prof = super::profiles::ProfileManager::get_profile(prof_id);
            if (manifest.document.margins_mm.top - DEFAULT_MARGIN_TOP_MM).abs() < 1e-4
                && (manifest.document.margins_mm.bottom - DEFAULT_MARGIN_BOTTOM_MM).abs() < 1e-4
            {
                manifest.document.margins_mm = MarginSpec {
                    top: prof.geometry.margins_mm.top,
                    bottom: prof.geometry.margins_mm.bottom,
                    inside: prof.geometry.margins_mm.inside,
                    outside: prof.geometry.margins_mm.outside,
                };
                manifest.document.facing_pages = prof.geometry.facing_pages;
            }
        }
        let chapters = plan_chapters(dir, &manifest, &mut warnings)?;
        let output_document = output_path(dir, manifest_path, &manifest, &mut warnings)?;
        let output_pdf = if manifest.build.export_pdf { Some(pdf_path(dir, &manifest, &output_document)?) } else { None };
        validate_lists(&manifest)?;
        validate_numbering(&manifest)?;

        if manifest.publication_title.chars().count() > MAX_TITLE_CHARS {
            return Err(format!("publicationTitle: at most {MAX_TITLE_CHARS} characters"));
        }
        if manifest.cover.source.chars().count() > MAX_PATH_CHARS {
            return Err("cover.source: path too long".into());
        }
        if manifest.cover.source.trim().is_empty() {
            warnings.push("cover.source is empty: the first chapter workbook with a COVER sheet supplies the cover".into());
        }

        Ok(Self { manifest, manifest_path: manifest_path.to_path_buf(), output_document, output_pdf, chapters, warnings })
    }

    /// The ordered pipeline for this plan.
    pub(crate) fn steps(&self) -> Vec<CompileStep> {
        let prof = self.manifest.profile.as_deref().map(super::profiles::ProfileManager::get_profile);
        let include_inside = prof.as_ref().map(|p| p.structure.include_inside_cover).unwrap_or(true) && self.manifest.cover.include_inside;
        let include_back = prof.as_ref().map(|p| p.structure.include_back_cover).unwrap_or(true) && self.manifest.cover.include_back;

        let mut steps = vec![CompileStep::NewDocument, CompileStep::MasterSetup, CompileStep::FrontCover];
        if include_inside {
            steps.push(CompileStep::InsideCover);
        }
        steps.push(CompileStep::NormalizeSeedPage);
        // Without a dedicated workbook the first chapter supplies the leading narrative rows; the
        // step is planned anyway and discovers them at compile time.
        steps.push(CompileStep::FrontMatter);
        let is_brs = self.manifest.profile.as_deref() == Some("bps_brs");
        let toc_at_end = self.manifest.toc.placement == "end";
        if !is_brs && !toc_at_end && !self.manifest.toc.lists.is_empty() {
            steps.push(CompileStep::TocPlaceholders);
        }
        steps.extend((0..self.chapters.len()).map(CompileStep::Chapter));
        if !is_brs && toc_at_end && !self.manifest.toc.lists.is_empty() {
            steps.push(CompileStep::TocPlaceholders);
        }
        if include_back {
            steps.push(CompileStep::BackCover);
        }
        if !is_brs && !self.manifest.toc.lists.is_empty() {
            steps.push(CompileStep::TocFinalize);
        }
        steps.extend([CompileStep::Sections, CompileStep::RunningHeaders, CompileStep::SaveDocument]);
        if self.output_pdf.is_some() {
            steps.push(CompileStep::ExportPdf);
        }
        steps
    }

    /// A dedicated front-matter workbook adds pages of its own; the chapter-1 fallback is emitted
    /// by the chapter steps, which discover the leading narrative rows at compile time.
    pub(crate) fn has_front_matter(&self) -> bool {
        !self.manifest.front_matter.workbook.trim().is_empty()
    }

    pub(crate) fn dir(&self) -> &Path {
        self.manifest_path.parent().filter(|p| !p.as_os_str().is_empty()).unwrap_or_else(|| Path::new("."))
    }

    pub(crate) fn resolve(&self, relative: &str) -> PathBuf {
        resolve_against(self.dir(), relative)
    }

    pub(crate) fn header_scope(&self) -> HeaderScope {
        if self.manifest.header_footer.scope.trim().eq_ignore_ascii_case("all") { HeaderScope::All } else { HeaderScope::Body }
    }

    /// One line for the Publication card: what the manifest makes and where it goes.
    pub(crate) fn summary_line(&self) -> String {
        let title = if self.manifest.publication_title.trim().is_empty() {
            self.manifest_path.file_stem().and_then(std::ffi::OsStr::to_str).unwrap_or("Publication").to_string()
        } else {
            self.manifest.publication_title.trim().to_string()
        };
        let chapters = self.chapters.len();
        let mut line = format!("{title} — {chapters} chapter{} → {}", if chapters == 1 { "" } else { "s" }, self.output_document.display());
        if let Some(pdf) = &self.output_pdf {
            line.push_str(&format!(" + {}", pdf.display()));
        }
        if !self.warnings.is_empty() {
            line.push_str(&format!(" ({} warning{})", self.warnings.len(), if self.warnings.len() == 1 { "" } else { "s" }));
        }
        line
    }
}

fn resolve_against(dir: &Path, path: &str) -> PathBuf {
    let p = Path::new(path);
    if p.is_absolute() { p.to_path_buf() } else { dir.join(p) }
}

fn plan_chapters(dir: &Path, manifest: &ProjectManifest, warnings: &mut Vec<String>) -> Result<Vec<ChapterPlan>, String> {
    if manifest.chapters.is_empty() {
        return Err("chapters: at least one chapter is required".into());
    }
    if manifest.chapters.len() > MAX_CHAPTERS {
        return Err(format!("chapters: at most {MAX_CHAPTERS} chapters"));
    }
    let mut paths = HashSet::new();
    let mut numbers = HashSet::new();
    let mut plans = Vec::with_capacity(manifest.chapters.len());
    let mut previous = None;
    for (i, chapter) in manifest.chapters.iter().enumerate() {
        if chapter.path.trim().is_empty() {
            return Err(format!("chapters[{i}].path: required"));
        }
        if chapter.path.chars().count() > MAX_PATH_CHARS {
            return Err(format!("chapters[{i}].path: at most {MAX_PATH_CHARS} characters"));
        }
        if chapter.title.chars().count() > MAX_TITLE_CHARS {
            return Err(format!("chapters[{i}].title: at most {MAX_TITLE_CHARS} characters"));
        }
        if chapter.chapter == 0 {
            return Err(format!("chapters[{i}].chapter: must be at least 1"));
        }
        if !numbers.insert(chapter.chapter) {
            return Err(format!("chapters[{i}].chapter: {} appears twice", chapter.chapter));
        }
        let path = resolve_against(dir, &chapter.path);
        let key = std::fs::canonicalize(&path).unwrap_or_else(|_| path.clone());
        if !paths.insert(key.clone()) {
            return Err(format!("chapters[{i}].path: {} is used twice", path.display()));
        }
        if !path.is_file() {
            return Err(format!("chapters[{i}].path: {} is not a file", path.display()));
        }
        let extension = path.extension().and_then(std::ffi::OsStr::to_str).unwrap_or("").to_ascii_lowercase();
        if extension != "xlsx" && extension != "xlsm" {
            return Err(format!("chapters[{i}].path: {} is not an .xlsx/.xlsm workbook", path.display()));
        }
        if let Some(previous) = previous
            && chapter.chapter < previous
        {
            warnings.push(format!("chapters are out of order: {} follows {previous}", chapter.chapter));
        }
        previous = Some(chapter.chapter);
        plans.push(ChapterPlan { chapter: chapter.chapter, title: chapter.title.trim().to_string(), source: ChapterSource::Workbook(path) });
    }
    Ok(plans)
}

fn output_path(dir: &Path, manifest_path: &Path, manifest: &ProjectManifest, _warnings: &mut Vec<String>) -> Result<PathBuf, String> {
    let document = if manifest.output.document.trim().is_empty() {
        let stem = manifest_path.file_stem().and_then(std::ffi::OsStr::to_str).unwrap_or("publication");
        dir.join(format!("{stem}.designcraft"))
    } else {
        resolve_against(dir, manifest.output.document.trim())
    };
    if document.extension().and_then(std::ffi::OsStr::to_str).is_none() {
        return Err(format!("output.document: {} needs a .designcraft extension", document.display()));
    }
    require_writable_parent(&document, "output.document")?;
    Ok(document)
}

fn pdf_path(dir: &Path, manifest: &ProjectManifest, document: &Path) -> Result<PathBuf, String> {
    let pdf = if manifest.output.pdf.trim().is_empty() { document.with_extension("pdf") } else { resolve_against(dir, manifest.output.pdf.trim()) };
    if pdf.extension().and_then(std::ffi::OsStr::to_str).is_none() {
        return Err(format!("output.pdf: {} needs a .pdf extension", pdf.display()));
    }
    require_writable_parent(&pdf, "output.pdf")?;
    Ok(pdf)
}

fn require_writable_parent(path: &Path, field: &str) -> Result<(), String> {
    let parent = path.parent().ok_or_else(|| format!("{field}: {} has no folder", path.display()))?;
    if !parent.is_dir() {
        return Err(format!("{field}: folder {} does not exist", parent.display()));
    }
    let probe = parent.join(format!(".pubgen-write-probe-{}", std::process::id()));
    match std::fs::OpenOptions::new().create(true).write(true).truncate(true).open(&probe) {
        Ok(_) => {
            let _ = std::fs::remove_file(&probe);
            Ok(())
        }
        Err(e) => Err(format!("{field}: folder {} is not writable: {e}", parent.display())),
    }
}

fn validate_lists(manifest: &ProjectManifest) -> Result<(), String> {
    let mut seen = HashSet::new();
    for (i, list) in manifest.toc.lists.iter().enumerate() {
        let list = list.trim();
        if !matches!(list, "contents" | "tables" | "charts") {
            return Err(format!("toc.lists[{i}]: `{list}` is not contents, tables or charts"));
        }
        if !seen.insert(list) {
            return Err(format!("toc.lists[{i}]: {list} appears twice"));
        }
    }
    if manifest.toc.placement != "afterFrontMatter" && manifest.toc.placement != "end" {
        return Err("toc.placement: afterFrontMatter or end".into());
    }
    Ok(())
}

fn validate_numbering(manifest: &ProjectManifest) -> Result<(), String> {
    for (field, style) in
        [("pagination.frontMatterFormat", &manifest.pagination.front_matter_format), ("pagination.bodyFormat", &manifest.pagination.body_format)]
    {
        if !matches!(style.as_str(), "arabic" | "upperRoman" | "lowerRoman" | "upperLetters" | "lowerLetters") {
            return Err(format!("{field}: unsupported numbering style `{style}`"));
        }
    }
    Ok(())
}

/// One compiled chapter in the report.
#[derive(Clone, Debug)]
pub(crate) struct ChapterReport {
    pub chapter: u32,
    pub workbook: String,
    pub pages: usize,
    pub tables: usize,
    pub charts: usize,
}

/// What a finished compile produced.
#[derive(Clone, Debug, Default)]
pub(crate) struct CompileReport {
    pub document: String,
    pub pdf: Option<String>,
    pub pages: usize,
    pub chapters: Vec<ChapterReport>,
    pub warnings: Vec<String>,
    pub elapsed_ms: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum JobState {
    Running,
    Done,
    Cancelled,
    Failed(String),
}

/// One step's result: the job keeps going, or it stopped and says why.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum StepOutcome {
    Pending,
    Done,
    Cancelled,
    Failed(String),
}

#[derive(Clone, Debug)]
struct PreparedChapter {
    state: PubgenState,
    /// Leading `G`/`Note`/`Ulasan` rows before the first divider (front matter of a chapter that
    /// has no dedicated workbook).
    narrative_prefix: usize,
}

/// The compiler's step machine. A panel advances it one step per frame; `run_all` runs it in one
/// call for the command palette, the control channel and tests.
#[derive(Clone, Debug)]
pub(crate) struct CompileJob {
    pub(crate) plan: CompilePlan,
    steps: Vec<CompileStep>,
    next: usize,
    pub(crate) state: JobState,
    /// The document the compiler created; a switch to another tab stops the job instead of
    /// building chapters into the wrong document.
    doc_uid: Option<u64>,
    base: PubgenState,
    prepared: Vec<Option<PreparedChapter>>,
    front_matter_rows: usize,
    body_start: Option<usize>,
    back_cover: Option<usize>,
    toc_pages: Vec<(String, usize)>,
    chapter_ranges: Vec<usize>,
    chapter_stats: Vec<ChapterReport>,
    warnings: Vec<String>,
    started: Instant,
    cancelled: Arc<AtomicBool>,
    report: CompileReport,
}

impl CompileJob {
    pub(crate) fn new(plan: CompilePlan) -> Self {
        let mut base = PubgenState {
            bilingual_output: plan.manifest.bilingual,
            toc_pair_bilingual: plan.manifest.bilingual,
            cover_style: plan.manifest.cover.style,
            cover_theme_preset: plan.manifest.cover.theme_preset,
            cover_include_inside_back: false,
            toc_hanging_indent: plan.manifest.toc.hanging_indent,
            translate_with_ai: false,
            ..PubgenState::default()
        };
        if !plan.manifest.publication_title.trim().is_empty() {
            base.cover_title = plan.manifest.publication_title.trim().to_string();
        }
        let steps = plan.steps();
        let warnings = plan.warnings.clone();
        let prepared = (0..plan.chapters.len()).map(|_| None).collect();
        Self {
            plan,
            steps,
            next: 0,
            state: JobState::Running,
            doc_uid: None,
            base,
            prepared,
            front_matter_rows: 0,
            body_start: None,
            back_cover: None,
            toc_pages: Vec::new(),
            chapter_ranges: Vec::new(),
            chapter_stats: Vec::new(),
            warnings,
            started: Instant::now(),
            cancelled: Arc::new(AtomicBool::new(false)),
            report: CompileReport::default(),
        }
    }

    pub(crate) fn cancel(&self) {
        self.cancelled.store(true, Ordering::Relaxed);
    }

    /// A job that merely claims to run: the menu bar's ● marker tests only need the flag.
    #[cfg(test)]
    pub(crate) fn fake_running() -> Self {
        Self::new(CompilePlan {
            manifest: ProjectManifest::default(),
            manifest_path: std::path::PathBuf::new(),
            output_document: std::path::PathBuf::new(),
            output_pdf: None,
            chapters: Vec::new(),
            warnings: Vec::new(),
        })
    }

    pub(crate) fn progress(&self) -> (usize, usize) {
        (self.next.min(self.steps.len()), self.steps.len())
    }

    /// The step about to run, for the progress line.
    pub(crate) fn current_step(&self) -> Option<String> {
        self.steps.get(self.next).map(|s| s.label(&self.plan))
    }

    pub(crate) fn report(&self) -> &CompileReport {
        &self.report
    }

    /// Run one step. Errors name the step and (for a chapter) the workbook.
    pub(crate) fn advance(&mut self, app: &mut DesignApp) -> StepOutcome {
        if self.state != JobState::Running {
            return match &self.state {
                JobState::Failed(e) => StepOutcome::Failed(e.clone()),
                JobState::Cancelled => StepOutcome::Cancelled,
                _ => StepOutcome::Done,
            };
        }
        if self.cancelled.load(Ordering::Relaxed) {
            self.state = JobState::Cancelled;
            return StepOutcome::Cancelled;
        }
        if let Some(uid) = self.doc_uid
            && app.session.active().map(|d| d.uid) != Some(uid)
        {
            let message = "the compiled document is no longer active; switch back to it and compile again".to_string();
            self.state = JobState::Failed(message.clone());
            return StepOutcome::Failed(message);
        }
        let Some(step) = self.steps.get(self.next).cloned() else {
            self.finish(app);
            return StepOutcome::Done;
        };
        match self.run_step(app, &step) {
            Ok(()) => {
                if matches!(step, CompileStep::NewDocument) {
                    self.doc_uid = app.session.active().map(|d| d.uid);
                }
                self.next = self.next.saturating_add(1);
                if self.next >= self.steps.len() {
                    self.finish(app);
                    StepOutcome::Done
                } else {
                    StepOutcome::Pending
                }
            }
            Err(e) => {
                let message = format!("{}: {e}", step.label(&self.plan));
                self.state = JobState::Failed(message.clone());
                StepOutcome::Failed(message)
            }
        }
    }

    /// Run every step, for the command palette, the control channel and tests.
    pub(crate) fn run_all(&mut self, app: &mut DesignApp) -> Result<CompileReport, String> {
        loop {
            match self.advance(app) {
                StepOutcome::Pending => {}
                StepOutcome::Done => return Ok(self.report.clone()),
                StepOutcome::Cancelled => return Err("compile cancelled".into()),
                StepOutcome::Failed(e) => return Err(e),
            }
        }
    }

    fn finish(&mut self, app: &mut DesignApp) {
        self.state = JobState::Done;
        self.report.document = self.plan.output_document.display().to_string();
        self.report.pdf = self.plan.output_pdf.as_ref().map(|p| p.display().to_string());
        self.report.pages = page_count(app).unwrap_or(0);
        self.report.chapters = std::mem::take(&mut self.chapter_stats);
        self.report.warnings = std::mem::take(&mut self.warnings);
        self.report.elapsed_ms = self.started.elapsed().as_millis().min(u128::from(u64::MAX)) as u64;
    }

    fn run_step(&mut self, app: &mut DesignApp, step: &CompileStep) -> Result<(), String> {
        match step {
            CompileStep::NewDocument => self.new_document(app),
            CompileStep::MasterSetup => self.master_setup(app),
            CompileStep::FrontCover => self.front_cover(app),
            CompileStep::InsideCover => execute_generate_inside_cover(app),
            CompileStep::NormalizeSeedPage => self.normalize_seed_page(app),
            CompileStep::FrontMatter => self.front_matter(app),
            CompileStep::TocPlaceholders => self.toc_placeholders(app),
            CompileStep::Chapter(i) => self.chapter(app, *i),
            CompileStep::BackCover => self.back_cover(app),
            CompileStep::TocFinalize => self.toc_finalize(app),
            CompileStep::Sections => self.sections(app),
            CompileStep::RunningHeaders => self.running_headers(app),
            CompileStep::SaveDocument => {
                let path = self.plan.output_document.to_string_lossy().to_string();
                app.run("file.saveAs", serde_json::json!({ "path": path })).map_err(|e| format!("{}: {e}", self.plan.output_document.display()))?;
                Ok(())
            }
            CompileStep::ExportPdf => {
                let Some(path) = &self.plan.output_pdf else { return Ok(()) };
                let path = path.to_string_lossy().to_string();
                app.run("file.exportPdf", serde_json::json!({ "path": path })).map_err(|e| format!("{}: {e}", path))?;
                Ok(())
            }
        }
    }

    fn new_document(&mut self, app: &mut DesignApp) -> Result<(), String> {
        let doc = &self.plan.manifest.document;
        let title = if self.plan.manifest.publication_title.trim().is_empty() {
            self.plan.manifest_path.file_stem().and_then(std::ffi::OsStr::to_str).unwrap_or("Publication").to_string()
        } else {
            self.plan.manifest.publication_title.trim().to_string()
        };
        app.run(
            "file.new",
            serde_json::json!({
                "preset": doc.preset,
                "facingPages": doc.facing_pages,
                "pages": 1,
                "title": title,
            }),
        )?;
        let m = &doc.margins_mm;
        app.run(
            "layout.documentSetup",
            serde_json::json!({
                "margins": { "top": m.top, "bottom": m.bottom, "inside": m.inside, "outside": m.outside },
                "bleed": doc.bleed_mm.max(0.0),
            }),
        )?;
        Ok(())
    }

    fn master_setup(&mut self, app: &mut DesignApp) -> Result<(), String> {
        app.run("layout.parents.new", serde_json::json!({ "prefix": "A", "name": "Body" }))?;
        Ok(())
    }

    fn front_cover(&mut self, app: &mut DesignApp) -> Result<(), String> {
        if let Some(source) = self.cover_workbook()? {
            match execute_convert_excel(&source.to_string_lossy()) {
                Ok(pkg) => match pkg.cover {
                    Some(cover) => apply_excel_cover(&mut self.base, &cover, source.parent()),
                    None => self.warnings.push(format!("{}: no COVER sheet, using the cover defaults", source.display())),
                },
                Err(e) => self.warnings.push(format!("{}: cover values not read: {e}", source.display())),
            }
        } else {
            self.warnings.push("no cover workbook with a COVER sheet; using the cover defaults".into());
        }
        execute_generate_front_cover(app, &self.base)?;
        Ok(())
    }

    /// The manifest's cover workbook, else the first chapter workbook carrying a COVER sheet.
    fn cover_workbook(&self) -> Result<Option<PathBuf>, String> {
        if !self.plan.manifest.cover.source.trim().is_empty() {
            let path = self.plan.resolve(&self.plan.manifest.cover.source);
            if !path.is_file() {
                return Err(format!("cover.source: {} is not a file", path.display()));
            }
            return Ok(Some(path));
        }
        for chapter in &self.plan.chapters {
            let ChapterSource::Workbook(path) = &chapter.source else { continue };
            if workbook_has_cover_sheet(path) {
                return Ok(Some(path.clone()));
            }
        }
        Ok(None)
    }

    fn normalize_seed_page(&mut self, app: &mut DesignApp) -> Result<(), String> {
        let candidate = if self.plan.manifest.cover.include_inside { 2 } else { 1 };
        let (total, empty) = {
            let active = app.session.active().ok_or("no active document")?;
            (active.doc.page_count(), page_is_empty(&active.doc, candidate))
        };
        if candidate >= total {
            return Ok(());
        }
        if empty {
            app.run("layout.pages.delete", serde_json::json!({ "pages": [candidate] }))?;
        } else {
            self.warnings.push(format!("page {} kept: the seeded page is not empty", candidate + 1));
        }
        Ok(())
    }

    fn front_matter(&mut self, app: &mut DesignApp) -> Result<(), String> {
        if self.plan.has_front_matter() {
            let path = self.plan.resolve(self.plan.manifest.front_matter.workbook.trim());
            if !path.is_file() {
                return Err(format!("front_matter.workbook: {} is not a file", path.display()));
            }
            let plan = ChapterPlan { chapter: 0, title: "Front matter".into(), source: ChapterSource::Workbook(path) };
            let prepared = self.prepare(&plan)?;
            let len = prepared.state.card1_records.len();
            if len == 0 {
                self.warnings.push("front_matter.workbook: the catalog has no rows".into());
                return Ok(());
            }
            let pages = self.emit(app, &prepared.state, 1, len)?;
            if pages == 0 {
                self.warnings.push("front_matter.workbook: no pages were generated".into());
            }
            return Ok(());
        }
        let Some(first) = self.plan.chapters.first().cloned() else { return Ok(()) };
        let prepared = self.prepare(&first)?;
        let rows = prepared.narrative_prefix;
        self.front_matter_rows = rows;
        if rows == 0 {
            self.warnings.push(format!("{}: no leading narrative rows, so the lists follow the inside cover", first.label()));
        } else {
            self.emit(app, &prepared.state, 1, rows)?;
        }
        if let Some(slot) = self.prepared.first_mut() {
            *slot = Some(prepared);
        }
        Ok(())
    }

    fn toc_placeholders(&mut self, app: &mut DesignApp) -> Result<(), String> {
        let lists = &self.plan.manifest.toc.lists;
        if lists.is_empty() {
            return Ok(());
        }
        let total = page_count(app)?;
        app.run("layout.pages.insert", serde_json::json!({ "after": total.saturating_sub(1), "count": lists.len() }))?;
        self.toc_pages = lists.iter().enumerate().map(|(i, list)| (list.clone(), total + i)).collect();
        Ok(())
    }

    fn chapter(&mut self, app: &mut DesignApp, index: usize) -> Result<(), String> {
        let Some(plan) = self.plan.chapters.get(index).cloned() else {
            return Err(format!("chapter {index} is not planned"));
        };
        let prepared = match self.prepared.get_mut(index).and_then(Option::take) {
            Some(prepared) => prepared,
            None => self.prepare(&plan)?,
        };
        let len = prepared.state.card1_records.len();
        let from = if index == 0 && self.front_matter_rows > 0 { self.front_matter_rows.saturating_add(1) } else { 1 };
        let start = page_count(app)?;
        let (pages, tables, charts) = if from > len {
            self.warnings.push(format!("{}: no body rows after the front matter", plan.label()));
            (0, 0, 0)
        } else {
            let (tables, charts) = count_kinds(&prepared.state, from, len);
            (self.emit(app, &prepared.state, from, len)?, tables, charts)
        };
        if pages > 0 {
            self.chapter_ranges.push(start);
            if self.body_start.is_none() {
                self.body_start = Some(start);
            }
        }
        self.chapter_stats.push(ChapterReport {
            chapter: plan.chapter,
            workbook: plan.source.workbook().display().to_string(),
            pages,
            tables,
            charts,
        });
        Ok(())
    }

    fn back_cover(&mut self, app: &mut DesignApp) -> Result<(), String> {
        let total = page_count(app)?;
        execute_generate_back_cover(app, &self.base)?;
        if page_count(app)? > total {
            self.back_cover = Some(page_count(app)?.saturating_sub(1));
        } else {
            self.warnings.push("the back cover was not added".into());
        }
        Ok(())
    }

    fn toc_finalize(&mut self, app: &mut DesignApp) -> Result<(), String> {
        if self.toc_pages.is_empty() {
            return Ok(());
        }
        let mut added = 0usize;
        for pass in 0..2 {
            let mut pass_added = 0usize;
            for (list, page) in self.toc_pages.clone() {
                let st = self.toc_state(&list);
                let plan = TocPlan::from_state(&st);
                let (_entries, pages_added) = generate_toc(app, &st, page.saturating_add(pass_added), &plan, true)?;
                pass_added = pass_added.saturating_add(pages_added as usize);
            }
            added = added.saturating_add(pass_added);
            if pass_added == 0 {
                break;
            }
            if pass == 1 {
                self.warnings.push(format!("the lists still grew by {pass_added} page(s) on the second pass"));
            }
            self.refresh_toc_pages(app);
        }
        // Lists before the body (the default placement) move the body and the back cover; lists at
        // the end move only the back cover.
        let before_body = self.body_start.is_some_and(|b| self.toc_pages.first().is_some_and(|(_, p)| *p < b));
        if before_body && let Some(start) = self.body_start {
            self.body_start = Some(start.saturating_add(added));
        }
        if let Some(back) = self.back_cover {
            self.back_cover = Some(back.saturating_add(added));
        }
        self.refresh_toc_pages(app);
        Ok(())
    }

    /// Look each list up by title: generating one list can move the next.
    fn refresh_toc_pages(&mut self, app: &mut DesignApp) {
        let Some(active) = app.session.active() else { return };
        let mut found = Vec::with_capacity(self.toc_pages.len());
        for (list, page) in &self.toc_pages {
            let title = list_title(list, self.toc_title_bilingual());
            let located = active.doc.tocs.iter().find(|t| t.title.trim() == title.trim()).and_then(|t| toc_story_page(app, t.story));
            found.push((list.clone(), located.unwrap_or(*page)));
        }
        self.toc_pages = found;
    }

    fn toc_title_bilingual(&self) -> bool {
        self.plan.manifest.toc.title_bilingual && self.plan.manifest.bilingual
    }

    fn toc_state(&self, list: &str) -> PubgenState {
        let mut st = self.base.clone();
        st.toc_kind = match list {
            "tables" => 1,
            "charts" => 2,
            _ => 3,
        };
        st.toc_title = list_title(list, self.toc_title_bilingual());
        st.toc_include_self = false;
        st.toc_hanging_indent = self.plan.manifest.toc.hanging_indent;
        st.toc_pair_bilingual = self.plan.manifest.bilingual;
        st.bilingual_output = self.plan.manifest.bilingual;
        st.card1_records.clear();
        st
    }

    fn sections(&mut self, app: &mut DesignApp) -> Result<(), String> {
        let p = &self.plan.manifest.pagination;
        app.run("layout.section", serde_json::json!({ "page": 1, "startNumber": 1, "style": p.front_matter_format }))?;
        let Some(body) = self.body_start else { return Ok(()) };
        let mut params = serde_json::json!({
            "page": body.saturating_add(1),
            "style": p.body_format,
        });
        if p.restart_at_body {
            params["startNumber"] = serde_json::json!(1);
        }
        app.run("layout.section", params)?;
        Ok(())
    }

    fn running_headers(&mut self, app: &mut DesignApp) -> Result<(), String> {
        const STYLE: &str = "Running Header";
        let has_style = app.session.active().is_some_and(|a| a.doc.styles.char_style(STYLE).is_some());
        if !has_style {
            app.run("style.character.create", serde_json::json!({ "name": STYLE }))?;
        }
        app.run(
            "variables.define",
            serde_json::json!({
                "name": STYLE,
                "type": "runningHeader",
                "style": STYLE,
                "character": true,
                "use": "lastOnPage",
            }),
        )?;
        for page in &self.chapter_ranges {
            if !tag_chapter_title(app, *page, STYLE)? {
                self.warnings.push(format!("page {}: no narrative chapter title to use as a running header", page + 1));
            }
        }
        let scope = match self.plan.header_scope() {
            HeaderScope::All => serde_json::json!("all"),
            HeaderScope::Body => {
                let Some(first) = self.body_start else { return Ok(()) };
                let last = self.back_cover.unwrap_or_else(|| page_count(app).unwrap_or(first + 1));
                serde_json::json!({ "range": [first + 1, last.max(first + 1)] })
            }
        };
        app.run("layout.headerFooter.apply", serde_json::json!({ "preset": self.plan.manifest.header_footer.preset, "scope": scope }))?;
        Ok(())
    }

    /// Convert a chapter's workbook (or use its catalog) into a scratch panel state, so a failed
    /// chapter never touches the user's Card 1/2 settings.
    fn prepare(&mut self, chapter: &ChapterPlan) -> Result<PreparedChapter, String> {
        let mut st = self.chapter_profile();
        match &chapter.source {
            ChapterSource::Workbook(path) => {
                let pkg = execute_convert_excel(&path.to_string_lossy())?;
                load_card1_csv(&mut st, &pkg.catalog_path);
                st.card2_sheet_tables = pkg.table_paths;
            }
            ChapterSource::Catalog { catalog, tables } => {
                load_card1_csv(&mut st, &catalog.to_string_lossy());
                st.card2_sheet_tables = tables.iter().map(|p| p.to_string_lossy().to_string()).collect();
            }
        }
        if st.card1_records.is_empty() {
            return Err(format!("{}: the catalog has no rows", chapter.label()));
        }
        let narrative_prefix = leading_narrative_rows(&st);
        Ok(PreparedChapter { state: st, narrative_prefix })
    }

    fn chapter_profile(&self) -> PubgenState {
        let mut st = self.base.clone();
        st.card1_path.clear();
        st.card1_records.clear();
        st.card1_headers.clear();
        st.card1_status = None;
        st.card2_path.clear();
        st.card2_matrix.clear();
        st.card2_sheet_tables.clear();
        st.card2_status = None;
        st.excel_batch_covers.clear();
        st.excel_had_cover = false;
        st.translate_with_ai = false;
        st.card1_preset = 0;
        st.card1_append = true;
        st.card1_range_all = true;
        st.card1_auto_insert_tables = true;
        st.card2_first_row_header = true;
        st.card2_col_numbers = true;
        st.card2_table_footer = true;
        st.card2_row_conditions = true;
        st
    }

    fn emit(&mut self, app: &mut DesignApp, st: &PubgenState, from: usize, to: usize) -> Result<usize, String> {
        let mut st = st.clone();
        st.card1_range_all = from == 1 && to == st.card1_records.len();
        st.card1_range_from = from;
        st.card1_range_to = to;
        let before = page_count(app)?;
        execute_generate_pages(app, &st)?;
        Ok(page_count(app)?.saturating_sub(before))
    }
}

fn page_count(app: &DesignApp) -> Result<usize, String> {
    app.session.active().map(|a| a.doc.page_count()).ok_or_else(|| "no active document".to_string())
}

fn list_title(list: &str, bilingual: bool) -> String {
    let kind = match list {
        "tables" => 1,
        "charts" => 2,
        _ => 3,
    };
    toc_kind_title(kind, bilingual).unwrap_or("Contents").to_string()
}

fn workbook_has_cover_sheet(path: &Path) -> bool {
    super::excel::parse_xlsx_sheets(&path.to_string_lossy()).is_ok_and(|sheets| sheets.iter().any(|s| s.name.trim().eq_ignore_ascii_case("COVER")))
}

fn leading_narrative_rows(st: &PubgenState) -> usize {
    let mut rows = 0usize;
    for row in &st.card1_records {
        let kind = row.get(st.card1_type_col.min(2)).map(String::as_str).unwrap_or("").trim();
        if is_divider_type(kind) {
            break;
        }
        let narrative = kind.eq_ignore_ascii_case("G") || kind.eq_ignore_ascii_case("Note") || is_ulasan_type(kind);
        if !narrative {
            break;
        }
        rows = rows.saturating_add(1);
    }
    rows
}

fn count_kinds(st: &PubgenState, from: usize, to: usize) -> (usize, usize) {
    let start = from.saturating_sub(1);
    let mut tables = 0usize;
    let mut charts = 0usize;
    for row in st.card1_records.get(start..to).unwrap_or_default() {
        let kind = row.get(st.card1_type_col.min(2)).map(String::as_str).unwrap_or("").trim();
        if is_chart_type(kind) {
            charts = charts.saturating_add(1);
        } else if !is_divider_type(kind) && !is_ulasan_type(kind) && !kind.eq_ignore_ascii_case("G") && !kind.eq_ignore_ascii_case("Note") {
            tables = tables.saturating_add(1);
        }
    }
    (tables, charts)
}

fn page_is_empty(doc: &designcraft_doc::Document, page: usize) -> bool {
    let Some((si, pi)) = doc.page_loc(page) else { return false };
    let Some(spread) = doc.spreads.get(si) else { return false };
    if spread.pages.len() == 1 {
        return spread.items.is_empty();
    }
    !spread.items.iter().any(|it| spread.page_at_x(it.bounds().center().x) == Some(pi))
}

/// Apply the running-header character style to the divider page's chapter title. The paragraph
/// style stays whatever it was (`TOC Chapter` or `TOC Entry`), so the contents list keeps naming
/// exactly the pages it named before.
fn tag_chapter_title(app: &mut DesignApp, page: usize, style: &str) -> Result<bool, String> {
    let target = {
        let Some(active) = app.session.active() else { return Ok(false) };
        let doc = &active.doc;
        let on_page = |story: &designcraft_doc::Story| story.frames.iter().any(|f| doc.page_of_item(*f) == Some(page));
        let labelled = doc.spreads.iter().enumerate().find_map(|(si, spread)| {
            spread.items.iter().find_map(|item| {
                if !on_page_item(doc, si, spread, item, page) {
                    return None;
                }
                let label = format!("{} {}", item.label.to_lowercase(), item.name.to_lowercase());
                if !label.contains("title") || label.contains("title_en") {
                    return None;
                }
                let tf = item.text_frame()?;
                let story = doc.story(tf.story)?;
                (!story.text.trim().is_empty()).then_some((tf.story, story.len()))
            })
        });
        labelled.or_else(|| {
            doc.stories.iter().find_map(|(sid, story)| {
                let first = story.paras.iter().find(|p| !p.style.trim().is_empty())?;
                (first.style == TOC_NARRATIVE_STYLE && !story.text.trim().is_empty() && on_page(story)).then_some((*sid, story.len()))
            })
        })
    };
    let Some((sid, len)) = target else { return Ok(false) };
    if let Some(active) = app.session.active_mut() {
        active.selection = Selection::text(TextSel { story: sid, anchor: 0, focus: len, frame: None, cell: None });
    }
    app.run("style.character.apply", serde_json::json!({ "name": style }))?;
    if let Some(active) = app.session.active_mut() {
        active.selection = Selection::default();
    }
    Ok(true)
}

fn on_page_item(
    doc: &designcraft_doc::Document,
    spread_index: usize,
    spread: &designcraft_doc::Spread,
    item: &designcraft_doc::Item,
    page: usize,
) -> bool {
    let Some((si, pi)) = doc.page_loc(page) else { return false };
    si == spread_index && (spread.pages.len() == 1 || spread.page_at_x(item.bounds().center().x) == Some(pi))
}

/// The `pubgen.compileProject` UI command: the command palette, the control channel and scripts.
/// `sync` defaults to true; `sync: false` hands the job to the Pubgen panel, which runs one step
/// per frame. Headless runs (no egui context) always run synchronously.
pub(crate) fn compile_project(app: &mut DesignApp, p: &Value) -> Result<Value, String> {
    let path = p.get("path").and_then(Value::as_str).map(str::trim).filter(|s| !s.is_empty()).map(str::to_string).or_else(|| {
        let remembered = app.session.prefs.pubgen_project_path.trim();
        (!remembered.is_empty()).then(|| remembered.to_string())
    });
    let Some(path) = path else {
        if let Some(ctx) = app.egui_ctx.clone() {
            super::with_state(&ctx, |st| {
                st.dialog_excel_open = true;
            });
        }
        return Ok(serde_json::json!({"dialog": "pubgen.compileProject", "open": true}));
    };
    if path.chars().count() > MAX_PATH_CHARS {
        return Err("path: too long".into());
    }
    let mut plan = CompilePlan::load(Path::new(&path))?;
    if let Some(pdf) = p.get("pdf").and_then(Value::as_str).map(str::trim).filter(|s| !s.is_empty()) {
        plan.output_pdf = Some(pdf_override(&plan, pdf)?);
    }
    ensure_output_not_open(app, &plan)?;
    remember_manifest(app, &path)?;
    let sync = p.get("sync").and_then(Value::as_bool).unwrap_or(true);
    if sync || app.egui_ctx.is_none() {
        let mut job = CompileJob::new(plan);
        return job.run_all(app).map(|report| report_json(&report));
    }
    let state_id = egui::Id::unique(super::PANEL_ID);
    let mut st = app.egui_ctx.as_ref().and_then(|ctx| ctx.data(|d| d.get_temp::<PubgenState>(state_id))).unwrap_or_else(|| PubgenState {
        bilingual_output: true,
        cover_include_inside_back: true,
        ..Default::default()
    });
    if st.compile_job.is_some() {
        return Err("a publication compile is already running".into());
    }
    st.compile_manifest_path = path.clone();
    st.compile_checked = path.clone();
    st.compile_summary = plan.summary_line();
    st.compile_status = Some(("Compiling…".into(), true));
    st.compile_job = Some(Box::new(CompileJob::new(plan)));
    if let Some(ctx) = &app.egui_ctx {
        ctx.data_mut(|d| d.insert_temp(state_id, st));
        ctx.request_repaint();
    }
    Ok(serde_json::json!({"started": true, "path": path}))
}

/// Advance a frame-stepped compile by one step. The panel calls this once per frame.
pub(crate) fn poll_compile_job(app: &mut DesignApp, st: &mut PubgenState) {
    let Some(mut job) = st.compile_job.take() else { return };
    match job.advance(app) {
        StepOutcome::Pending => {
            let (done, total) = job.progress();
            let step = job.current_step().unwrap_or_default();
            st.compile_status = Some((format!("Compiling step {done} of {total}: {step}"), true));
            st.compile_job = Some(job);
            if let Some(ctx) = &app.egui_ctx {
                ctx.request_repaint();
            }
        }
        StepOutcome::Done => st.compile_status = Some((format_report(job.report()), true)),
        StepOutcome::Cancelled => {
            st.compile_status = Some(("Compile cancelled — the document is open and unsaved; the previous build is unchanged.".into(), false))
        }
        StepOutcome::Failed(e) => {
            st.compile_status = Some((format!("Compile failed — {e}. The document is open and unsaved; the previous build is unchanged."), false))
        }
    }
}

pub(crate) const COVER_STYLES: &[&str] = &[
    "Wave Curves",
    "Geometric & Medallion",
    "Folded Origami",
    "Chevron & Medallion",
    "Honeycomb Hexagon",
    "Corporate Headline",
    "Diagonal Arrow",
    "Modern Horizon",
    "Business Report",
    "Urban Angle",
    "Tri-Color Bands",
    "Prism Diamonds",
    "Quarter Stack",
    "BPS Berita Resmi Statistik (BRS)",
];

pub(crate) const COVER_THEMES: &[&str] = &[
    "Classic BPS (Blue/Gold)",
    "Emerald Growth (Green)",
    "Sunset Crimson (Red)",
    "Corporate Navy (Deep Blue)",
    "Modern Slate (Gray)",
    "Business Report (Report Blue)",
    "Urban Dark (Charcoal/Gold)",
];

fn cover_style_label(style: usize) -> String {
    if let Some(name) = super::ai_cover::style_name(style) {
        format!("AI {name} (online)")
    } else {
        COVER_STYLES.get(style).copied().unwrap_or("Wave Curves").to_string()
    }
}

fn deduce_chapter_title(stem: &str, num: u32) -> String {
    let lower = stem.to_lowercase();
    let bps_standard = [
        "Geografi",
        "Iklim",
        "Pemerintahan",
        "Kependudukan",
        "Sosial",
        "Pertanian",
        "Industri & Energi",
        "Pariwisata",
        "Transportasi & Komunikasi",
        "Keuangan",
        "Pengeluaran Penduduk",
        "Perdagangan",
    ];
    if (1..=12).contains(&num) {
        let idx = (num - 1) as usize;
        if (lower.contains(&format!("bab_0{num}")) || lower.contains(&format!("bab_{num}")) || lower.contains(&format!("bab {num}")))
            && let Some(t) = bps_standard.get(idx)
        {
            return t.to_string();
        }
    }
    stem.replace(['_', '-'], " ").trim().to_string()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum NewProjectTab {
    Files,
    Cover,
    TocAndLayout,
}

#[derive(Clone, Debug)]
pub(crate) struct NewProjectDialog {
    pub tab: NewProjectTab,
    pub manifest_save_path: String,
    pub publication_title: String,
    pub bilingual: bool,
    pub output_document: String,
    pub export_pdf: bool,
    pub output_pdf: String,
    pub chapters: Vec<ChapterSpec>,
    pub new_chapter_num: u32,
    pub new_chapter_title: String,
    pub new_chapter_path: String,
    pub front_matter_workbook: String,
    pub cover_source: String,
    pub cover_style: usize,
    pub cover_theme_preset: usize,
    pub cover_include_inside: bool,
    pub cover_include_back: bool,
    pub toc_placement: String,
    pub toc_contents: bool,
    pub toc_tables: bool,
    pub toc_charts: bool,
    pub toc_title_bilingual: bool,
    pub toc_hanging_indent: bool,
    pub pagination_front_format: String,
    pub pagination_body_format: String,
    pub pagination_restart_body: bool,
    pub header_footer_preset: String,
    pub error_message: Option<String>,
}

impl Default for NewProjectDialog {
    fn default() -> Self {
        Self {
            tab: NewProjectTab::Files,
            manifest_save_path: String::new(),
            publication_title: String::new(),
            bilingual: true,
            output_document: String::new(),
            export_pdf: true,
            output_pdf: String::new(),
            chapters: Vec::new(),
            new_chapter_num: 1,
            new_chapter_title: String::new(),
            new_chapter_path: String::new(),
            front_matter_workbook: String::new(),
            cover_source: String::new(),
            cover_style: 11,       // Prism Diamonds
            cover_theme_preset: 0, // Classic BPS
            cover_include_inside: true,
            cover_include_back: true,
            toc_placement: "afterFrontMatter".into(),
            toc_contents: true,
            toc_tables: true,
            toc_charts: true,
            toc_title_bilingual: true,
            toc_hanging_indent: true,
            pagination_front_format: "lowerRoman".into(),
            pagination_body_format: "arabic".into(),
            pagination_restart_body: true,
            header_footer_preset: "book".into(),
            error_message: None,
        }
    }
}

impl NewProjectDialog {
    pub(crate) fn build_manifest(&self) -> Result<(PathBuf, ProjectManifest), String> {
        let manifest_path_str = self.manifest_save_path.trim();
        if manifest_path_str.is_empty() {
            return Err("Please specify a path to save pubgen_project.json".into());
        }
        let save_path = PathBuf::from(manifest_path_str);
        if save_path.extension().and_then(std::ffi::OsStr::to_str).map(|e| e.to_ascii_lowercase()) != Some("json".into()) {
            return Err(format!("{}: manifest file must have a .json extension", save_path.display()));
        }
        if self.chapters.is_empty() {
            return Err("Please add at least one chapter workbook (.xlsx)".into());
        }
        for (i, ch) in self.chapters.iter().enumerate() {
            if ch.path.trim().is_empty() {
                return Err(format!("Chapter {} has an empty path", i + 1));
            }
        }

        let manifest_dir = save_path.parent().filter(|p| !p.as_os_str().is_empty()).unwrap_or_else(|| Path::new("."));

        let rel = |raw: &str| -> String {
            let s = raw.trim();
            if s.is_empty() {
                return String::new();
            }
            let target = Path::new(s);
            if let Ok(r) = target.strip_prefix(manifest_dir) { r.to_string_lossy().replace('\\', "/") } else { s.replace('\\', "/") }
        };

        let title = self.publication_title.trim().to_string();

        let output_doc = if self.output_document.trim().is_empty() { String::new() } else { rel(&self.output_document) };

        let output_pdf = if !self.export_pdf || self.output_pdf.trim().is_empty() { String::new() } else { rel(&self.output_pdf) };

        let chapters =
            self.chapters.iter().map(|c| ChapterSpec { chapter: c.chapter, path: rel(&c.path), title: c.title.trim().to_string() }).collect();

        let mut lists = Vec::new();
        if self.toc_contents {
            lists.push("contents".into());
        }
        if self.toc_tables {
            lists.push("tables".into());
        }
        if self.toc_charts {
            lists.push("charts".into());
        }

        let manifest = ProjectManifest {
            version: 1,
            publication_title: title,
            bilingual: self.bilingual,
            profile: None,
            output: OutputSpec { document: output_doc, pdf: output_pdf },
            document: DocumentSpec::default(),
            cover: CoverSpec {
                source: rel(&self.cover_source),
                style: self.cover_style,
                theme_preset: self.cover_theme_preset,
                include_inside: self.cover_include_inside,
                include_back: self.cover_include_back,
            },
            front_matter: FrontMatterSpec { workbook: rel(&self.front_matter_workbook) },
            chapters,
            pagination: PaginationSpec {
                front_matter_format: self.pagination_front_format.clone(),
                body_format: self.pagination_body_format.clone(),
                restart_at_body: self.pagination_restart_body,
            },
            toc: TocSpec {
                placement: self.toc_placement.clone(),
                lists,
                title_bilingual: self.toc_title_bilingual,
                hanging_indent: self.toc_hanging_indent,
            },
            header_footer: HeaderFooterSpec { preset: self.header_footer_preset.clone(), scope: "body".into() },
            build: BuildSpec { keep_intermediate: true, export_pdf: self.export_pdf },
        };

        Ok((save_path, manifest))
    }

    pub(crate) fn save_manifest(&self) -> Result<PathBuf, String> {
        let (save_path, manifest) = self.build_manifest()?;
        if let Some(parent) = save_path.parent() {
            if !parent.as_os_str().is_empty() && !parent.exists() {
                std::fs::create_dir_all(parent).map_err(|e| format!("create folder {}: {e}", parent.display()))?;
            }
            if !manifest.output.document.is_empty() {
                let doc_path = resolve_against(parent, &manifest.output.document);
                if let Some(doc_parent) = doc_path.parent()
                    && !doc_parent.as_os_str().is_empty()
                    && !doc_parent.exists()
                {
                    let _ = std::fs::create_dir_all(doc_parent);
                }
            }
            if !manifest.output.pdf.is_empty() {
                let pdf_path = resolve_against(parent, &manifest.output.pdf);
                if let Some(pdf_parent) = pdf_path.parent()
                    && !pdf_parent.as_os_str().is_empty()
                    && !pdf_parent.exists()
                {
                    let _ = std::fs::create_dir_all(pdf_parent);
                }
            }
        }
        let json_str = serde_json::to_string_pretty(&manifest).map_err(|e| format!("serialize manifest: {e}"))?;
        std::fs::write(&save_path, json_str).map_err(|e| format!("write {}: {e}", save_path.display()))?;
        Ok(save_path)
    }
}

/// The Publication card at the top of the Pubgen panel.
pub(crate) fn compile_card_ui(app: &mut DesignApp, ui: &mut Ui, st: &mut PubgenState, t: &Tokens) {
    let lang = app.ui.language.clone();
    section(ui, crate::i18n::tr(&lang, "Publication Compiler"), true);

    let remembered = app.session.prefs.pubgen_project_path.trim().to_string();
    ui.horizontal(|ui| {
        let label = ui.label(RichText::new(crate::i18n::tr(&lang, "Manifest:")).size(11.0).color(t.text));
        let width = (ui.available_width() - 110.0).clamp(60.0, 200.0);
        let mut edited = st.compile_manifest_path.clone();
        let resp = ui
            .add_sized(
                [width, FIELD_H],
                egui::TextEdit::singleline(&mut edited).hint_text(if remembered.is_empty() { "pubgen_project.json" } else { &remembered }),
            )
            .labelled_by(label.id);
        if resp.changed() {
            st.compile_manifest_path = edited;
            st.compile_checked.clear();
            st.compile_summary.clear();
        }
        if outline_button(ui, crate::i18n::tr(&lang, "Open…"), 50.0).clicked() {
            let ctx = ui.ctx().clone();
            let _ = app.ask_open("pubgenProject", crate::app::file_dialogs::Bound::App, move |_app, path| {
                super::with_state(&ctx, |st| {
                    st.compile_manifest_path = path;
                    st.compile_checked.clear();
                    st.compile_summary.clear();
                });
                Ok(serde_json::Value::Null)
            });
        }
        if outline_button(ui, crate::i18n::tr(&lang, "New…"), 46.0).clicked() {
            st.compile_new_dialog = Some(Box::new(NewProjectDialog::default()));
        }
    });

    let path = if st.compile_manifest_path.trim().is_empty() { remembered } else { st.compile_manifest_path.trim().to_string() };
    if !path.is_empty() && st.compile_checked != path {
        st.compile_summary = match CompilePlan::load(Path::new(&path)) {
            Ok(plan) => plan.summary_line(),
            Err(e) => e,
        };
        st.compile_checked = path.clone();
    }
    if !st.compile_summary.is_empty() {
        ui.add(egui::Label::new(RichText::new(&st.compile_summary).size(10.0).color(t.text_dim)).wrap());
    }

    if path.is_empty() {
        ui.add_space(2.0);
        ui.vertical_centered(|ui| {
            if ui
                .button(RichText::new(crate::i18n::tr(&lang, "Create New Project Manifest…")).size(11.0).color(t.accent))
                .on_hover_text(crate::i18n::tr(&lang, "Open the project builder form to create a new pubgen_project.json"))
                .clicked()
            {
                st.compile_new_dialog = Some(Box::new(NewProjectDialog::default()));
            }
        });
        ui.add_space(2.0);
    }

    let running = st.compile_job.is_some();
    ui.horizontal(|ui| {
        let compile = ui.add_enabled(!running && !path.is_empty(), egui::Button::new(crate::i18n::tr(&lang, "Compile")));
        if compile.clicked() {
            start_compile(app, &path, st);
        }
        if running
            && ui.button(crate::i18n::tr(&lang, "Cancel")).clicked()
            && let Some(job) = &st.compile_job
        {
            job.cancel();
        }
    });

    if let Some(job) = &st.compile_job {
        let (done, total) = job.progress();
        let fraction = if total == 0 { 0.0 } else { done as f32 / total as f32 };
        ui.add(egui::ProgressBar::new(fraction).text(format!("{done} / {total}")));
        if let Some(step) = job.current_step() {
            ui.add(egui::Label::new(RichText::new(step).size(10.0).color(t.text_dim)).wrap());
        }
    }

    if let Some((msg, ok)) = &st.compile_status {
        let color = if *ok { egui::Color32::from_rgb(80, 200, 120) } else { egui::Color32::from_rgb(220, 80, 80) };
        ui.add(egui::Label::new(RichText::new(msg).size(10.5).color(color)).wrap());
    }

    // Shared Cloudflare free-tier remaining (from the Supabase tracker) — shown in
    // the compile/download card as well, per the quota-notification requirement.
    if let Some(n) = st.cover_quota_covers {
        ui.add(egui::Label::new(RichText::new(format!("Cloudflare AI: ~{n} covers left today")).size(10.0).color(t.text_dim)));
    }

    // Modal dialog to create a new pubgen_project.json
    compile_new_project_dialog_ui(app, ui, st, t);
}

fn render_new_project_files_tab(app: &mut DesignApp, ui: &mut Ui, dialog: &mut NewProjectDialog, t: &Tokens, lang: &str) {
    ui.label(RichText::new(crate::i18n::tr(lang, "Publication Details:")).strong().size(12.0).color(t.text));
    ui.horizontal(|ui| {
        ui.label(RichText::new("Title:").size(11.0).color(t.text));
        let w = (ui.available_width() - 10.0).max(180.0);
        let resp = ui
            .add_sized([w, FIELD_H], egui::TextEdit::singleline(&mut dialog.publication_title).hint_text("e.g. Kabupaten Karawang Dalam Angka 2026"));
        if resp.changed() && dialog.output_document.is_empty() && !dialog.publication_title.trim().is_empty() {
            let stem: String = dialog.publication_title.chars().map(|c| if c.is_alphanumeric() || c == '-' { c } else { '_' }).collect();
            dialog.output_document = format!("build/{stem}.designcraft");
            if dialog.export_pdf {
                dialog.output_pdf = format!("build/{stem}.pdf");
            }
        }
    });

    ui.checkbox(&mut dialog.bilingual, crate::i18n::tr(lang, "Bilingual Publication (Indonesian / English)"));

    ui.add_space(4.0);
    ui.label(RichText::new(crate::i18n::tr(lang, "Manifest Save Path:")).strong().size(12.0).color(t.text));
    ui.horizontal(|ui| {
        let w = (ui.available_width() - 75.0).max(120.0);
        ui.add_sized(
            [w, FIELD_H],
            egui::TextEdit::singleline(&mut dialog.manifest_save_path).hint_text("e.g. C:/Projects/Karawang/pubgen_project.json"),
        );
        if outline_button(ui, crate::i18n::tr(lang, "Browse…"), 64.0).clicked() {
            let ctx = ui.ctx().clone();
            let _ = app.ask_save("pubgen_project.json", crate::app::file_dialogs::Bound::App, move |_app, picked| {
                super::with_state(&ctx, |st| {
                    if let Some(dialog) = &mut st.compile_new_dialog {
                        dialog.manifest_save_path = picked;
                    }
                });
                Ok(serde_json::Value::Null)
            });
        }
    });
    ui.label(RichText::new("All chapter and output paths will be saved relative to this manifest folder.").size(10.0).color(t.text_dim));

    ui.add_space(6.0);
    ui.horizontal(|ui| {
        ui.label(RichText::new(format!("Chapters ({})", dialog.chapters.len())).strong().size(12.0).color(t.text));
        if outline_button(ui, crate::i18n::tr(lang, "+ Add Workbook…"), 130.0).clicked() {
            let ctx = ui.ctx().clone();
            let _ = app.ask_open("dataMerge", crate::app::file_dialogs::Bound::App, move |_app, path| {
                super::with_state(&ctx, |st| {
                    if let Some(dialog) = &mut st.compile_new_dialog {
                        let p = PathBuf::from(&path);
                        let num = (dialog.chapters.len() + 1) as u32;
                        let stem = p.file_stem().and_then(std::ffi::OsStr::to_str).unwrap_or("Chapter");
                        let title = deduce_chapter_title(stem, num);
                        dialog.chapters.push(ChapterSpec { chapter: num, path, title });
                        if dialog.manifest_save_path.is_empty()
                            && let Some(parent) = p.parent()
                        {
                            dialog.manifest_save_path = parent.join("pubgen_project.json").to_string_lossy().to_string();
                        }
                    }
                });
                Ok(serde_json::Value::Null)
            });
        }
    });

    // Manual quick-add row
    ui.horizontal(|ui| {
        ui.label(RichText::new("Quick add:").size(10.5).color(t.text_dim));
        ui.add_sized([35.0, FIELD_H], egui::DragValue::new(&mut dialog.new_chapter_num).prefix("#"));
        ui.add_sized([90.0, FIELD_H], egui::TextEdit::singleline(&mut dialog.new_chapter_title).hint_text("Title"));
        let w = (ui.available_width() - 60.0).max(80.0);
        ui.add_sized([w, FIELD_H], egui::TextEdit::singleline(&mut dialog.new_chapter_path).hint_text("Path to .xlsx"));
        if ui.button(crate::i18n::tr(lang, "Add")).clicked() && !dialog.new_chapter_path.trim().is_empty() {
            let num = dialog.new_chapter_num;
            let title =
                if dialog.new_chapter_title.trim().is_empty() { format!("Chapter {num}") } else { dialog.new_chapter_title.trim().to_string() };
            dialog.chapters.push(ChapterSpec { chapter: num, path: dialog.new_chapter_path.trim().to_string(), title });
            dialog.new_chapter_num = dialog.new_chapter_num.saturating_add(1);
            dialog.new_chapter_title.clear();
            dialog.new_chapter_path.clear();
        }
    });

    if dialog.chapters.is_empty() {
        ui.label(
            RichText::new(crate::i18n::tr(lang, "No chapters added yet. Click '+ Add Workbook…' to select chapter files."))
                .size(10.5)
                .color(t.text_dim),
        );
    } else {
        let mut to_remove = None;
        let mut to_move_up = None;
        let mut to_move_down = None;
        let len = dialog.chapters.len();

        for (idx, ch) in dialog.chapters.iter_mut().enumerate() {
            ui.horizontal(|ui| {
                ui.label(RichText::new(format!("#{}:", ch.chapter)).strong().color(t.text));
                ui.add_sized([100.0, FIELD_H], egui::TextEdit::singleline(&mut ch.title).hint_text("Title"));
                let w = (ui.available_width() - 85.0).max(80.0);
                ui.add_sized([w, FIELD_H], egui::TextEdit::singleline(&mut ch.path).hint_text("Path"));
                if idx > 0 && ui.button("▲").on_hover_text("Move up").clicked() {
                    to_move_up = Some(idx);
                }
                if idx + 1 < len && ui.button("▼").on_hover_text("Move down").clicked() {
                    to_move_down = Some(idx);
                }
                if ui.button("✕").on_hover_text("Remove").clicked() {
                    to_remove = Some(idx);
                }
            });
        }

        if let Some(idx) = to_remove {
            dialog.chapters.remove(idx);
            for (i, c) in dialog.chapters.iter_mut().enumerate() {
                c.chapter = (i + 1) as u32;
            }
        }
        if let Some(idx) = to_move_up {
            dialog.chapters.swap(idx, idx - 1);
            for (i, c) in dialog.chapters.iter_mut().enumerate() {
                c.chapter = (i + 1) as u32;
            }
        }
        if let Some(idx) = to_move_down {
            dialog.chapters.swap(idx, idx + 1);
            for (i, c) in dialog.chapters.iter_mut().enumerate() {
                c.chapter = (i + 1) as u32;
            }
        }
    }

    ui.add_space(6.0);
    ui.label(RichText::new(crate::i18n::tr(lang, "Front Matter Workbook (Optional):")).strong().size(11.5).color(t.text));
    ui.horizontal(|ui| {
        let w = (ui.available_width() - 75.0).max(120.0);
        ui.add_sized([w, FIELD_H], egui::TextEdit::singleline(&mut dialog.front_matter_workbook).hint_text("e.g. workbooks/00_Front_Matter.xlsx"));
        if outline_button(ui, crate::i18n::tr(lang, "Browse…"), 64.0).clicked() {
            let ctx = ui.ctx().clone();
            let _ = app.ask_open("dataMerge", crate::app::file_dialogs::Bound::App, move |_app, path| {
                super::with_state(&ctx, |st| {
                    if let Some(dialog) = &mut st.compile_new_dialog {
                        dialog.front_matter_workbook = path;
                    }
                });
                Ok(serde_json::Value::Null)
            });
        }
    });
    ui.label(
        RichText::new("If empty, leading narrative rows (G/Note/Ulasan) of Chapter 1 are emitted as front matter.").size(10.0).color(t.text_dim),
    );

    ui.add_space(6.0);
    ui.label(RichText::new(crate::i18n::tr(lang, "Output Files:")).strong().size(11.5).color(t.text));
    ui.horizontal(|ui| {
        ui.label(RichText::new("Document:").size(11.0).color(t.text));
        let w = (ui.available_width() - 10.0).max(140.0);
        ui.add_sized([w, FIELD_H], egui::TextEdit::singleline(&mut dialog.output_document).hint_text("Optional (default: beside manifest)"));
    });
    ui.checkbox(&mut dialog.export_pdf, crate::i18n::tr(lang, "Export PDF alongside document"));
    if dialog.export_pdf {
        ui.horizontal(|ui| {
            ui.label(RichText::new("PDF Path:").size(11.0).color(t.text));
            let w = (ui.available_width() - 10.0).max(140.0);
            ui.add_sized([w, FIELD_H], egui::TextEdit::singleline(&mut dialog.output_pdf).hint_text("Optional (default: beside document)"));
        });
    }
}

fn render_new_project_cover_tab(app: &mut DesignApp, ui: &mut Ui, dialog: &mut NewProjectDialog, t: &Tokens, lang: &str) {
    ui.label(RichText::new(crate::i18n::tr(lang, "Cover Template & Theme:")).strong().size(12.0).color(t.text));

    // Cover Style
    ui.horizontal(|ui| {
        ui.label(RichText::new("Cover style:").size(11.0).color(t.text));
        let cur_name = cover_style_label(dialog.cover_style);
        let combo_w = (ui.available_width() - 10.0).clamp(120.0, 240.0);
        craft_ui::ComboBox::from_id_salt("new_proj_cover_style").selected_text(cur_name).width(combo_w).show_ui(ui, |ui| {
            for (idx, name) in COVER_STYLES.iter().enumerate() {
                ui.selectable_value(&mut dialog.cover_style, idx, *name);
            }
            ui.separator();
            for (i, p) in super::ai_cover::AI_PRESETS.iter().enumerate() {
                let sid = super::ai_cover::AI_STYLE_BASE + i;
                ui.selectable_value(&mut dialog.cover_style, sid, format!("AI {} (online)", p.name));
            }
        });
    });

    // Cover Theme
    ui.horizontal(|ui| {
        ui.label(RichText::new("Color theme:").size(11.0).color(t.text));
        let cur_theme = COVER_THEMES.get(dialog.cover_theme_preset).copied().unwrap_or("Classic BPS (Blue/Gold)");
        let combo_w = (ui.available_width() - 10.0).clamp(120.0, 240.0);
        craft_ui::ComboBox::from_id_salt("new_proj_cover_theme").selected_text(cur_theme).width(combo_w).show_ui(ui, |ui| {
            for (idx, name) in COVER_THEMES.iter().enumerate() {
                ui.selectable_value(&mut dialog.cover_theme_preset, idx, *name);
            }
        });
    });

    ui.add_space(4.0);
    ui.checkbox(&mut dialog.cover_include_inside, crate::i18n::tr(lang, "Include Inside Cover (Halaman Judul)"));
    ui.checkbox(&mut dialog.cover_include_back, crate::i18n::tr(lang, "Include Back Cover"));

    ui.add_space(6.0);
    ui.label(RichText::new(crate::i18n::tr(lang, "Cover Source Workbook (Optional):")).strong().size(11.5).color(t.text));
    ui.horizontal(|ui| {
        let w = (ui.available_width() - 75.0).max(120.0);
        ui.add_sized([w, FIELD_H], egui::TextEdit::singleline(&mut dialog.cover_source).hint_text("e.g. workbooks/BAB_01_2025.xlsx"));
        if outline_button(ui, crate::i18n::tr(lang, "Browse…"), 64.0).clicked() {
            let ctx = ui.ctx().clone();
            let _ = app.ask_open("dataMerge", crate::app::file_dialogs::Bound::App, move |_app, path| {
                super::with_state(&ctx, |st| {
                    if let Some(dialog) = &mut st.compile_new_dialog {
                        dialog.cover_source = path;
                    }
                });
                Ok(serde_json::Value::Null)
            });
        }
    });
    ui.label(
        RichText::new("Workbook carrying the COVER sheet. If empty, the first chapter workbook with a COVER sheet supplies values.")
            .size(10.0)
            .color(t.text_dim),
    );
}

fn render_new_project_toc_tab(ui: &mut Ui, dialog: &mut NewProjectDialog, t: &Tokens, lang: &str) {
    ui.label(RichText::new(crate::i18n::tr(lang, "Table of Contents Lists:")).strong().size(12.0).color(t.text));
    ui.checkbox(&mut dialog.toc_contents, crate::i18n::tr(lang, "Daftar Isi (Contents — Chapters & Narratives)"));
    ui.checkbox(&mut dialog.toc_tables, crate::i18n::tr(lang, "Daftar Tabel (List of Tables)"));
    ui.checkbox(&mut dialog.toc_charts, crate::i18n::tr(lang, "Daftar Grafik (List of Charts)"));

    ui.add_space(4.0);
    ui.horizontal(|ui| {
        ui.label(RichText::new("Placement:").size(11.0).color(t.text));
        ui.radio_value(&mut dialog.toc_placement, "afterFrontMatter".into(), crate::i18n::tr(lang, "After Front Matter"));
        ui.radio_value(&mut dialog.toc_placement, "end".into(), crate::i18n::tr(lang, "At End of Document"));
    });

    ui.checkbox(&mut dialog.toc_title_bilingual, crate::i18n::tr(lang, "Bilingual TOC Titles (e.g. Daftar Isi / Contents)"));
    ui.checkbox(&mut dialog.toc_hanging_indent, crate::i18n::tr(lang, "Indent wrapped TOC entries (hanging indent)"));

    ui.separator();
    ui.label(RichText::new(crate::i18n::tr(lang, "Page Numbering & Sections:")).strong().size(12.0).color(t.text));

    const NUMBERING_STYLES: &[(&str, &str)] = &[
        ("lowerRoman", "Lower Roman (i, ii, iii)"),
        ("arabic", "Arabic (1, 2, 3)"),
        ("upperRoman", "Upper Roman (I, II, III)"),
        ("lowerLetters", "Lower Letters (a, b, c)"),
        ("upperLetters", "Upper Letters (A, B, C)"),
    ];

    ui.horizontal(|ui| {
        ui.label(RichText::new("Front matter format:").size(11.0).color(t.text));
        let cur = NUMBERING_STYLES.iter().find(|(k, _)| *k == dialog.pagination_front_format).map(|(_, l)| *l).unwrap_or("Lower Roman");
        craft_ui::ComboBox::from_id_salt("new_proj_fm_fmt").selected_text(cur).width(160.0).show_ui(ui, |ui| {
            for (val, label) in NUMBERING_STYLES {
                ui.selectable_value(&mut dialog.pagination_front_format, (*val).into(), *label);
            }
        });
    });

    ui.horizontal(|ui| {
        ui.label(RichText::new("Body format:").size(11.0).color(t.text));
        let cur = NUMBERING_STYLES.iter().find(|(k, _)| *k == dialog.pagination_body_format).map(|(_, l)| *l).unwrap_or("Arabic");
        craft_ui::ComboBox::from_id_salt("new_proj_body_fmt").selected_text(cur).width(160.0).show_ui(ui, |ui| {
            for (val, label) in NUMBERING_STYLES {
                ui.selectable_value(&mut dialog.pagination_body_format, (*val).into(), *label);
            }
        });
    });

    ui.checkbox(&mut dialog.pagination_restart_body, crate::i18n::tr(lang, "Restart numbering at arabic 1 on Chapter 1"));

    ui.separator();
    ui.label(RichText::new(crate::i18n::tr(lang, "Running Headers & Footers:")).strong().size(12.0).color(t.text));
    ui.horizontal(|ui| {
        ui.label(RichText::new("Preset:").size(11.0).color(t.text));
        let presets = ["book", "standard", "minimal"];
        craft_ui::ComboBox::from_id_salt("new_proj_hf_preset").selected_text(&dialog.header_footer_preset).width(120.0).show_ui(ui, |ui| {
            for p in presets {
                ui.selectable_value(&mut dialog.header_footer_preset, p.into(), p);
            }
        });
    });
    ui.label(
        RichText::new("Running headers propagate each chapter's title across its body pages and skip covers/front matter.")
            .size(10.0)
            .color(t.text_dim),
    );
}

pub(crate) fn compile_new_project_dialog_ui(app: &mut DesignApp, ui: &mut Ui, st: &mut PubgenState, t: &Tokens) {
    let Some(mut dialog) = st.compile_new_dialog.take() else { return };
    let mut open = true;
    let mut close = false;
    let mut save_and_compile = false;
    let mut save_and_open = false;

    let lang = app.ui.language.clone();

    let mut close_btn = false;
    super::dialogs::dialog_window(
        ui.ctx(),
        crate::i18n::tr(&lang, "New Publication Project (pubgen_project.json)"),
        crate::i18n::is_rtl(&lang),
        true,
        &mut open,
        580.0,
        |ui| {
            // Tabs
            ui.horizontal(|ui| {
                ui.selectable_value(&mut dialog.tab, NewProjectTab::Files, crate::i18n::tr(&lang, "1. Files & Chapters"));
                ui.selectable_value(&mut dialog.tab, NewProjectTab::Cover, crate::i18n::tr(&lang, "2. Cover Style"));
                ui.selectable_value(&mut dialog.tab, NewProjectTab::TocAndLayout, crate::i18n::tr(&lang, "3. TOC & Layout"));
            });
            ui.separator();

            egui::ScrollArea::vertical().max_height(480.0).show(ui, |ui| match dialog.tab {
                NewProjectTab::Files => render_new_project_files_tab(app, ui, &mut dialog, t, &lang),
                NewProjectTab::Cover => render_new_project_cover_tab(app, ui, &mut dialog, t, &lang),
                NewProjectTab::TocAndLayout => render_new_project_toc_tab(ui, &mut dialog, t, &lang),
            });

            ui.separator();
            if let Some(err) = &dialog.error_message {
                ui.label(RichText::new(err).size(11.0).color(egui::Color32::from_rgb(220, 80, 80)));
            }

            ui.horizontal(|ui| {
                if ui.button(crate::i18n::tr(&lang, "Cancel")).clicked() {
                    close_btn = true;
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button(RichText::new(crate::i18n::tr(&lang, "Save & Compile Now")).color(t.accent)).clicked() {
                        save_and_compile = true;
                    }
                    if ui.button(crate::i18n::tr(&lang, "Save Project Manifest")).clicked() {
                        save_and_open = true;
                    }
                });
            });
        },
    );
    if close_btn {
        close = true;
    }

    if save_and_open || save_and_compile {
        match dialog.save_manifest() {
            Ok(saved_path) => {
                let path_str = saved_path.to_string_lossy().to_string();
                st.compile_manifest_path = path_str.clone();
                st.compile_checked.clear();
                st.compile_summary.clear();
                let _ = remember_manifest(app, &path_str);
                if save_and_compile {
                    start_compile(app, &path_str, st);
                } else {
                    st.compile_status = Some(("Project manifest saved. Ready to compile.".into(), true));
                }
                close = true;
            }
            Err(e) => {
                dialog.error_message = Some(e);
            }
        }
    }

    if open && !close {
        st.compile_new_dialog = Some(dialog);
    }
}

fn start_compile(app: &mut DesignApp, path: &str, st: &mut PubgenState) {
    match CompilePlan::load(Path::new(path)) {
        Ok(plan) => {
            let summary = plan.summary_line();
            if let Err(e) = ensure_output_not_open(app, &plan).and_then(|()| remember_manifest(app, path)) {
                st.compile_status = Some((e, false));
                return;
            }
            st.compile_manifest_path = path.to_string();
            st.compile_checked = path.to_string();
            st.compile_summary = summary;
            st.compile_status = Some(("Compiling…".into(), true));
            st.compile_job = Some(Box::new(CompileJob::new(plan)));
        }
        Err(e) => st.compile_status = Some((e, false)),
    }
}

fn pdf_override(plan: &CompilePlan, path: &str) -> Result<PathBuf, String> {
    let resolved = resolve_against(plan.dir(), path);
    if resolved.extension().and_then(std::ffi::OsStr::to_str).is_none() {
        return Err(format!("pdf: {} needs a .pdf extension", resolved.display()));
    }
    require_writable_parent(&resolved, "pdf")?;
    Ok(resolved)
}

fn remember_manifest(app: &mut DesignApp, path: &str) -> Result<(), String> {
    let stored = std::fs::canonicalize(path).unwrap_or_else(|_| PathBuf::from(path)).to_string_lossy().to_string();
    app.run("prefs.set", serde_json::json!({"pubgenProjectPath": stored})).map(|_| ()).map_err(|e| format!("prefs: {e}"))
}

/// Refuse to build over a document that is open in another tab: saving would replace the file on
/// disk while the open copy keeps editing the old state.
fn ensure_output_not_open(app: &DesignApp, plan: &CompilePlan) -> Result<(), String> {
    let output = canonical_or(&plan.output_document);
    if app.session.documents().iter().filter_map(|d| d.path.as_deref()).any(|p| canonical_or(Path::new(p)) == output) {
        return Err(format!(
            "{} is open in a tab; close it before compiling so the build cannot overwrite unsaved edits",
            plan.output_document.display()
        ));
    }
    Ok(())
}

fn canonical_or(path: &Path) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

fn report_json(report: &CompileReport) -> Value {
    serde_json::json!({
        "document": report.document,
        "pdf": report.pdf,
        "pages": report.pages,
        "chapters": report
            .chapters
            .iter()
            .map(|c| serde_json::json!({
                "chapter": c.chapter,
                "workbook": c.workbook,
                "pages": c.pages,
                "tables": c.tables,
                "charts": c.charts,
            }))
            .collect::<Vec<_>>(),
        "warnings": report.warnings,
        "elapsedMs": report.elapsed_ms,
    })
}

fn format_report(report: &CompileReport) -> String {
    let mut line = format!(
        "Compiled {} page(s) from {} chapter(s) into {} in {:.1}s",
        report.pages,
        report.chapters.len(),
        report.document,
        report.elapsed_ms as f64 / 1000.0
    );
    if let Some(pdf) = &report.pdf {
        line.push_str(&format!(" — PDF {pdf}"));
    }
    if !report.warnings.is_empty() {
        line.push_str(&format!(". Warnings: {}", report.warnings.join("; ")));
    }
    line
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("dc-pubgen-compile-{}-{name}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn workbook(dir: &Path, name: &str) -> String {
        std::fs::write(dir.join(name), b"xlsx placeholder").unwrap();
        name.to_string()
    }

    fn manifest(dir: &Path, chapters: &[(&str, u32)]) -> ProjectManifest {
        let mut m: ProjectManifest = serde_json::from_str("{}").unwrap();
        m.chapters = chapters.iter().map(|(name, n)| ChapterSpec { chapter: *n, path: workbook(dir, name), title: format!("Chapter {n}") }).collect();
        m
    }

    #[test]
    fn manifest_with_profile_overrides_margins_and_facing() {
        let dir = temp_dir("profile-manifest");
        let manifest_path = dir.join("pubgen_project.json");
        let mut m = manifest(&dir, &[("a.xlsx", 1)]);
        m.profile = Some("bps_brs".into());
        let plan = CompilePlan::from_manifest(&manifest_path, m).unwrap();
        assert_eq!(plan.manifest.document.margins_mm.top, 18.0);
        assert_eq!(plan.manifest.document.margins_mm.inside, 18.0);
        assert!(!plan.manifest.document.facing_pages);
    }

    #[test]
    fn defaults_need_only_chapters() {
        let m: ProjectManifest = serde_json::from_str(r#"{"chapters":[{"chapter":1,"path":"a.xlsx"}]}"#).unwrap();
        assert_eq!(m.version, 1);
        assert_eq!(m.document.preset, "A4");
        assert_eq!(m.document.margins_mm.top, 20.20);
        assert_eq!(m.cover.style, 0);
        assert!(m.cover.include_inside && m.cover.include_back);
        assert_eq!(m.toc.placement, "afterFrontMatter");
        assert_eq!(m.toc.lists, ["contents", "tables", "charts"]);
        assert_eq!(m.header_footer.preset, "book");
        assert!(!m.build.export_pdf);
        assert!(!m.bilingual);
    }

    #[test]
    fn paths_resolve_against_the_manifest_folder_not_the_cwd() {
        let dir = temp_dir("resolve");
        workbook(&dir, "BAB 01.xlsx");
        let m: ProjectManifest = serde_json::from_str(r#"{"chapters":[{"chapter":1,"path":"BAB 01.xlsx"}]}"#).unwrap();
        let plan = CompilePlan::from_manifest(&dir.join("project.json"), m).unwrap();
        assert_eq!(plan.chapters[0].source, ChapterSource::Workbook(dir.join("BAB 01.xlsx")));
        assert_eq!(plan.output_document, dir.join("project.designcraft"));
        assert_eq!(plan.resolve("workbooks/a.xlsx"), dir.join("workbooks/a.xlsx"));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn invalid_manifests_are_named_errors() {
        let dir = temp_dir("invalid");
        workbook(&dir, "a.xlsx");
        workbook(&dir, "b.xlsx");
        let base: ProjectManifest = serde_json::from_str(r#"{"chapters":[{"chapter":1,"path":"a.xlsx"}]}"#).unwrap();

        let empty = ProjectManifest { chapters: Vec::new(), ..base.clone() };
        assert!(CompilePlan::from_manifest(&dir.join("p.json"), empty).unwrap_err().contains("at least one chapter"));

        let missing =
            ProjectManifest { chapters: vec![ChapterSpec { chapter: 1, path: "missing.xlsx".into(), title: String::new() }], ..base.clone() };
        assert!(CompilePlan::from_manifest(&dir.join("p.json"), missing).unwrap_err().contains("is not a file"));

        let csv = ProjectManifest { chapters: vec![ChapterSpec { chapter: 1, path: workbook(&dir, "c.csv"), title: String::new() }], ..base.clone() };
        assert!(CompilePlan::from_manifest(&dir.join("p.json"), csv).unwrap_err().contains("not an .xlsx"));

        let duplicate_path = ProjectManifest {
            chapters: vec![
                ChapterSpec { chapter: 1, path: "a.xlsx".into(), title: String::new() },
                ChapterSpec { chapter: 2, path: "a.xlsx".into(), title: String::new() },
            ],
            ..base.clone()
        };
        assert!(CompilePlan::from_manifest(&dir.join("p.json"), duplicate_path).unwrap_err().contains("used twice"));

        let duplicate_number = ProjectManifest {
            chapters: vec![
                ChapterSpec { chapter: 1, path: "a.xlsx".into(), title: String::new() },
                ChapterSpec { chapter: 1, path: "b.xlsx".into(), title: String::new() },
            ],
            ..base.clone()
        };
        assert!(CompilePlan::from_manifest(&dir.join("p.json"), duplicate_number).unwrap_err().contains("appears twice"));

        let zero = ProjectManifest { chapters: vec![ChapterSpec { chapter: 0, path: "a.xlsx".into(), title: String::new() }], ..base.clone() };
        assert!(CompilePlan::from_manifest(&dir.join("p.json"), zero).unwrap_err().contains("at least 1"));

        let many = ProjectManifest {
            chapters: (1..=65).map(|n| ChapterSpec { chapter: n, path: workbook(&dir, &format!("{n}.xlsx")), title: String::new() }).collect(),
            ..base.clone()
        };
        assert!(CompilePlan::from_manifest(&dir.join("p.json"), many).unwrap_err().contains("at most 64"));

        let output = ProjectManifest { output: OutputSpec { document: "missing/out.designcraft".into(), pdf: String::new() }, ..base.clone() };
        assert!(CompilePlan::from_manifest(&dir.join("p.json"), output).unwrap_err().contains("does not exist"));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn chapter_order_is_preserved_and_descending_numbers_warn() {
        let dir = temp_dir("order");
        let m = manifest(&dir, &[("a.xlsx", 2), ("b.xlsx", 1)]);
        let plan = CompilePlan::from_manifest(&dir.join("p.json"), m).unwrap();
        assert_eq!(plan.chapters.iter().map(|c| c.chapter).collect::<Vec<_>>(), [2, 1]);
        assert!(plan.warnings.iter().any(|w| w.contains("out of order")));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn steps_follow_the_pipeline_for_one_two_and_ten_chapters() {
        let dir = temp_dir("steps");
        let check = |count: usize| {
            let names: Vec<String> = (1..=count).map(|n| format!("{n}.xlsx")).collect();
            let chapters: Vec<(&str, u32)> = names.iter().map(String::as_str).zip(1..).collect();
            let m = manifest(&dir, &chapters);
            let plan = CompilePlan::from_manifest(&dir.join("p.json"), m).unwrap();
            let steps = plan.steps();
            assert_eq!(steps.first(), Some(&CompileStep::NewDocument));
            assert_eq!(steps.get(1), Some(&CompileStep::MasterSetup));
            assert_eq!(steps.get(2), Some(&CompileStep::FrontCover));
            assert_eq!(steps.get(3), Some(&CompileStep::InsideCover));
            assert_eq!(steps.iter().filter(|s| matches!(s, CompileStep::Chapter(_))).count(), count);
            assert_eq!(steps.last(), Some(&CompileStep::SaveDocument), "no PDF by default");
            let chapter_positions: Vec<usize> =
                steps.iter().enumerate().filter(|(_, s)| matches!(s, CompileStep::Chapter(_))).map(|(i, _)| i).collect();
            assert!(chapter_positions.windows(2).all(|w| w[1] == w[0] + 1), "{steps:?}");
            let back = steps.iter().position(|s| *s == CompileStep::BackCover).unwrap();
            assert!(back > *chapter_positions.last().unwrap());
        };
        check(1);
        check(2);
        check(10);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn optional_steps_follow_the_manifest() {
        let dir = temp_dir("optional");
        let mut m = manifest(&dir, &[("a.xlsx", 1)]);
        m.cover.include_inside = false;
        m.cover.include_back = false;
        m.toc.lists.clear();
        m.build.export_pdf = true;
        let plan = CompilePlan::from_manifest(&dir.join("p.json"), m).unwrap();
        let steps = plan.steps();
        assert!(!steps.contains(&CompileStep::InsideCover));
        assert!(!steps.contains(&CompileStep::BackCover));
        assert!(steps.contains(&CompileStep::FrontMatter), "the chapter-1 fallback is discovered at compile time");
        assert!(!steps.contains(&CompileStep::TocPlaceholders));
        assert!(!steps.contains(&CompileStep::TocFinalize));
        assert_eq!(steps.last(), Some(&CompileStep::ExportPdf));
        assert_eq!(plan.output_pdf, Some(dir.join("p.pdf")), "the PDF follows the document path");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn lists_at_the_end_are_planned_after_the_chapters() {
        let dir = temp_dir("lists-end");
        let m: ProjectManifest = serde_json::from_value(serde_json::json!({
            "chapters": [
                {"chapter": 1, "path": workbook(&dir, "a.xlsx")},
                {"chapter": 2, "path": workbook(&dir, "b.xlsx")},
            ],
            "toc": {"placement": "end", "lists": ["contents"]},
        }))
        .unwrap();
        let plan = CompilePlan::from_manifest(&dir.join("p.json"), m).unwrap();
        let steps = plan.steps();
        let last_chapter = steps.iter().rposition(|s| matches!(s, CompileStep::Chapter(_))).unwrap();
        let lists = steps.iter().position(|s| *s == CompileStep::TocPlaceholders).unwrap();
        assert!(lists > last_chapter, "{steps:?}");
        assert!(steps.iter().position(|s| *s == CompileStep::BackCover).unwrap() > lists);
        std::fs::remove_dir_all(&dir).ok();
    }

    fn catalog(dir: &Path, name: &str, rows: &str) -> PathBuf {
        let path = dir.join(format!("{name}_catalog.csv"));
        std::fs::write(&path, format!("type,number,title,file\n{rows}")).unwrap();
        path
    }

    /// A two-chapter publication whose chapters read pre-converted catalogs, so the pipeline runs
    /// without binary XLSX fixtures.
    fn catalog_plan(dir: &Path, chapters: &[(&str, &str)], lists: &[&str]) -> CompilePlan {
        let mut manifest: ProjectManifest = serde_json::from_value(serde_json::json!({
            "publicationTitle": "Test Publication",
            "cover": {"style": 0, "includeInside": false, "includeBack": false},
            "toc": {"lists": lists, "titleBilingual": false},
            "build": {"exportPdf": false},
        }))
        .unwrap();
        manifest.chapters = chapters
            .iter()
            .enumerate()
            .map(|(i, (name, _))| ChapterSpec { chapter: i as u32 + 1, path: workbook(dir, &format!("{name}.xlsx")), title: String::new() })
            .collect();
        let mut plan = CompilePlan::from_manifest(&dir.join("project.json"), manifest).unwrap();
        for (i, (name, rows)) in chapters.iter().enumerate() {
            let catalog = catalog(dir, name, rows);
            plan.chapters.get_mut(i).unwrap().source = ChapterSource::Catalog { catalog, tables: Vec::new() };
        }
        plan
    }

    fn app() -> DesignApp {
        DesignApp::new(designcraft_engine::Session::new(), crate::Services::default())
    }

    #[test]
    fn compiles_two_catalogs_into_one_numbered_document() {
        let dir = temp_dir("compile");
        let plan = catalog_plan(
            &dir,
            &[
                ("chapter1", "Div,01,Geografi,\nTabel,01.01,Luas Wilayah,\nGrafik,01.02,Kepadatan,\n"),
                ("chapter2", "Div,02,Iklim,\nTabel,02.01,Curah Hujan,\n"),
            ],
            &["contents", "tables"],
        );
        let output = plan.output_document.clone();
        let mut app = app();
        let mut job = CompileJob::new(plan);
        let report = job.run_all(&mut app).unwrap();

        assert!(output.is_file(), "the compiled document is written only on success");
        assert_eq!(report.chapters.len(), 2);
        assert_eq!(report.chapters[0].tables, 1);
        assert_eq!(report.chapters[0].charts, 1);
        assert_eq!(report.warnings.iter().filter(|w| w.contains("narrative")).count(), 1, "chapter 1 has no front matter");

        let active = app.session.active().unwrap();
        let doc = &active.doc;
        let body = job.body_start.unwrap();
        assert!(body > 0);
        assert_eq!(doc.page_name(body), "1", "the body restarts at arabic 1");
        assert!(doc.page_name(body - 1).chars().all(|c| "ivxlcdm".contains(c)), "front matter is roman: {}", doc.page_name(body - 1));
        assert_eq!(doc.tocs.len(), 2);

        assert!(doc.styles.char_style("Running Header").is_some());
        assert!(doc.variable("Running Header").is_some());
        let hf = doc.header_footer.as_ref().unwrap();
        assert!(matches!(hf.scope, designcraft_doc::HfScope::Range { .. }), "body-only running headers: {:?}", hf.scope);
    }

    #[test]
    fn the_second_list_pass_adds_no_pages() {
        let dir = temp_dir("toc-fixed-point");
        let plan =
            catalog_plan(&dir, &[("chapter1", "Div,01,Geografi,\nTabel,01.01,Luas,\nGrafik,01.02,Kepadatan,\n")], &["contents", "tables", "charts"]);
        let mut app = app();
        let mut job = CompileJob::new(plan);
        job.run_all(&mut app).unwrap();

        let pages: Vec<(String, usize)> = job.toc_pages.clone();
        let lists = job.plan.manifest.toc.lists.clone();
        for ((list, page), kind) in pages.iter().zip(lists.iter()) {
            let st = job.toc_state(kind);
            let plan = TocPlan::from_state(&st);
            let (_count, added) = generate_toc(&mut app, &st, *page, &plan, true).unwrap();
            assert_eq!(added, 0, "{list} is stable on a fresh pass");
        }
    }

    #[test]
    fn the_first_chapters_leading_narrative_rows_become_front_matter() {
        let dir = temp_dir("fallback");
        let plan = catalog_plan(&dir, &[("chapter1", "G,Preface,Sambutan,\nUlasan,01,Geografi,\nTabel,01.01,Luas,\n")], &["contents"]);
        let mut app = app();
        let mut job = CompileJob::new(plan);
        job.run_all(&mut app).unwrap();

        assert_eq!(job.front_matter_rows, 2, "the G and Ulasan rows both precede the first table");
        let active = app.session.active().unwrap();
        let text: String = active.doc.stories.values().map(|s| s.text.clone()).collect();
        assert!(text.contains("Sambutan"), "the narrative page is emitted before the body");
        assert!(job.body_start.unwrap() > 2);
    }

    #[test]
    fn a_failed_compile_writes_no_document() {
        let dir = temp_dir("failed");
        let plan = catalog_plan(&dir, &[("chapter1", "")], &[]);
        let output = plan.output_document.clone();
        let mut app = app();
        let mut job = CompileJob::new(plan);
        let error = job.run_all(&mut app).unwrap_err();
        assert!(error.contains("no rows"), "{error}");
        assert!(!output.exists());
        assert!(matches!(job.state, JobState::Failed(_)));
    }

    #[test]
    fn cancelling_before_the_first_step_writes_nothing() {
        let dir = temp_dir("cancel");
        let plan = catalog_plan(&dir, &[("chapter1", "Div,01,Geografi,\n")], &[]);
        let output = plan.output_document.clone();
        let mut app = app();
        let mut job = CompileJob::new(plan);
        job.cancel();
        assert_eq!(job.advance(&mut app), StepOutcome::Cancelled);
        assert!(!output.exists());
    }

    #[test]
    fn catalog_sources_keep_catalog_and_tables_unchanged() {
        let dir = temp_dir("catalog");
        workbook(&dir, "a.xlsx");
        let m = manifest(&dir, &[("a.xlsx", 1)]);
        let mut plan = CompilePlan::from_manifest(&dir.join("p.json"), m).unwrap();
        let catalog = dir.join("a_catalog.csv");
        let tables = vec![dir.join("01_01.csv")];
        plan.chapters[0].source = ChapterSource::Catalog { catalog: catalog.clone(), tables: tables.clone() };
        assert_eq!(plan.chapters[0].source, ChapterSource::Catalog { catalog, tables });
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn unknown_toc_lists_and_numbering_styles_are_refused() {
        let dir = temp_dir("lists");
        workbook(&dir, "a.xlsx");
        let mut m = manifest(&dir, &[("a.xlsx", 1)]);
        m.toc.lists = vec!["figures".into()];
        assert!(CompilePlan::from_manifest(&dir.join("p.json"), m).unwrap_err().contains("contents, tables or charts"));

        let mut m = manifest(&dir, &[("a.xlsx", 1)]);
        m.pagination.body_format = "roman_lower".into();
        assert!(CompilePlan::from_manifest(&dir.join("p.json"), m).unwrap_err().contains("unsupported numbering style"));

        let mut m = manifest(&dir, &[("a.xlsx", 1)]);
        m.toc.placement = "middle".into();
        assert!(CompilePlan::from_manifest(&dir.join("p.json"), m).unwrap_err().contains("afterFrontMatter or end"));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn stepwise_and_run_all_produce_the_same_publication() {
        let dir = temp_dir("stepwise");
        let chapters = [("chapter1", "Div,01,Geografi,\nTabel,01.01,Luas,\n"), ("chapter2", "Div,02,Iklim,\nTabel,02.01,Hujan,\n")];
        let mut sync_app = app();
        let sync_report = CompileJob::new(catalog_plan(&dir, &chapters, &["contents", "tables"])).run_all(&mut sync_app).unwrap();

        let mut step_app = app();
        let mut job = CompileJob::new(catalog_plan(&dir, &chapters, &["contents", "tables"]));
        let mut last_done = 0usize;
        let mut total = None;
        loop {
            let (done, steps) = job.progress();
            assert!(done >= last_done, "progress never goes back: {done} after {last_done}");
            assert_eq!(*total.get_or_insert(steps), steps, "the step count never changes");
            last_done = done;
            match job.advance(&mut step_app) {
                StepOutcome::Pending => {}
                StepOutcome::Done => break,
                StepOutcome::Cancelled => panic!("not cancelled"),
                StepOutcome::Failed(e) => panic!("{e}"),
            }
        }

        assert_eq!(job.report().pages, sync_report.pages);
        assert_eq!(job.report().chapters.len(), sync_report.chapters.len());
        assert_eq!(step_app.session.active().unwrap().doc.page_count(), sync_app.session.active().unwrap().doc.page_count());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn cancelling_midway_stops_before_the_next_step() {
        let dir = temp_dir("cancel-midway");
        let plan = catalog_plan(&dir, &[("chapter1", "Div,01,Geografi,\nTabel,01.01,Luas,\n")], &["contents"]);
        let output = plan.output_document.clone();
        let mut app = app();
        let mut job = CompileJob::new(plan);
        for _ in 0..3 {
            assert_eq!(job.advance(&mut app), StepOutcome::Pending);
        }
        let pages = app.session.active().unwrap().doc.page_count();
        job.cancel();
        assert_eq!(job.advance(&mut app), StepOutcome::Cancelled);
        assert_eq!(app.session.active().unwrap().doc.page_count(), pages, "cancelling runs no further step");
        assert_eq!(job.state, JobState::Cancelled);
        assert!(!output.exists(), "a cancelled compile saves nothing");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn a_failed_compile_leaves_the_previous_build_byte_identical() {
        let dir = temp_dir("previous-build");
        let plan = catalog_plan(&dir, &[("chapter1", "")], &[]);
        let output = plan.output_document.clone();
        std::fs::write(&output, b"previous build").unwrap();
        let mut app = app();
        let error = CompileJob::new(plan).run_all(&mut app).unwrap_err();
        assert!(error.contains("no rows"), "{error}");
        assert_eq!(std::fs::read(&output).unwrap(), b"previous build");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn a_document_open_at_the_output_path_is_refused() {
        let dir = temp_dir("output-open");
        workbook(&dir, "a.xlsx");
        let manifest_path = dir.join("p.json");
        std::fs::write(&manifest_path, r#"{"chapters":[{"chapter":1,"path":"a.xlsx"}]}"#).unwrap();
        let mut app = app();
        app.run("file.new", serde_json::json!({"pages": 1})).unwrap();
        let output = dir.join("p.designcraft");
        app.run("file.saveAs", serde_json::json!({"path": output.to_string_lossy()})).unwrap();

        let error = compile_project(&mut app, &serde_json::json!({"path": manifest_path.to_string_lossy()})).unwrap_err();
        assert!(error.contains("open in a tab"), "{error}");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn the_compile_command_is_registered_with_its_parameters() {
        let entry = crate::menus::UI_COMMANDS.iter().find(|c| c.0 == "pubgen.compileProject");
        let (_, label, shortcut, params) = entry.unwrap();
        assert_eq!(*label, "Compile Publication");
        assert_eq!(*shortcut, Some("Ctrl+Shift+B"));
        for key in ["path", "sync", "pdf"] {
            assert!(params.contains(key), "params doc names `{key}`: {params}");
        }
    }

    #[test]
    fn the_command_names_a_missing_manifest() {
        let dir = temp_dir("command-missing");
        let mut app = app();
        let path = dir.join("missing.json");
        let error = compile_project(&mut app, &serde_json::json!({"path": path.to_string_lossy()})).unwrap_err();
        assert!(error.contains("missing.json"), "{error}");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn new_project_dialog_defaults_and_build() {
        let dir = temp_dir("new-dialog");
        let wb1 = workbook(&dir, "BAB_01.xlsx");
        let wb2 = workbook(&dir, "BAB_02.xlsx");

        let mut dialog = NewProjectDialog::default();
        dialog.manifest_save_path = dir.join("pubgen_project.json").to_string_lossy().to_string();
        dialog.publication_title = "Kabupaten Karawang 2026".to_string();
        dialog.chapters.push(ChapterSpec { chapter: 1, path: wb1, title: "Geografi".into() });
        dialog.chapters.push(ChapterSpec { chapter: 2, path: wb2, title: "Iklim".into() });

        let (save_path, manifest) = dialog.build_manifest().unwrap();
        assert_eq!(save_path, dir.join("pubgen_project.json"));
        assert_eq!(manifest.publication_title, "Kabupaten Karawang 2026");
        assert!(manifest.bilingual);
        assert_eq!(manifest.chapters.len(), 2);
        assert_eq!(manifest.chapters[0].title, "Geografi");
        assert_eq!(manifest.cover.style, 11);
        assert_eq!(manifest.toc.lists, ["contents", "tables", "charts"]);
        assert!(manifest.build.export_pdf);

        let plan = CompilePlan::from_manifest(&save_path, manifest).unwrap();
        assert_eq!(plan.chapters.len(), 2);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn new_project_dialog_validates_path_and_chapters() {
        let mut dialog = NewProjectDialog::default();
        assert!(dialog.build_manifest().unwrap_err().contains("specify a path"));

        dialog.manifest_save_path = "project.txt".into();
        assert!(dialog.build_manifest().unwrap_err().contains(".json"));

        dialog.manifest_save_path = "project.json".into();
        assert!(dialog.build_manifest().unwrap_err().contains("at least one chapter"));

        dialog.chapters.push(ChapterSpec { chapter: 1, path: "  ".into(), title: "Title".into() });
        assert!(dialog.build_manifest().unwrap_err().contains("empty path"));
    }

    #[test]
    fn new_project_dialog_saves_file_and_loads() {
        let dir = temp_dir("new-dialog-save");
        let wb = workbook(&dir, "BAB_01.xlsx");
        let manifest_file = dir.join("pubgen_project.json");

        let mut dialog = NewProjectDialog::default();
        dialog.manifest_save_path = manifest_file.to_string_lossy().to_string();
        dialog.publication_title = "Test Publication".into();
        dialog.chapters.push(ChapterSpec { chapter: 1, path: wb, title: "Geografi".into() });

        let saved = dialog.save_manifest().unwrap();
        assert_eq!(saved, manifest_file);
        assert!(manifest_file.is_file());

        let plan = CompilePlan::load(&manifest_file).unwrap();
        assert_eq!(plan.manifest.publication_title, "Test Publication");
        assert_eq!(plan.chapters.len(), 1);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn new_project_dialog_normalizes_relative_paths() {
        let dir = temp_dir("new-dialog-rel");
        let workbooks_dir = dir.join("workbooks");
        std::fs::create_dir_all(&workbooks_dir).unwrap();
        let wb = workbooks_dir.join("BAB_01.xlsx");
        std::fs::write(&wb, b"xlsx content").unwrap();

        let manifest_file = dir.join("pubgen_project.json");

        let mut dialog = NewProjectDialog::default();
        dialog.manifest_save_path = manifest_file.to_string_lossy().to_string();
        dialog.chapters.push(ChapterSpec { chapter: 1, path: wb.to_string_lossy().to_string(), title: "Geografi".into() });

        let (_, manifest) = dialog.build_manifest().unwrap();
        assert_eq!(manifest.chapters[0].path, "workbooks/BAB_01.xlsx");
        std::fs::remove_dir_all(&dir).ok();
    }
}
