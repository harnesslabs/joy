use indicatif::{ProgressBar, ProgressStyle};
use serde::Serialize;
use serde_json::Value;
use std::io::IsTerminal;
use std::io::{self, Write};
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

use crate::commands::CommandOutput;
use crate::error::JoyError;

/// Output mode selected by CLI flags.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputMode {
  Human,
  Json,
}

/// User preference for color rendering in human output.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorPreference {
  Auto,
  Always,
  Never,
}

/// User preference for progress rendering in human output.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProgressPreference {
  Auto,
  Always,
  Never,
}

/// User preference for glyph rendering in human output.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GlyphPreference {
  Auto,
  Unicode,
  Ascii,
}

/// Resolved glyph mode after capability detection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GlyphMode {
  Unicode,
  Ascii,
}

/// Resolved human UI behavior used by the output renderer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HumanUiConfig {
  pub color_enabled: bool,
  pub progress_enabled: bool,
  pub glyph_mode: GlyphMode,
  pub stderr_is_tty: bool,
}

impl Default for HumanUiConfig {
  fn default() -> Self {
    Self {
      color_enabled: false,
      progress_enabled: false,
      glyph_mode: GlyphMode::Ascii,
      stderr_is_tty: false,
    }
  }
}

/// Install the resolved output configuration for this process invocation.
///
/// `joy` is a single-process CLI and all human progress helpers route through this global state so
/// lower layers (`fetch`, `registry`, command handlers) do not need to thread renderer handles.
pub fn configure_ui(mode: OutputMode, ui: HumanUiConfig) {
  let mut state = output_state().lock().expect("output state lock poisoned");
  if let Some(pb) = state.progress.take() {
    pb.finish_and_clear();
  }
  state.mode = mode;
  state.ui = ui;
}

/// Builder for structured human-mode command output.
#[derive(Debug, Default, Clone)]
pub struct HumanMessageBuilder {
  title: String,
  lines: Vec<String>,
  warnings: Vec<String>,
  hints: Vec<String>,
}

impl HumanMessageBuilder {
  pub fn new(title: impl Into<String>) -> Self {
    Self { title: title.into(), ..Self::default() }
  }

  pub fn line(mut self, line: impl Into<String>) -> Self {
    self.lines.push(line.into());
    self
  }

  pub fn kv(mut self, key: &str, value: impl Into<String>) -> Self {
    self.lines.push(format!("- {key}: {}", value.into()));
    self
  }

  pub fn warning(mut self, warning: impl Into<String>) -> Self {
    self.warnings.push(warning.into());
    self
  }

  pub fn hint(mut self, hint: impl Into<String>) -> Self {
    self.hints.push(hint.into());
    self
  }

  pub fn build(self) -> String {
    let mut out = String::new();
    out.push_str(&self.title);
    for line in self.lines {
      out.push('\n');
      out.push_str(&line);
    }
    for warning in self.warnings {
      out.push('\n');
      out.push_str("warning: ");
      out.push_str(&warning);
    }
    for hint in self.hints {
      out.push('\n');
      out.push_str("hint: ");
      out.push_str(&hint);
    }
    out
  }
}

#[derive(Debug, Serialize)]
struct SuccessEnvelope<'a> {
  ok: bool,
  command: &'a str,
  data: &'a Value,
}

#[derive(Debug, Serialize)]
struct ErrorEnvelope<'a> {
  ok: bool,
  command: &'a str,
  error: ErrorPayload<'a>,
}

#[derive(Debug, Serialize)]
struct ErrorPayload<'a> {
  code: &'a str,
  message: &'a str,
}

#[derive(Debug)]
struct OutputRuntime {
  mode: OutputMode,
  ui: HumanUiConfig,
  progress: Option<ProgressBar>,
}

impl Default for OutputRuntime {
  fn default() -> Self {
    Self { mode: OutputMode::Human, ui: HumanUiConfig::default(), progress: None }
  }
}

fn output_state() -> &'static Mutex<OutputRuntime> {
  static STATE: OnceLock<Mutex<OutputRuntime>> = OnceLock::new();
  STATE.get_or_init(|| Mutex::new(OutputRuntime::default()))
}

/// Render a successful command result to stdout in the selected mode.
pub fn print_success(mode: OutputMode, result: &CommandOutput) -> io::Result<()> {
  match mode {
    OutputMode::Human => {
      clear_progress();
      let ui = current_ui();
      let rendered = render_human_message(&result.human_message, ui);
      write_block(&mut io::stdout(), &rendered)
    },
    OutputMode::Json => {
      let envelope = success_envelope(result);
      write_json(&mut io::stdout(), &envelope)
    },
  }
}

/// Render a command error in human or machine-readable form.
///
/// JSON mode writes to stdout intentionally so callers can treat all command output as a single
/// stream while still relying on process exit codes for success/failure.
pub fn print_error(mode: OutputMode, command: &'static str, err: &JoyError) -> io::Result<()> {
  match mode {
    OutputMode::Human => {
      clear_progress();
      let ui = current_ui();
      let mut rendered = render_human_error(command, err, ui);
      if let Some(hint) = human_error_hint(command, err) {
        rendered.push('\n');
        rendered.push_str(&render_hint_line(&hint, ui));
      }
      write_block(&mut io::stderr(), &rendered)
    },
    OutputMode::Json => {
      let envelope = error_envelope(command, err);
      write_json(&mut io::stdout(), &envelope)
    },
  }
}

fn write_json<T: Serialize>(writer: &mut impl Write, value: &T) -> io::Result<()> {
  serde_json::to_writer_pretty(&mut *writer, value)?;
  writer.write_all(b"\n")?;
  writer.flush()
}

fn write_block(writer: &mut impl Write, text: &str) -> io::Result<()> {
  writer.write_all(text.as_bytes())?;
  if !text.ends_with('\n') {
    writer.write_all(b"\n")?;
  }
  writer.flush()
}

fn success_envelope<'a>(result: &'a CommandOutput) -> SuccessEnvelope<'a> {
  SuccessEnvelope { ok: true, command: result.command, data: &result.data }
}

fn error_envelope<'a>(command: &'a str, err: &'a JoyError) -> ErrorEnvelope<'a> {
  ErrorEnvelope {
    ok: false,
    command,
    error: ErrorPayload { code: err.code, message: &err.message },
  }
}

fn human_error_hint(command: &str, err: &JoyError) -> Option<String> {
  match err.code {
    "manifest_not_found" => Some("Run `joy init` in this directory (or `joy new <name>`).".into()),
    "toolchain_not_found" | "toolchain_probe_failed" => {
      Some("Run `joy doctor` to inspect compiler and ninja availability.".into())
    },
    "lockfile_missing" | "lockfile_stale" | "lockfile_incomplete" | "lockfile_mismatch" => {
      let example = format!("joy {command} --update-lock");
      if err.message.contains("--update-lock") {
        Some(format!("Refresh the lockfile and rerun (for example `{example}`)."))
      } else {
        Some(format!("Refresh the lockfile with `{example}`."))
      }
    },
    "offline_cache_miss" => Some(
      "Warm the cache online first (for example `joy sync`) or rerun without `--offline`.".into(),
    ),
    "offline_network_disabled" => {
      Some("Rerun without `--offline` / `--frozen`, or ensure the cache is already warm.".into())
    },
    "invalid_version_requirement" => {
      Some("Use a valid semver requirement such as `^1`, `~1.2`, or `>=1.2, <2.0`.".into())
    },
    "version_not_found" => Some(
      "Check available tags for the dependency (or relax the version range) and rerun online to refresh the mirror.".into(),
    ),
    "registry_not_configured" => {
      Some("Set `JOY_REGISTRY_DEFAULT` to the registry index git URL/path and rerun.".into())
    },
    "registry_package_not_found" => {
      Some("Verify the package exists in the configured registry index and retry.".into())
    },
    "registry_alias_unsupported" => Some(
      "This registry entry maps to a different source package ID; alias package support is deferred in the current phase cut.".into(),
    ),
    "recipe_load_failed" => {
      Some("Run `joy doctor` to validate the bundled recipe store and local environment.".into())
    },
    "dependency_not_found" if matches!(command, "remove" | "update") => {
      Some("Use `joy tree` to inspect current dependencies before editing.".into())
    },
    _ => None,
  }
}

fn current_ui() -> HumanUiConfig {
  let state = output_state().lock().expect("output state lock poisoned");
  state.ui
}

fn clear_progress() {
  let progress = {
    let mut state = output_state().lock().expect("output state lock poisoned");
    state.progress.take()
  };
  if let Some(pb) = progress {
    pb.finish_and_clear();
  }
}

fn with_progress_bar<F>(f: F)
where
  F: FnOnce(&ProgressBar),
{
  let maybe_pb = {
    let state = output_state().lock().expect("output state lock poisoned");
    state.progress.clone()
  };
  if let Some(pb) = maybe_pb {
    f(&pb);
  }
}

fn progress_runtime_enabled() -> bool {
  let state = output_state().lock().expect("output state lock poisoned");
  state.mode == OutputMode::Human && state.ui.progress_enabled && state.ui.stderr_is_tty
}

fn progress_tty_only_enabled() -> bool {
  let state = output_state().lock().expect("output state lock poisoned");
  state.mode == OutputMode::Human && state.ui.stderr_is_tty
}

fn ensure_spinner(message: &str) {
  let mut state = output_state().lock().expect("output state lock poisoned");
  if state.mode != OutputMode::Human || !state.ui.progress_enabled || !state.ui.stderr_is_tty {
    return;
  }
  let ui = state.ui;

  let pb = state.progress.get_or_insert_with(|| {
    let pb = ProgressBar::new_spinner();
    pb.enable_steady_tick(Duration::from_millis(90));
    let template = if ui.color_enabled { "{spinner:.cyan} {msg}" } else { "{spinner} {msg}" };
    let mut style = match ProgressStyle::with_template(template) {
      Ok(style) => style,
      Err(_) => ProgressStyle::default_spinner(),
    };
    style = match ui.glyph_mode {
      GlyphMode::Unicode => style.tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]),
      GlyphMode::Ascii => style.tick_strings(&["-", "\\", "|", "/"]),
    };
    pb.set_style(style);
    pb
  });
  pb.set_message(message.to_string());
}

fn render_human_message(raw: &str, ui: HumanUiConfig) -> String {
  raw
    .lines()
    .enumerate()
    .map(|(idx, line)| render_human_line(line, idx == 0, ui))
    .collect::<Vec<_>>()
    .join("\n")
}

fn render_human_error(command: &str, err: &JoyError, ui: HumanUiConfig) -> String {
  let mut lines = Vec::new();
  let mut message_lines = err.message.lines();
  let first = message_lines.next().unwrap_or_default();
  let marker = glyphs(ui).error;
  let header = format!(
    "{marker} error[{code}]: {msg}",
    code = err.code,
    msg = highlight_inline(first, ui, LineKind::Error)
  );
  lines.push(colorize(&header, ui, Style::RedBold));

  for extra in message_lines {
    let extra_rendered = highlight_inline(extra, ui, LineKind::Error);
    let prefix = glyphs(ui).detail;
    let line = format!("  {prefix} {extra_rendered}");
    lines.push(colorize(&line, ui, Style::Red));
  }

  // For verbose build/run errors, add an extra contextual hint line even before machine-coded hints.
  if matches!(command, "build" | "run") && (err.code == "build_failed" || err.code == "run_failed")
  {
    lines.push(render_info_line(
      "Compiler/Ninja diagnostics are shown above; inspect the referenced file/line diagnostics first.",
      ui,
    ));
  }

  lines.join("\n")
}

fn render_human_line(line: &str, is_title: bool, ui: HumanUiConfig) -> String {
  if let Some(rest) = line.strip_prefix("warning: ") {
    return render_warning_line(rest, ui);
  }
  if let Some(rest) = line.strip_prefix("hint: ") {
    return render_hint_line(rest, ui);
  }

  let trimmed = line.trim_start();
  let indent_len = line.len() - trimmed.len();
  let indent = &line[..indent_len];
  if let Some(rest) = trimmed.strip_prefix("- ") {
    let bullet = glyphs(ui).bullet;
    let styled_rest = highlight_inline(rest, ui, LineKind::Normal);
    let bullet_text = colorize(bullet, ui, Style::Dim);
    return format!("{indent}{bullet_text} {styled_rest}");
  }

  let highlighted = highlight_inline(line, ui, LineKind::Normal);
  if is_title && looks_structured_title(line) {
    let title_style = if looks_warning_title(line) { Style::YellowBold } else { Style::GreenBold };
    let marker = if looks_warning_title(line) { glyphs(ui).warning } else { glyphs(ui).success };
    let marker = colorize(
      marker,
      ui,
      if looks_warning_title(line) { Style::YellowBold } else { Style::GreenBold },
    );
    format!("{marker} {}", colorize(&highlighted, ui, title_style))
  } else if trimmed.ends_with(':') {
    colorize(&highlighted, ui, Style::Bold)
  } else {
    highlighted
  }
}

fn looks_structured_title(line: &str) -> bool {
  matches!(
    line.split_whitespace().next(),
    Some(
      "Created"
        | "Initialized"
        | "Build"
        | "Synchronized"
        | "Recipe"
        | "Doctor"
        | "Removed"
        | "Added"
        | "Dependency"
        | "Updated"
        | "Refreshed"
        | "No"
        | "Program"
    )
  )
}

fn looks_warning_title(line: &str) -> bool {
  let lower = line.to_ascii_lowercase();
  lower.contains("reported issues") || lower.starts_with("no ")
}

fn render_warning_line(text: &str, ui: HumanUiConfig) -> String {
  let marker = colorize(glyphs(ui).warning, ui, Style::YellowBold);
  let body = highlight_inline(text, ui, LineKind::Warning);
  format!("{marker} {}", colorize(&body, ui, Style::Yellow))
}

fn render_hint_line(text: &str, ui: HumanUiConfig) -> String {
  let marker = colorize(glyphs(ui).hint, ui, Style::CyanBold);
  let body = highlight_inline(text, ui, LineKind::Hint);
  format!("{marker} {}", colorize(&body, ui, Style::Cyan))
}

fn render_info_line(text: &str, ui: HumanUiConfig) -> String {
  let marker = colorize(glyphs(ui).info, ui, Style::BlueBold);
  format!("{marker} {}", colorize(&highlight_inline(text, ui, LineKind::Normal), ui, Style::Blue))
}

#[derive(Debug, Clone, Copy)]
enum LineKind {
  Normal,
  Warning,
  Hint,
  Error,
}

fn highlight_inline(text: &str, ui: HumanUiConfig, kind: LineKind) -> String {
  let mut out = String::with_capacity(text.len() + 8);
  let mut in_code = false;
  let mut buf = String::new();
  for ch in text.chars() {
    if ch == '`' {
      if in_code {
        out.push_str(&colorize(&buf, ui, code_style_for(kind)));
        buf.clear();
      }
      in_code = !in_code;
      out.push('`');
      continue;
    }
    if in_code {
      buf.push(ch);
    } else {
      out.push(ch);
    }
  }
  if !buf.is_empty() {
    if in_code {
      out.push_str(&colorize(&buf, ui, code_style_for(kind)));
    } else {
      out.push_str(&buf);
    }
  }
  out
}

fn code_style_for(kind: LineKind) -> Style {
  match kind {
    LineKind::Error => Style::RedBold,
    LineKind::Warning => Style::YellowBold,
    LineKind::Hint => Style::CyanBold,
    LineKind::Normal => Style::BlueBold,
  }
}

#[derive(Debug, Clone, Copy)]
enum Style {
  GreenBold,
  YellowBold,
  Yellow,
  CyanBold,
  Cyan,
  BlueBold,
  Blue,
  RedBold,
  Red,
  Bold,
  Dim,
}

fn colorize(text: &str, ui: HumanUiConfig, style: Style) -> String {
  if !ui.color_enabled {
    return text.to_string();
  }
  let code = match style {
    Style::GreenBold => "1;32",
    Style::YellowBold => "1;33",
    Style::Yellow => "33",
    Style::CyanBold => "1;36",
    Style::Cyan => "36",
    Style::BlueBold => "1;34",
    Style::Blue => "34",
    Style::RedBold => "1;31",
    Style::Red => "31",
    Style::Bold => "1",
    Style::Dim => "2",
  };
  format!("\x1b[{code}m{text}\x1b[0m")
}

struct GlyphSet {
  success: &'static str,
  error: &'static str,
  warning: &'static str,
  hint: &'static str,
  info: &'static str,
  stage: &'static str,
  detail: &'static str,
  bullet: &'static str,
}

fn glyphs(ui: HumanUiConfig) -> GlyphSet {
  match ui.glyph_mode {
    GlyphMode::Unicode => GlyphSet {
      success: "✓",
      error: "✖",
      warning: "⚠",
      hint: "ℹ",
      info: "•",
      stage: "▸",
      detail: "•",
      bullet: "•",
    },
    GlyphMode::Ascii => GlyphSet {
      success: "OK",
      error: "ERR",
      warning: "WARN",
      hint: "HINT",
      info: "*",
      stage: ">>",
      detail: "-",
      bullet: "-",
    },
  }
}

fn format_progress_stage_line(message: &str, ui: HumanUiConfig) -> String {
  let prefix = colorize(glyphs(ui).stage, ui, Style::CyanBold);
  format!("{prefix} {}", highlight_inline(message, ui, LineKind::Normal))
}

fn format_progress_detail_line(message: &str, ui: HumanUiConfig) -> String {
  let prefix = colorize(glyphs(ui).detail, ui, Style::Dim);
  format!("  {prefix} {}", highlight_inline(message, ui, LineKind::Normal))
}

fn write_stderr_line(line: &str) -> io::Result<()> {
  let mut stderr = io::stderr();
  stderr.write_all(line.as_bytes())?;
  stderr.write_all(b"\n")?;
  stderr.flush()
}

/// Emit a human-mode stage/status line to stderr.
pub fn progress_stage(message: &str) {
  let ui = current_ui();
  if progress_runtime_enabled() {
    ensure_spinner(message);
    return;
  }
  let should_emit = {
    let state = output_state().lock().expect("output state lock poisoned");
    state.mode == OutputMode::Human
  };
  if should_emit {
    let _ = write_stderr_line(&format_progress_stage_line(message, ui));
  }
}

/// Emit a human-mode detail line to stderr.
pub fn progress_detail(message: &str) {
  let ui = current_ui();
  if progress_runtime_enabled() {
    let line = format_progress_detail_line(message, ui);
    with_progress_bar(|pb| {
      pb.println(line);
    });
    return;
  }
  let should_emit = {
    let state = output_state().lock().expect("output state lock poisoned");
    state.mode == OutputMode::Human
  };
  if should_emit {
    let _ = write_stderr_line(&format_progress_detail_line(message, ui));
  }
}

/// Emit a human-mode detail line to stderr only when stderr is a TTY.
pub fn progress_detail_tty(message: &str) {
  if progress_tty_only_enabled() || io::stderr().is_terminal() {
    progress_detail(message);
  }
}

#[cfg(test)]
mod tests {
  use serde_json::json;

  use super::{
    GlyphMode, HumanMessageBuilder, HumanUiConfig, error_envelope, format_progress_detail_line,
    format_progress_stage_line, human_error_hint, render_human_error, render_human_message,
    success_envelope,
  };
  use crate::commands::CommandOutput;
  use crate::error::JoyError;

  fn plain_ui() -> HumanUiConfig {
    HumanUiConfig {
      color_enabled: false,
      progress_enabled: false,
      glyph_mode: GlyphMode::Ascii,
      stderr_is_tty: false,
    }
  }

  fn styled_ui() -> HumanUiConfig {
    HumanUiConfig {
      color_enabled: true,
      progress_enabled: true,
      glyph_mode: GlyphMode::Unicode,
      stderr_is_tty: true,
    }
  }

  #[test]
  fn json_error_envelope_shape_is_stable() {
    let err = JoyError::new("build", "toolchain_not_found", "No compiler found", 1);
    let value = serde_json::to_value(error_envelope("build", &err)).expect("serialize envelope");

    assert_eq!(
      value,
      json!({
          "ok": false,
          "command": "build",
          "error": {
              "code": "toolchain_not_found",
              "message": "No compiler found"
          }
      })
    );
  }

  #[test]
  fn json_success_envelope_shape_is_stable() {
    let result = CommandOutput::new("recipe-check", "ok", json!({"recipe_count": 9}));
    let value = serde_json::to_value(success_envelope(&result)).expect("serialize envelope");
    assert_eq!(
      value,
      json!({
        "ok": true,
        "command": "recipe-check",
        "data": {
          "recipe_count": 9
        }
      })
    );
  }

  #[test]
  fn human_message_builder_renders_lines_warnings_and_hints() {
    let msg = HumanMessageBuilder::new("Done")
      .kv("project", "demo")
      .line("- mode: debug")
      .warning("joy.lock may be stale")
      .hint("rerun `joy build --update-lock`")
      .build();
    assert_eq!(
      msg,
      "Done\n- project: demo\n- mode: debug\nwarning: joy.lock may be stale\nhint: rerun `joy build --update-lock`"
    );
  }

  #[test]
  fn lockfile_errors_get_human_hint() {
    let err = JoyError::new("build", "lockfile_stale", "joy.lock manifest hash does not match", 1);
    let hint = human_error_hint("build", &err).expect("hint");
    assert!(hint.contains("joy build --update-lock"));
  }

  #[test]
  fn human_renderer_applies_semantic_markers_and_preserves_bullets() {
    let msg = HumanMessageBuilder::new("Build finished")
      .kv("binary", "/tmp/demo")
      .warning("something happened")
      .hint("run `joy doctor`")
      .build();
    let rendered = render_human_message(&msg, plain_ui());
    assert!(rendered.contains("OK Build finished"));
    assert!(rendered.contains("- binary: /tmp/demo"));
    assert!(rendered.contains("WARN something happened"));
    assert!(rendered.contains("HINT run `joy doctor`"));
  }

  #[test]
  fn human_renderer_styles_with_ansi_when_enabled() {
    let rendered = render_human_message("Build finished\n- binary: `foo`", styled_ui());
    assert!(rendered.contains("\u{1b}["));
    assert!(rendered.contains("✓"));
  }

  #[test]
  fn human_error_renderer_highlights_backticked_segments() {
    let err = JoyError::new(
      "build",
      "build_failed",
      "ninja build failed\nSee `/tmp/demo/.joy/build/ninja.last.stderr.log`",
      1,
    );
    let rendered = render_human_error("build", &err, styled_ui());
    assert!(rendered.contains("✖ error[build_failed]"));
    assert!(rendered.contains("`\u{1b}[1;31m/tmp/demo/.joy/build/ninja.last.stderr.log\u{1b}[0m`"));
  }

  #[test]
  fn progress_formatters_use_glyph_sets() {
    let ascii = format_progress_stage_line("Starting build", plain_ui());
    assert!(ascii.starts_with(">> "));
    let unicode = format_progress_detail_line("Fetching `fmtlib/fmt`", styled_ui());
    assert!(unicode.contains("•"));
  }
}
