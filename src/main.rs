mod shell_init;

use chrono::{DateTime, Duration, Utc};
use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    execute, queue,
    style::{Attribute, Print, SetAttribute},
    terminal::{self, Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen},
};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::env;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

#[cfg(windows)]
use winapi::um::wincon::{SetConsoleCP, SetConsoleOutputCP};
#[cfg(windows)]
const CP_UTF8: u32 = 65001;

#[cfg(windows)]
fn setup_console() {
    unsafe {
        SetConsoleCP(CP_UTF8);
        SetConsoleOutputCP(CP_UTF8);
    }
}

#[cfg(not(windows))]
fn setup_console() {}

#[cfg(windows)]
const PATH_SEP: char = '\\';
#[cfg(not(windows))]
const PATH_SEP: char = '/';

const MAX_HISTORY: usize = 1000;
const MAX_UNDO_STACK: usize = 50;
const HISTORY_LIST_LIMIT: usize = 20;
const MISSING_HISTORY_RETENTION_DAYS: i64 = 90;
const HISTORY_PRUNE_INTERVAL_HOURS: i64 = 24;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct HistoryEntry {
    path: String,
    last_visited: DateTime<Utc>,
    visit_count: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DirectoryCandidate {
    name: String,
    path: PathBuf,
}

#[derive(Debug, Serialize, Deserialize)]
struct AppState {
    history: Vec<HistoryEntry>,
    undo_stack: VecDeque<String>,
    redo_stack: VecDeque<String>,
    current_dir: Option<String>,
    #[serde(default)]
    last_pruned_at: Option<DateTime<Utc>>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            history: Vec::new(),
            undo_stack: VecDeque::new(),
            redo_stack: VecDeque::new(),
            current_dir: None,
            last_pruned_at: None,
        }
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct Aliases {
    #[serde(flatten)]
    map: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
enum Ranking {
    #[default]
    Recent,
    Frecency,
    Frequency,
}

impl Ranking {
    fn parse(value: &str) -> Option<Self> {
        match value.to_ascii_lowercase().as_str() {
            "recent" => Some(Self::Recent),
            "frecency" => Some(Self::Frecency),
            "frequency" => Some(Self::Frequency),
            _ => None,
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Recent => "recent",
            Self::Frecency => "frecency",
            Self::Frequency => "frequency",
        }
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct Config {
    #[serde(default)]
    excludes: Vec<String>,
    #[serde(default)]
    ranking: Ranking,
    #[serde(default)]
    fzf_opts: Option<String>,
}

fn get_config_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".config")
        .join("j")
}

fn get_state_path() -> PathBuf {
    get_config_dir().join("state.json")
}

fn get_aliases_path() -> PathBuf {
    get_config_dir().join("aliases.json")
}

fn get_config_path() -> PathBuf {
    get_config_dir().join("config.json")
}

fn load_state() -> AppState {
    let path = get_state_path();
    fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .map(|mut state: AppState| {
            if !history_is_sorted_by_recency(&state.history) {
                sort_history_by_recency(&mut state.history);
            }
            state
        })
        .unwrap_or_default()
}

fn save_state(state: &mut AppState) -> io::Result<()> {
    let now = Utc::now();
    if history_prune_is_due(state, now) {
        prune_stale_missing_history(state, now);
        state.last_pruned_at = Some(now);
    }
    fs::create_dir_all(get_config_dir())?;
    let path = get_state_path();
    let json = serde_json::to_string_pretty(state)?;
    fs::write(path, json)
}

fn load_aliases() -> Aliases {
    let path = get_aliases_path();
    fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn save_aliases(aliases: &Aliases) -> io::Result<()> {
    fs::create_dir_all(get_config_dir())?;
    let path = get_aliases_path();
    let json = serde_json::to_string_pretty(aliases)?;
    fs::write(path, json)
}

fn load_config() -> Config {
    let path = get_config_path();
    fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn save_config(config: &Config) -> io::Result<()> {
    fs::create_dir_all(get_config_dir())?;
    let path = get_config_path();
    let json = serde_json::to_string_pretty(config)?;
    fs::write(path, json)
}

/// Check if a path matches any exclude pattern
fn is_excluded(path: &str, config: &Config) -> bool {
    let path_lower = path.to_lowercase();
    for pattern in &config.excludes {
        let pattern_lower = pattern.to_lowercase();
        // Simple glob matching: support * as wildcard
        if pattern_lower.contains('*') {
            let parts: Vec<&str> = pattern_lower.split('*').collect();
            let mut pos = 0;
            let mut matched = true;
            for (i, part) in parts.iter().enumerate() {
                if part.is_empty() {
                    continue;
                }
                if let Some(found_pos) = path_lower[pos..].find(part) {
                    if i == 0 && found_pos != 0 && !pattern_lower.starts_with('*') {
                        matched = false;
                        break;
                    }
                    pos += found_pos + part.len();
                } else {
                    matched = false;
                    break;
                }
            }
            if matched {
                return true;
            }
        } else {
            // Exact substring match
            if path_lower.contains(&pattern_lower) {
                return true;
            }
        }
    }
    false
}

fn split_option_string(value: &str) -> Result<Vec<String>, String> {
    let mut args = Vec::new();
    let mut current = String::new();
    let mut chars = value.chars().peekable();
    let mut quote = None;

    while let Some(ch) = chars.next() {
        match (quote, ch) {
            (Some(active), ch) if ch == active => quote = None,
            (Some('"'), '\\') => {
                if matches!(chars.peek(), Some('"' | '\\')) {
                    current.push(chars.next().unwrap());
                } else {
                    current.push('\\');
                }
            }
            (Some(_), ch) => current.push(ch),
            (None, '"' | '\'') => quote = Some(ch),
            (None, '\\') => {
                if matches!(chars.peek(), Some(next) if next.is_whitespace() || matches!(next, '"' | '\'' | '\\'))
                {
                    current.push(chars.next().unwrap());
                } else {
                    current.push('\\');
                }
            }
            (None, ch) if ch.is_whitespace() => {
                if !current.is_empty() {
                    args.push(std::mem::take(&mut current));
                }
            }
            (None, ch) => current.push(ch),
        }
    }

    if quote.is_some() {
        return Err("unterminated quote".to_string());
    }
    if !current.is_empty() {
        args.push(current);
    }
    Ok(args)
}

fn configured_fzf_args(config: &Config) -> Vec<String> {
    let value = env::var("J_FZF_OPTS")
        .ok()
        .or_else(|| config.fzf_opts.clone());
    match value {
        Some(value) => match split_option_string(&value) {
            Ok(args) => args,
            Err(error) => {
                eprintln!("Ignoring invalid fzf options: {}", error);
                Vec::new()
            }
        },
        None => Vec::new(),
    }
}

/// Run fzf with candidates and return selected path
fn run_fzf(candidates: &[String], query: &str, config: &Config) -> Option<String> {
    let fzf_cmd = if cfg!(windows) { "fzf.exe" } else { "fzf" };
    let mut args = vec![
        "--height".to_string(),
        "40%".to_string(),
        "--reverse".to_string(),
    ];
    args.extend(configured_fzf_args(config));
    args.push("--query".to_string());
    args.push(query.to_string());
    
    let mut child = match Command::new(fzf_cmd)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
    {
        Ok(child) => child,
        Err(_) => {
            eprintln!("fzf not found. Install fzf for interactive selection.");
            return None;
        }
    };
    
    if let Some(mut stdin) = child.stdin.take() {
        for candidate in candidates {
            writeln!(stdin, "{}", candidate).ok();
        }
    }
    
    let output = child.wait_with_output().ok()?;
    if output.status.success() {
        let selected = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !selected.is_empty() {
            return Some(selected);
        }
    }
    None
}

fn direct_directory_candidates(base: &Path) -> io::Result<Vec<DirectoryCandidate>> {
    let mut candidates = Vec::new();

    for entry in fs::read_dir(base)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            candidates.push(DirectoryCandidate {
                name: entry.file_name().to_string_lossy().into_owned(),
                path,
            });
        }
    }

    candidates.sort_by(|left, right| {
        left.name
            .to_lowercase()
            .cmp(&right.name.to_lowercase())
            .then_with(|| left.name.cmp(&right.name))
    });
    Ok(candidates)
}

#[derive(Debug)]
struct DirectoryBrowser {
    current: PathBuf,
    candidates: Vec<DirectoryCandidate>,
    selected: usize,
}

impl DirectoryBrowser {
    fn new(start: &Path) -> io::Result<Self> {
        Ok(Self {
            current: start.to_path_buf(),
            candidates: direct_directory_candidates(start)?,
            selected: 0,
        })
    }

    fn select_previous(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
    }

    fn select_next(&mut self) {
        if self.selected + 1 < self.candidates.len() {
            self.selected += 1;
        }
    }

    fn selected_path(&self) -> Option<PathBuf> {
        self.candidates
            .get(self.selected)
            .map(|candidate| candidate.path.clone())
    }

    fn current_path(&self) -> PathBuf {
        self.current.clone()
    }

    fn select_prefix(&mut self, prefix: &str) -> bool {
        let prefix = prefix.to_lowercase();
        if let Some(index) = self
            .candidates
            .iter()
            .position(|candidate| candidate.name.to_lowercase().starts_with(&prefix))
        {
            self.selected = index;
            true
        } else {
            false
        }
    }

    fn move_into_selected(&mut self) -> io::Result<bool> {
        let Some(next) = self.selected_path() else {
            return Ok(false);
        };
        let candidates = direct_directory_candidates(&next)?;
        self.current = next;
        self.candidates = candidates;
        self.selected = 0;
        Ok(true)
    }

    fn selected_or_current_path(&self) -> PathBuf {
        self.selected_path()
            .unwrap_or_else(|| self.current_path())
    }

    fn move_to_parent(&mut self) -> io::Result<bool> {
        let Some(parent) = self.current.parent().map(Path::to_path_buf) else {
            return Ok(false);
        };
        let previous = self.current.clone();
        let candidates = direct_directory_candidates(&parent)?;
        self.current = parent;
        self.candidates = candidates;
        self.selected = self
            .candidates
            .iter()
            .position(|candidate| candidate.path == previous)
            .unwrap_or(0);
        Ok(true)
    }
}

#[cfg(windows)]
fn open_terminal_output() -> io::Result<File> {
    OpenOptions::new().write(true).open("CONOUT$")
}

#[cfg(not(windows))]
fn open_terminal_output() -> io::Result<File> {
    OpenOptions::new().read(true).write(true).open("/dev/tty")
}

struct SelectorTerminal {
    output: File,
}

impl SelectorTerminal {
    fn enter() -> io::Result<Self> {
        let mut output = open_terminal_output()?;
        terminal::enable_raw_mode()?;
        if let Err(error) = execute!(output, EnterAlternateScreen, cursor::Hide) {
            terminal::disable_raw_mode().ok();
            return Err(error);
        }
        Ok(Self { output })
    }

    fn output(&mut self) -> &mut File {
        &mut self.output
    }
}

impl Drop for SelectorTerminal {
    fn drop(&mut self) {
        execute!(self.output, cursor::Show, LeaveAlternateScreen).ok();
        terminal::disable_raw_mode().ok();
    }
}

fn history_list_candidates(state: &AppState, keyword: &str) -> Vec<String> {
    let keyword = keyword.to_lowercase();
    state
        .history
        .iter()
        .rev()
        .filter(|entry| {
            keyword.is_empty()
                || split_path(&entry.path)
                    .last()
                    .is_some_and(|name| name.to_lowercase().contains(&keyword))
        })
        .take(HISTORY_LIST_LIMIT)
        .map(|entry| entry.path.clone())
        .collect()
}

fn remove_history_path(state: &mut AppState, path: &str) -> bool {
    let before_len = state.history.len();
    state
        .history
        .retain(|entry| !entry.path.eq_ignore_ascii_case(path));
    state.history.len() < before_len
}

fn truncate_terminal_line(text: &str, width: usize) -> String {
    let char_count = text.chars().count();
    if char_count <= width {
        return text.to_string();
    }
    if width <= 1 {
        return "…".chars().take(width).collect();
    }
    let mut result: String = text.chars().take(width - 1).collect();
    result.push('…');
    result
}

fn render_history_selector(
    output: &mut impl Write,
    candidates: &[String],
    selected: usize,
    keyword: &str,
    status: &str,
) -> io::Result<()> {
    let (width, height) = terminal::size().unwrap_or((80, 24));
    let list_height = usize::from(height.saturating_sub(4).max(1));
    let offset = selected.saturating_add(1).saturating_sub(list_height);

    queue!(
        output,
        cursor::MoveTo(0, 0),
        Clear(ClearType::All),
        SetAttribute(Attribute::Bold),
        Print(format!("History ({})", candidates.len())),
        SetAttribute(Attribute::Reset)
    )?;
    if !keyword.is_empty() {
        queue!(output, Print(format!("  filter: {}", keyword)))?;
    }
    queue!(
        output,
        Print("\r\n↑/↓: select  Enter: move  x: delete  Esc/q: cancel\r\n"),
        Print(truncate_terminal_line(status, usize::from(width))),
        Print("\r\n")
    )?;

    if candidates.is_empty() {
        queue!(output, Print("No matching history entries."))?;
    } else {
        for (index, path) in candidates
            .iter()
            .enumerate()
            .skip(offset)
            .take(list_height)
        {
            let missing = if Path::new(path).is_dir() {
                ""
            } else {
                "  [missing]"
            };
            let line = truncate_terminal_line(
                &format!("{}{}{}", if index == selected { "> " } else { "  " }, path, missing),
                usize::from(width),
            );
            if index == selected {
                queue!(
                    output,
                    SetAttribute(Attribute::Reverse),
                    Print(line),
                    SetAttribute(Attribute::Reset)
                )?;
            } else {
                queue!(output, Print(line))?;
            }
            queue!(output, Print("\r\n"))?;
        }
    }

    output.flush()
}

fn render_directory_browser(
    output: &mut impl Write,
    browser: &DirectoryBrowser,
    prefix: &str,
    status: &str,
) -> io::Result<()> {
    let (width, height) = terminal::size().unwrap_or((80, 24));
    let list_height = usize::from(height.saturating_sub(1).max(1));
    let offset = browser
        .selected
        .saturating_add(1)
        .saturating_sub(list_height);

    queue!(
        output,
        cursor::MoveTo(0, 0),
        Clear(ClearType::All),
        Print(truncate_terminal_line(
            &if prefix.is_empty() {
                status.to_string()
            } else if status.is_empty() {
                format!("prefix: {}", prefix)
            } else {
                format!("prefix: {}  {}", prefix, status)
            },
            usize::from(width)
        )),
        Print("\r\n")
    )?;

    if browser.candidates.is_empty() {
        queue!(output, Print("No subdirectories."))?;
    } else {
        for (index, candidate) in browser
            .candidates
            .iter()
            .enumerate()
            .skip(offset)
            .take(list_height)
        {
            let line = truncate_terminal_line(
                &format!(
                    "{}{}",
                    if index == browser.selected { "> " } else { "  " },
                    candidate.name
                ),
                usize::from(width),
            );
            if index == browser.selected {
                queue!(
                    output,
                    SetAttribute(Attribute::Reverse),
                    Print(line),
                    SetAttribute(Attribute::Reset)
                )?;
            } else {
                queue!(output, Print(line))?;
            }
            queue!(output, Print("\r\n"))?;
        }
    }

    output.flush()
}

fn browse_directories(start: &Path) -> io::Result<Option<PathBuf>> {
    let mut browser = DirectoryBrowser::new(start)?;
    let mut prefix = String::new();
    let mut status = String::new();
    let mut terminal = SelectorTerminal::enter()?;

    loop {
        render_directory_browser(terminal.output(), &browser, &prefix, &status)?;

        if let Event::Key(key) = event::read()? {
            if key.kind == KeyEventKind::Release {
                continue;
            }
            status.clear();

            if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
                return Ok(None);
            }

            match key.code {
                KeyCode::Up => {
                    prefix.clear();
                    browser.select_previous();
                }
                KeyCode::Down => {
                    prefix.clear();
                    browser.select_next();
                }
                KeyCode::Right => match browser.move_into_selected() {
                    Ok(true) => prefix.clear(),
                    Ok(false) => status = "No subdirectory selected.".to_string(),
                    Err(error) => status = format!("Cannot open directory: {}", error),
                },
                KeyCode::Left => match browser.move_to_parent() {
                    Ok(true) => prefix.clear(),
                    Ok(false) => status = "Already at the filesystem root.".to_string(),
                    Err(error) => status = format!("Cannot open parent directory: {}", error),
                },
                KeyCode::Enter => {
                    return Ok(Some(browser.selected_or_current_path()));
                }
                KeyCode::Backspace => {
                    prefix.pop();
                    if !prefix.is_empty() {
                        browser.select_prefix(&prefix);
                    }
                }
                KeyCode::Char('.') => {
                    return Ok(Some(browser.current_path()));
                }
                KeyCode::Char(character)
                    if !key.modifiers.intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
                {
                    prefix.push(character);
                    if !browser.select_prefix(&prefix) {
                        status = "No directory starts with this prefix.".to_string();
                    }
                }
                KeyCode::Esc => return Ok(None),
                _ => {}
            }
        }
    }
}

fn select_history(state: &mut AppState, keyword: &str) -> io::Result<Option<String>> {
    let mut candidates = history_list_candidates(state, keyword);
    let mut selected = 0;
    let mut status = String::new();
    let mut terminal = SelectorTerminal::enter()?;

    loop {
        render_history_selector(terminal.output(), &candidates, selected, keyword, &status)?;

        if let Event::Key(key) = event::read()? {
            if key.kind == KeyEventKind::Release {
                continue;
            }
            status.clear();

            if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
                return Ok(None);
            }

            match key.code {
                KeyCode::Up if !candidates.is_empty() => {
                    selected = if selected == 0 {
                        candidates.len() - 1
                    } else {
                        selected - 1
                    };
                }
                KeyCode::Down if !candidates.is_empty() => {
                    selected = (selected + 1) % candidates.len();
                }
                KeyCode::Enter if !candidates.is_empty() => {
                    let path = &candidates[selected];
                    if Path::new(path).is_dir() {
                        return Ok(Some(path.clone()));
                    }
                    status = "Directory not found. Press x to delete this entry.".to_string();
                }
                KeyCode::Char('x' | 'X') if !candidates.is_empty() => {
                    let path = candidates[selected].clone();
                    if remove_history_path(state, &path) {
                        save_state(state)?;
                    }
                    candidates = history_list_candidates(state, keyword);
                    if !candidates.is_empty() {
                        selected = selected.min(candidates.len() - 1);
                    } else {
                        selected = 0;
                    }
                    status = format!("Deleted: {}", path);
                }
                KeyCode::Esc | KeyCode::Char('q' | 'Q') => return Ok(None),
                _ => {}
            }
        }
    }
}

fn frecency_score(entry: &HistoryEntry, now: DateTime<Utc>) -> u64 {
    let age_seconds = now
        .signed_duration_since(entry.last_visited)
        .num_seconds()
        .max(0);
    let count = u64::from(entry.visit_count);
    if age_seconds < 60 * 60 {
        count * 16
    } else if age_seconds < 24 * 60 * 60 {
        count * 8
    } else if age_seconds < 7 * 24 * 60 * 60 {
        count * 2
    } else {
        count
    }
}

fn ranked_history_entries<'a>(
    state: &'a AppState,
    config: &Config,
    now: DateTime<Utc>,
) -> Vec<&'a HistoryEntry> {
    let mut entries: Vec<&HistoryEntry> = state.history.iter().collect();
    let tie_break = |a: &&HistoryEntry, b: &&HistoryEntry| {
        b.last_visited
            .cmp(&a.last_visited)
            .then_with(|| a.path.cmp(&b.path))
    };
    match config.ranking {
        Ranking::Recent => {
            // History is kept in ascending last_visited order, so reversing is enough;
            // only entries sharing a timestamp need the path tie-breaker.
            entries.reverse();
            for group in entries.chunk_by_mut(|a, b| a.last_visited == b.last_visited) {
                group.sort_by(|a, b| a.path.cmp(&b.path));
            }
        }
        Ranking::Frecency => entries.sort_by(|a, b| {
            frecency_score(b, now)
                .cmp(&frecency_score(a, now))
                .then_with(|| tie_break(a, b))
        }),
        Ranking::Frequency => entries.sort_by(|a, b| {
            b.visit_count
                .cmp(&a.visit_count)
                .then_with(|| tie_break(a, b))
        }),
    }
    entries
}

/// Search history and return all matching candidates
fn search_history_all(state: &AppState, keyword: &str, config: &Config) -> Vec<String> {
    let tokens: Vec<String> = split_path(keyword)
        .iter()
        .map(|s| s.to_lowercase())
        .collect();
    
    let mut results = Vec::new();
    let ranked = ranked_history_entries(state, config, Utc::now());
    
    if tokens.is_empty() {
        // Return all valid directories
        for entry in ranked.iter().copied() {
            if !is_excluded(&entry.path, config) && Path::new(&entry.path).is_dir() {
                results.push(entry.path.clone());
            }
        }
        return results;
    }
    
    let last_token = tokens.last().unwrap();
    
    // Exact match on last directory name
    for entry in ranked.iter().copied() {
        if is_excluded(&entry.path, config) {
            continue;
        }
        let path = Path::new(&entry.path);
        if let Some(dir_name) = path.file_name().and_then(|n| n.to_str()) {
            if dir_name.to_lowercase() == *last_token {
                if tokens.len() > 1 {
                    let path_lower = entry.path.to_lowercase();
                    let path_parts: Vec<&str> = split_path(&path_lower);
                    let other_tokens = &tokens[..tokens.len()-1];
                    if tokens_match_in_order(
                        &path_parts[..path_parts.len().saturating_sub(1)],
                        other_tokens,
                    ) && Path::new(&entry.path).is_dir()
                    {
                        results.push(entry.path.clone());
                    }
                } else if Path::new(&entry.path).is_dir() {
                    results.push(entry.path.clone());
                }
            }
        }
    }
    
    // Partial match on last directory name
    for entry in ranked.iter().copied() {
        if is_excluded(&entry.path, config) {
            continue;
        }
        if results.contains(&entry.path) {
            continue;
        }
        let path = Path::new(&entry.path);
        if let Some(dir_name) = path.file_name().and_then(|n| n.to_str()) {
            if dir_name.to_lowercase().contains(last_token.as_str()) {
                if tokens.len() > 1 {
                    let path_lower = entry.path.to_lowercase();
                    let path_parts: Vec<&str> = split_path(&path_lower);
                    let other_tokens = &tokens[..tokens.len()-1];
                    if tokens_match_in_order(
                        &path_parts[..path_parts.len().saturating_sub(1)],
                        other_tokens,
                    ) && Path::new(&entry.path).is_dir()
                    {
                        results.push(entry.path.clone());
                    }
                } else if Path::new(&entry.path).is_dir() {
                    results.push(entry.path.clone());
                }
            }
        }
    }
    
    results
}

fn normalize_path_separator(path: &str) -> String {
    #[cfg(windows)]
    {
        path.replace('/', "\\")
    }
    #[cfg(not(windows))]
    {
        path.replace('\\', "/")
    }
}

fn split_path(path: &str) -> Vec<&str> {
    path.split(&['/', '\\'][..]).filter(|s| !s.is_empty()).collect()
}

#[cfg(windows)]
fn is_absolute_path(path: &str) -> bool {
    let normalized = normalize_path_separator(path);
    normalized.starts_with('\\')
        || (normalized.len() >= 2
            && normalized.chars().next().map(|c| c.is_ascii_alphabetic()).unwrap_or(false)
            && normalized.chars().nth(1) == Some(':'))
}

#[cfg(not(windows))]
fn is_absolute_path(path: &str) -> bool {
    path.starts_with('/')
}

fn is_relative_path(path: &str) -> bool {
    let normalized = normalize_path_separator(path);
    normalized.starts_with(&format!("..{}", PATH_SEP)) || normalized.starts_with(&format!(".{}", PATH_SEP))
}

#[cfg(windows)]
fn extract_drive(path: &str) -> Option<(char, &str)> {
    let chars: Vec<char> = path.chars().collect();
    if chars.len() >= 2 && chars[0].is_ascii_alphabetic() && chars[1] == ':' {
        let rest = if chars.len() > 2 { &path[2..] } else { "" };
        Some((chars[0].to_ascii_uppercase(), rest))
    } else {
        None
    }
}

#[cfg(not(windows))]
#[allow(dead_code)]
fn extract_drive(_path: &str) -> Option<(char, &str)> {
    None
}

fn expand_home(path: &str) -> Option<PathBuf> {
    if path.starts_with('~') {
        dirs::home_dir().map(|home| {
            if path.len() > 1 {
                let rest = normalize_path_separator(&path[1..]);
                let rest = rest.trim_start_matches(PATH_SEP);
                home.join(rest)
            } else {
                home
            }
        })
    } else {
        None
    }
}

fn try_local_path(keyword: &str) -> Option<PathBuf> {
    let current = env::current_dir().ok()?;
    let tokens = split_path(keyword);
    
    if tokens.is_empty() {
        return None;
    }
    
    let mut path = current;
    for token in &tokens {
        path = path.join(token);
    }
    
    if path.is_dir() {
        Some(path)
    } else {
        None
    }
}

/// Check if tokens appear in order within path parts
/// e.g., tokens ["first", "one"] matches path "/home/first/project/one"
/// but not "/home/one/project/first" (wrong order)
fn tokens_match_in_order(path_parts: &[&str], tokens: &[String]) -> bool {
    if tokens.is_empty() {
        return true;
    }
    
    let mut last_found_index: Option<usize> = None;
    
    for token in tokens {
        // Find the token in path_parts, starting after the last found position
        let start_index = last_found_index.map(|i| i + 1).unwrap_or(0);
        
        let found = path_parts[start_index..]
            .iter()
            .position(|part| part.contains(token.as_str()))
            .map(|pos| pos + start_index);
        
        match found {
            Some(idx) => {
                last_found_index = Some(idx);
            }
            None => {
                return false;
            }
        }
    }
    
    true
}

fn search_history(state: &AppState, keyword: &str, config: &Config) -> Option<String> {
    let tokens: Vec<String> = split_path(keyword)
        .iter()
        .map(|s| s.to_lowercase())
        .collect();
    
    if tokens.is_empty() {
        return None;
    }
    
    let last_token = tokens.last().unwrap();
    let ranked = ranked_history_entries(state, config, Utc::now());
    
    // First pass: exact match on last directory name, with order-aware matching
    for entry in ranked.iter().copied() {
        if is_excluded(&entry.path, config) {
            continue;
        }
        let path = Path::new(&entry.path);
        if let Some(dir_name) = path.file_name().and_then(|n| n.to_str()) {
            if dir_name.to_lowercase() == *last_token {
                if tokens.len() > 1 {
                    let path_lower = entry.path.to_lowercase();
                    let path_parts: Vec<&str> = split_path(&path_lower);
                    let other_tokens = &tokens[..tokens.len()-1];
                    
                    // Check if other tokens appear in order before the last token
                    if tokens_match_in_order(
                        &path_parts[..path_parts.len().saturating_sub(1)],
                        other_tokens,
                    ) && Path::new(&entry.path).is_dir()
                    {
                        return Some(entry.path.clone());
                    }
                } else if Path::new(&entry.path).is_dir() {
                    return Some(entry.path.clone());
                }
            }
        }
    }
    
    // Second pass: partial match on last directory name, with order-aware matching
    for entry in ranked.iter().copied() {
        if is_excluded(&entry.path, config) {
            continue;
        }
        let path = Path::new(&entry.path);
        if let Some(dir_name) = path.file_name().and_then(|n| n.to_str()) {
            if dir_name.to_lowercase().contains(last_token.as_str()) {
                if tokens.len() > 1 {
                    let path_lower = entry.path.to_lowercase();
                    let path_parts: Vec<&str> = split_path(&path_lower);
                    let other_tokens = &tokens[..tokens.len()-1];
                    
                    // Check if other tokens appear in order before the last token
                    if tokens_match_in_order(
                        &path_parts[..path_parts.len().saturating_sub(1)],
                        other_tokens,
                    ) && Path::new(&entry.path).is_dir()
                    {
                        return Some(entry.path.clone());
                    }
                } else if Path::new(&entry.path).is_dir() {
                    return Some(entry.path.clone());
                }
            }
        }
    }
    
    None
}

fn prune_stale_missing_history(state: &mut AppState, now: DateTime<Utc>) -> usize {
    let before_len = state.history.len();
    let retention = Duration::days(MISSING_HISTORY_RETENTION_DAYS);
    state.history.retain(|entry| {
        now.signed_duration_since(entry.last_visited) < retention
            || Path::new(&entry.path).exists()
    });
    before_len - state.history.len()
}

fn history_prune_is_due(state: &AppState, now: DateTime<Utc>) -> bool {
    state.last_pruned_at.is_none_or(|last_pruned_at| {
        let elapsed = now.signed_duration_since(last_pruned_at);
        elapsed < Duration::zero() || elapsed >= Duration::hours(HISTORY_PRUNE_INTERVAL_HOURS)
    })
}

fn sort_history_by_recency(history: &mut [HistoryEntry]) {
    history.sort_by_key(|entry| entry.last_visited);
}

fn history_is_sorted_by_recency(history: &[HistoryEntry]) -> bool {
    history
        .windows(2)
        .all(|entries| entries[0].last_visited <= entries[1].last_visited)
}

fn add_to_history(state: &mut AppState, path: &str) {
    let path = path.to_string();
    
    if let Some(index) = state
        .history
        .iter()
        .position(|e| e.path.eq_ignore_ascii_case(&path))
    {
        let mut entry = state.history.remove(index);
        entry.last_visited = Utc::now();
        entry.visit_count += 1;
        state.history.push(entry);
    } else {
        state.history.push(HistoryEntry {
            path: path.clone(),
            last_visited: Utc::now(),
            visit_count: 1,
        });
    }
    
    if state.history.len() > MAX_HISTORY {
        state.history.sort_by(|a, b| {
            let score_a = a.visit_count as i64 + a.last_visited.timestamp() / 86400;
            let score_b = b.visit_count as i64 + b.last_visited.timestamp() / 86400;
            score_b.cmp(&score_a)
        });
        state.history.truncate(MAX_HISTORY);
        sort_history_by_recency(&mut state.history);
    }
}

fn record_jump(state: &mut AppState, current_dir: Option<&str>, destination: &str) {
    if let Some(current_dir) = current_dir {
        push_undo(state, current_dir);
    }
    add_to_history(state, destination);
}

fn push_undo(state: &mut AppState, path: &str) {
    state.undo_stack.push_back(path.to_string());
    if state.undo_stack.len() > MAX_UNDO_STACK {
        state.undo_stack.pop_front();
    }
    state.redo_stack.clear();
}

fn output_path(path: &Path) {
    if let Some(path_str) = path.to_str() {
        #[cfg(windows)]
        let clean_path = if path_str.starts_with("\\\\?\\") {
            &path_str[4..]
        } else {
            path_str
        };
        #[cfg(not(windows))]
        let clean_path = path_str;
        
        println!("{}", clean_path);
    }
}

fn print_help() {
    eprintln!("j {} - Fast directory jump", env!("CARGO_PKG_VERSION"));
    eprintln!();
    eprintln!("Usage:");
    eprintln!("  j                  Jump to home directory");
    eprintln!("  j <keyword>        Jump to directory matching keyword");
    eprintln!("  j <kw1> <kw2> ...  Jump using multiple keywords (in order)");
    eprintln!("  j -i [keyword]     Interactive selection with fzf");
    eprintln!("  j -d               Browse directories with arrow keys");
    eprintln!("  j init <shell>     Generate shell integration");
    eprintln!("  j !<alias>         Jump to aliased directory");
    eprintln!("  j ~<path>          Jump to path under home directory");
    eprintln!();
    eprintln!("History:");
    eprintln!("  j -                Go back (Undo)");
    eprintln!("  j +                Go forward (Redo)");
    eprintln!("  j .                Jump to last visited directory");
    eprintln!("  j -c               Record current directory to history");
    eprintln!("  j -x               Remove current directory from history");
    eprintln!("  j -xa              Clear all history");
    eprintln!("  j -l [keyword]     Select from 20 recent name matches (x deletes)");
    eprintln!();
    eprintln!("Aliases:");
    eprintln!("  j -a <n>        Create alias for current directory");
    eprintln!("  j -ar <n>       Remove alias");
    eprintln!("  j -al              List aliases");
    eprintln!();
    eprintln!("Excludes:");
    eprintln!("  j --exclude-add <pattern>    Add exclude pattern");
    eprintln!("  j --exclude-remove <pattern> Remove exclude pattern");
    eprintln!("  j --exclude-list             List exclude patterns");
    eprintln!();
    eprintln!("Configuration:");
    eprintln!("  j --ranking [mode]           recent, frecency, or frequency");
    eprintln!("  j --fzf-opts [options]       Show or set fzf options");
    eprintln!("  j --fzf-opts-clear           Restore default fzf options");
    eprintln!();
    eprintln!("Examples:");
    eprintln!("  j src              Jump to 'src' directory");
    eprintln!("  j proj src         Jump with keywords in order");
    eprintln!("  j -i               Interactive fzf selection");
    eprintln!("  j -i proj          Filtered interactive selection");
    eprintln!("  j -d               Browse directories with arrow keys");
    #[cfg(windows)]
    eprintln!("  j d:src            Search in D: drive");
    eprintln!("  j --exclude-add node_modules");
}

fn main() {
    setup_console();

    let args: Vec<String> = env::args().collect();
    if let Some(arg) = args.get(1) {
        match arg.as_str() {
            "init" => {
                match shell_init::generate(&args[2..]) {
                    Ok(script) => print!("{}", script),
                    Err(error) => eprintln!("{}", error),
                }
                return;
            }
            "--version" | "-V" => {
                eprintln!("j {}", env!("CARGO_PKG_VERSION"));
                return;
            }
            "--help" | "-h" => {
                print_help();
                return;
            }
            _ => {}
        }
    }

    let mut state = load_state();

    let current_dir = env::current_dir()
        .ok()
        .and_then(|p| p.to_str().map(|s| s.to_string()));
    
    if args.len() < 2 {
        if let Some(home) = dirs::home_dir() {
            if let Some(ref cur) = current_dir {
                push_undo(&mut state, cur);
            }
            add_to_history(&mut state, home.to_str().unwrap_or(""));
            save_state(&mut state).ok();
            output_path(&home);
        }
        return;
    }
    
    let arg = &args[1];
    
    match arg.as_str() {
        // Interactive mode with fzf
        "-i" | "--interactive" => {
            let config = load_config();
            let keyword = if args.len() > 2 {
                args[2..].join("/")
            } else {
                String::new()
            };
            let candidates = search_history_all(&state, &keyword, &config);
            if candidates.is_empty() {
                eprintln!("No matches found");
                return;
            }
            if let Some(selected) = run_fzf(&candidates, &keyword, &config) {
                if Path::new(&selected).is_dir() {
                    // fzf may stay open for a while. Reload immediately before
                    // saving so another shell's newer history is not overwritten.
                    state = load_state();
                    record_jump(&mut state, current_dir.as_deref(), &selected);
                    if let Err(error) = save_state(&mut state) {
                        eprintln!("Failed to update history: {}", error);
                    }
                    println!("{}", selected);
                }
            }
            return;
        }
        // Browse directories with the built-in selector
        "-d" | "--directory" => {
            let Some(ref current) = current_dir else {
                eprintln!("Cannot get current directory");
                return;
            };
            match browse_directories(Path::new(current)) {
                Ok(Some(selected)) => {
                    let selected_path = selected.to_string_lossy().into_owned();
                    state = load_state();
                    record_jump(&mut state, current_dir.as_deref(), &selected_path);
                    if let Err(error) = save_state(&mut state) {
                        eprintln!("Failed to update history: {}", error);
                    }
                    output_path(&selected);
                }
                Ok(None) => {}
                Err(error) => eprintln!("Cannot open directory browser: {}", error),
            }
            return;
        }
        // Tab completion - output all matching paths
        "--complete" => {
            let config = load_config();
            let keyword = if args.len() > 2 {
                args[2..].join("/")
            } else {
                String::new()
            };
            let candidates = search_history_all(&state, &keyword, &config);
            for path in candidates {
                println!("{}", path);
            }
            return;
        }
        "--ranking" => {
            let mut config = load_config();
            if let Some(value) = args.get(2) {
                if let Some(ranking) = Ranking::parse(value) {
                    config.ranking = ranking;
                    if save_config(&config).is_ok() {
                        eprintln!("Ranking: {}", ranking.as_str());
                    }
                } else {
                    eprintln!("Ranking must be recent, frecency, or frequency");
                }
            } else {
                eprintln!("Ranking: {}", config.ranking.as_str());
            }
            return;
        }
        "--fzf-opts" => {
            let mut config = load_config();
            if args.len() > 2 {
                let value = args[2..].join(" ");
                match split_option_string(&value) {
                    Ok(_) => {
                        config.fzf_opts = Some(value);
                        if save_config(&config).is_ok() {
                            eprintln!("fzf options updated");
                        }
                    }
                    Err(error) => eprintln!("Invalid fzf options: {}", error),
                }
            } else if let Ok(value) = env::var("J_FZF_OPTS") {
                eprintln!("fzf options (J_FZF_OPTS): {}", value);
            } else if let Some(value) = &config.fzf_opts {
                eprintln!("fzf options: {}", value);
            } else {
                eprintln!("fzf options: default");
            }
            return;
        }
        "--fzf-opts-clear" => {
            let mut config = load_config();
            config.fzf_opts = None;
            if save_config(&config).is_ok() {
                eprintln!("fzf options cleared");
            }
            return;
        }
        // Exclude pattern management
        "--exclude-add" => {
            if let Some(pattern) = args.get(2) {
                let mut config = load_config();
                if !config.excludes.contains(pattern) {
                    config.excludes.push(pattern.clone());
                    save_config(&config).ok();
                    eprintln!("Added exclude pattern: {}", pattern);
                } else {
                    eprintln!("Pattern already exists: {}", pattern);
                }
            } else {
                eprintln!("Usage: j --exclude-add <pattern>");
            }
            return;
        }
        "--exclude-remove" => {
            if let Some(pattern) = args.get(2) {
                let mut config = load_config();
                if let Some(pos) = config.excludes.iter().position(|p| p == pattern) {
                    config.excludes.remove(pos);
                    save_config(&config).ok();
                    eprintln!("Removed exclude pattern: {}", pattern);
                } else {
                    eprintln!("Pattern not found: {}", pattern);
                }
            } else {
                eprintln!("Usage: j --exclude-remove <pattern>");
            }
            return;
        }
        "--exclude-list" => {
            let config = load_config();
            if config.excludes.is_empty() {
                eprintln!("No exclude patterns");
            } else {
                eprintln!("Exclude patterns:");
                for pattern in &config.excludes {
                    eprintln!("  {}", pattern);
                }
            }
            return;
        }
        "-c" => {
            if let Some(ref cur) = current_dir {
                add_to_history(&mut state, cur);
                save_state(&mut state).ok();
                eprintln!("Recorded: {}", cur);
            }
            return;
        }
        "-x" => {
            if let Some(ref cur) = current_dir {
                if remove_history_path(&mut state, cur) {
                    save_state(&mut state).ok();
                    eprintln!("Removed: {}", cur);
                } else {
                    eprintln!("Not in history: {}", cur);
                }
            }
            return;
        }
        "-" => {
            if let Some(prev) = state.undo_stack.pop_back() {
                if let Some(ref cur) = current_dir {
                    state.redo_stack.push_back(cur.clone());
                    if state.redo_stack.len() > MAX_UNDO_STACK {
                        state.redo_stack.pop_front();
                    }
                }
                save_state(&mut state).ok();
                println!("{}", prev);
            } else {
                eprintln!("No undo history");
            }
            return;
        }
        "+" => {
            if let Some(next) = state.redo_stack.pop_back() {
                if let Some(ref cur) = current_dir {
                    state.undo_stack.push_back(cur.clone());
                    if state.undo_stack.len() > MAX_UNDO_STACK {
                        state.undo_stack.pop_front();
                    }
                }
                save_state(&mut state).ok();
                println!("{}", next);
            } else {
                eprintln!("No redo history");
            }
            return;
        }
        "." => {
            if let Some(last) = state.history.last() {
                let last_path = last.path.clone();
                println!("{}", last_path);
            } else {
                eprintln!("No history");
            }
            return;
        }
        "--list" | "-l" => {
            let keyword = if args.len() > 2 {
                args[2..].join(" ")
            } else {
                String::new()
            };
            match select_history(&mut state, &keyword) {
                Ok(Some(path)) => {
                    if let Some(ref cur) = current_dir {
                        push_undo(&mut state, cur);
                    }
                    add_to_history(&mut state, &path);
                    save_state(&mut state).ok();
                    println!("{}", path);
                }
                Ok(None) => {}
                Err(error) => eprintln!("Cannot open history selector: {}", error),
            }
            return;
        }
        "-xa" => {
            state.history.clear();
            state.undo_stack.clear();
            state.redo_stack.clear();
            save_state(&mut state).ok();
            eprintln!("All history cleared");
            return;
        }
        "-a" => {
            if let Some(name) = args.get(2) {
                if let Some(ref cur) = current_dir {
                    let mut aliases = load_aliases();
                    let is_update = aliases.map.contains_key(name);
                    aliases.map.insert(name.clone(), cur.clone());
                    save_aliases(&aliases).ok();
                    if is_update {
                        eprintln!("Updated: {} -> {}", name, cur);
                    } else {
                        eprintln!("{} -> {}", name, cur);
                    }
                } else {
                    eprintln!("Cannot get current directory");
                }
            } else {
                eprintln!("Usage: j -a <name>");
            }
            return;
        }
        "-ar" => {
            if let Some(name) = args.get(2) {
                let mut aliases = load_aliases();
                if aliases.map.remove(name).is_some() {
                    save_aliases(&aliases).ok();
                    eprintln!("Alias removed: {}", name);
                } else {
                    eprintln!("Alias not found: {}", name);
                }
            } else {
                eprintln!("Usage: j -ar <name>");
            }
            return;
        }
        "-al" => {
            let aliases = load_aliases();
            if aliases.map.is_empty() {
                eprintln!("No aliases");
            } else {
                for (name, path) in &aliases.map {
                    eprintln!("!{} -> {}", name, path);
                }
            }
            return;
        }
        _ => {}
    }
    
    if arg.starts_with('!') {
        let alias_name = &arg[1..];
        let aliases = load_aliases();
        if let Some(path_str) = aliases.map.get(alias_name) {
            let path = PathBuf::from(path_str);
            if path.is_dir() {
                if let Some(ref cur) = current_dir {
                    push_undo(&mut state, cur);
                }
                add_to_history(&mut state, path_str);
                save_state(&mut state).ok();
                output_path(&path);
            } else {
                eprintln!("Directory does not exist: {}", path_str);
            }
        } else {
            eprintln!("Alias !{} not found", alias_name);
        }
        return;
    }
    
    if arg.starts_with('~') {
        if let Some(path) = expand_home(arg) {
            if path.is_dir() {
                if let Some(ref cur) = current_dir {
                    push_undo(&mut state, cur);
                }
                add_to_history(&mut state, path.to_str().unwrap_or(""));
                save_state(&mut state).ok();
                output_path(&path);
            } else {
                eprintln!("Directory not found: {}", path.display());
            }
        }
        return;
    }
    
    if is_absolute_path(arg) {
        let normalized = normalize_path_separator(arg);
        
        #[cfg(windows)]
        let path = if normalized == "\\" || normalized == "/" {
            if let Ok(cur) = env::current_dir() {
                if let Some(prefix) = cur.to_str().and_then(|s| s.get(0..3)) {
                    PathBuf::from(prefix)
                } else {
                    PathBuf::from("C:\\")
                }
            } else {
                PathBuf::from("C:\\")
            }
        } else {
            PathBuf::from(&normalized)
        };
        
        #[cfg(not(windows))]
        let path = PathBuf::from(&normalized);
        
        if path.is_dir() {
            if let Some(ref cur) = current_dir {
                push_undo(&mut state, cur);
            }
            add_to_history(&mut state, path.to_str().unwrap_or(""));
            save_state(&mut state).ok();
            output_path(&path);
        } else {
            eprintln!("Directory not found: {}", path.display());
        }
        return;
    }
    
    if is_relative_path(arg) {
        let normalized = normalize_path_separator(arg);
        if let Ok(current) = env::current_dir() {
            let path = current.join(&normalized);
            if let Ok(canonical) = path.canonicalize() {
                if canonical.is_dir() {
                    let path_str = canonical.to_str().unwrap_or("");
                    #[cfg(windows)]
                    let clean_path = if path_str.starts_with("\\\\?\\") {
                        &path_str[4..]
                    } else {
                        path_str
                    };
                    #[cfg(not(windows))]
                    let clean_path = path_str;
                    
                    if let Some(ref cur) = current_dir {
                        push_undo(&mut state, cur);
                    }
                    add_to_history(&mut state, clean_path);
                    save_state(&mut state).ok();
                    output_path(&canonical);
                    return;
                }
            }
        }
        eprintln!("Directory not found: {}", arg);
        return;
    }
    
    #[cfg(windows)]
    if let Some((drive, rest)) = extract_drive(arg) {
        let drive_root = format!("{}:\\", drive);
        if rest.is_empty() {
            let path = PathBuf::from(&drive_root);
            if path.is_dir() {
                if let Some(ref cur) = current_dir {
                    push_undo(&mut state, cur);
                }
                add_to_history(&mut state, &drive_root);
                save_state(&mut state).ok();
                output_path(&path);
            }
            return;
        }
        
        let search_term = rest.trim_start_matches('\\').trim_start_matches('/');
        
        let drive_prefix = format!("{}:", drive);
        let found_path = state.history.iter().rev().find_map(|entry| {
            if entry.path.to_uppercase().starts_with(&drive_prefix.to_uppercase()) {
                let path_lower = entry.path.to_lowercase();
                let search_lower = search_term.to_lowercase();
                let tokens: Vec<&str> = search_lower.split(&['\\', '/'][..]).filter(|s| !s.is_empty()).collect();
                
                let all_match = tokens.iter().all(|t| {
                    path_lower.split('\\').any(|part| part.contains(*t))
                });
                
                if all_match && Path::new(&entry.path).is_dir() {
                    Some(entry.path.clone())
                } else {
                    None
                }
            } else {
                None
            }
        });
        
        if let Some(found) = found_path {
            if let Some(ref cur) = current_dir {
                push_undo(&mut state, cur);
            }
            add_to_history(&mut state, &found);
            save_state(&mut state).ok();
            println!("{}", found);
            return;
        }
        
        eprintln!("Directory not found on {}: {}", drive, search_term);
        return;
    }
    
    // Support multiple arguments: j first one → search for "first/one"
    let search_keyword = if args.len() > 2 {
        args[1..].join("/")
    } else {
        arg.to_string()
    };
    
    if let Some(path) = try_local_path(&search_keyword) {
        if let Some(ref cur) = current_dir {
            push_undo(&mut state, cur);
        }
        add_to_history(&mut state, path.to_str().unwrap_or(""));
        save_state(&mut state).ok();
        output_path(&path);
        return;
    }
    
    let config = load_config();
    if let Some(path) = search_history(&state, &search_keyword, &config) {
        if let Some(ref cur) = current_dir {
            push_undo(&mut state, cur);
        }
        add_to_history(&mut state, &path);
        save_state(&mut state).ok();
        println!("{}", path);
        return;
    }
    
    eprintln!("Directory not found: {}", search_keyword);
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn history_entry(path: &str, timestamp: i64, visit_count: u32) -> HistoryEntry {
        HistoryEntry {
            path: path.to_string(),
            last_visited: Utc.timestamp_opt(timestamp, 0).unwrap(),
            visit_count,
        }
    }

    #[test]
    fn old_config_defaults_to_recent_ranking() {
        let config: Config = serde_json::from_str(r#"{"excludes":["tmp"]}"#).unwrap();
        assert_eq!(config.ranking, Ranking::Recent);
        assert_eq!(config.fzf_opts, None);
    }

    #[test]
    fn old_state_defaults_to_pruning_not_yet_run() {
        let state: AppState = serde_json::from_str(
            r#"{
                "history": [],
                "undo_stack": [],
                "redo_stack": [],
                "current_dir": null
            }"#,
        )
        .unwrap();

        assert_eq!(state.last_pruned_at, None);
    }

    #[test]
    fn history_pruning_runs_at_most_once_per_day() {
        let now = Utc.timestamp_opt(20_000_000, 0).unwrap();
        let mut state = AppState::default();

        assert!(history_prune_is_due(&state, now));

        state.last_pruned_at = Some(now - Duration::hours(HISTORY_PRUNE_INTERVAL_HOURS - 1));
        assert!(!history_prune_is_due(&state, now));

        state.last_pruned_at = Some(now - Duration::hours(HISTORY_PRUNE_INTERVAL_HOURS));
        assert!(history_prune_is_due(&state, now));

        state.last_pruned_at = Some(now + Duration::hours(1));
        assert!(history_prune_is_due(&state, now));
    }

    #[test]
    fn option_string_supports_quoted_fzf_values() {
        let args = split_option_string(r#"--height 60% --prompt "jump> " --border"#).unwrap();
        assert_eq!(
            args,
            vec!["--height", "60%", "--prompt", "jump> ", "--border"]
        );
        assert!(split_option_string(r#"--prompt "jump> "#).is_err());
    }

    #[test]
    fn direct_directory_candidates_only_include_immediate_directories() {
        let test_root = env::temp_dir().join(format!(
            "j-cmd-directories-{}-{}",
            std::process::id(),
            Utc::now().timestamp_nanos_opt().unwrap()
        ));
        fs::create_dir_all(test_root.join("beta").join("nested")).unwrap();
        fs::create_dir_all(test_root.join("Alpha")).unwrap();
        fs::write(test_root.join("file.txt"), "not a directory").unwrap();

        let candidates = direct_directory_candidates(&test_root).unwrap();
        let names: Vec<&str> = candidates
            .iter()
            .map(|candidate| candidate.name.as_str())
            .collect();

        assert_eq!(names, vec!["Alpha", "beta"]);
        assert!(candidates
            .iter()
            .all(|candidate| candidate.path.parent() == Some(test_root.as_path())));
        fs::remove_dir_all(test_root).unwrap();
    }

    #[test]
    fn directory_browser_moves_right_and_left_between_directories() {
        let test_root = env::temp_dir().join(format!(
            "j-cmd-browser-{}-{}",
            std::process::id(),
            Utc::now().timestamp_nanos_opt().unwrap()
        ));
        let alpha = test_root.join("Alpha");
        let beta = test_root.join("beta");
        let nested = beta.join("nested");
        fs::create_dir_all(&alpha).unwrap();
        fs::create_dir_all(&nested).unwrap();

        let mut browser = DirectoryBrowser::new(&test_root).unwrap();
        assert_eq!(browser.selected_path().as_deref(), Some(alpha.as_path()));

        browser.select_next();
        assert_eq!(browser.selected_path().as_deref(), Some(beta.as_path()));
        assert!(browser.move_into_selected().unwrap());
        assert_eq!(browser.current, beta);
        assert_eq!(browser.selected_path().as_deref(), Some(nested.as_path()));

        assert!(browser.move_to_parent().unwrap());
        assert_eq!(browser.current, test_root);
        assert_eq!(browser.selected_path().as_deref(), Some(beta.as_path()));

        fs::remove_dir_all(&browser.current).unwrap();
    }

    #[test]
    fn directory_browser_selects_case_insensitive_name_prefixes() {
        let test_root = env::temp_dir().join(format!(
            "j-cmd-prefix-{}-{}",
            std::process::id(),
            Utc::now().timestamp_nanos_opt().unwrap()
        ));
        let alpha = test_root.join("Alpha");
        let beta = test_root.join("beta");
        fs::create_dir_all(&alpha).unwrap();
        fs::create_dir_all(&beta).unwrap();

        let mut browser = DirectoryBrowser::new(&test_root).unwrap();
        assert!(browser.select_prefix("BE"));
        assert_eq!(browser.selected_path().as_deref(), Some(beta.as_path()));
        assert!(browser.select_prefix("a"));
        assert_eq!(browser.selected_path().as_deref(), Some(alpha.as_path()));
        assert!(!browser.select_prefix("missing"));
        assert_eq!(browser.selected_path().as_deref(), Some(alpha.as_path()));

        fs::remove_dir_all(test_root).unwrap();
    }

    #[test]
    fn directory_browser_selection_does_not_wrap_at_list_boundaries() {
        let test_root = env::temp_dir().join(format!(
            "j-cmd-boundaries-{}-{}",
            std::process::id(),
            Utc::now().timestamp_nanos_opt().unwrap()
        ));
        let alpha = test_root.join("Alpha");
        let beta = test_root.join("beta");
        fs::create_dir_all(&alpha).unwrap();
        fs::create_dir_all(&beta).unwrap();

        let mut browser = DirectoryBrowser::new(&test_root).unwrap();
        browser.select_previous();
        assert_eq!(browser.selected_path().as_deref(), Some(alpha.as_path()));

        browser.select_next();
        browser.select_next();
        assert_eq!(browser.selected_path().as_deref(), Some(beta.as_path()));

        fs::remove_dir_all(test_root).unwrap();
    }

    #[test]
    fn directory_browser_confirms_the_selected_directory_or_current_leaf() {
        let test_root = env::temp_dir().join(format!(
            "j-cmd-current-{}-{}",
            std::process::id(),
            Utc::now().timestamp_nanos_opt().unwrap()
        ));
        let child = test_root.join("child");
        let nested = child.join("nested");
        fs::create_dir_all(&nested).unwrap();

        let mut browser = DirectoryBrowser::new(&test_root).unwrap();
        assert_eq!(browser.current_path(), test_root);
        assert_eq!(browser.selected_path().as_deref(), Some(child.as_path()));

        assert_eq!(browser.selected_or_current_path(), child);
        assert_eq!(browser.current_path(), test_root);

        assert!(browser.move_into_selected().unwrap());
        assert_eq!(browser.current_path(), child);
        assert_eq!(browser.selected_path().as_deref(), Some(nested.as_path()));
        assert_eq!(browser.selected_or_current_path(), nested);

        assert!(browser.move_into_selected().unwrap());
        assert_eq!(browser.current_path(), nested);
        assert_eq!(browser.selected_path(), None);
        assert_eq!(browser.selected_or_current_path(), nested);

        fs::remove_dir_all(browser.current_path()).unwrap();
        fs::remove_dir_all(test_root).unwrap();
    }

    #[test]
    fn directory_browser_can_confirm_current_while_a_child_is_selected() {
        let test_root = env::temp_dir().join(format!(
            "j-cmd-confirm-current-{}-{}",
            std::process::id(),
            Utc::now().timestamp_nanos_opt().unwrap()
        ));
        let child = test_root.join("child");
        fs::create_dir_all(&child).unwrap();

        let browser = DirectoryBrowser::new(&test_root).unwrap();
        assert_eq!(browser.selected_path().as_deref(), Some(child.as_path()));
        assert_eq!(browser.current_path(), test_root);

        fs::remove_dir_all(test_root).unwrap();
    }

    #[test]
    fn ranking_modes_preserve_recent_default_and_allow_frequency() {
        let now = Utc.timestamp_opt(1_000_000, 0).unwrap();
        let state = AppState {
            history: vec![
                history_entry("frequent", 900_000, 20),
                history_entry("recent", 999_999, 1),
            ],
            ..AppState::default()
        };
        let mut config = Config::default();

        let recent = ranked_history_entries(&state, &config, now);
        assert_eq!(recent[0].path, "recent");

        config.ranking = Ranking::Frequency;
        let frequency = ranked_history_entries(&state, &config, now);
        assert_eq!(frequency[0].path, "frequent");

        config.ranking = Ranking::Frecency;
        let frecency = ranked_history_entries(&state, &config, now);
        assert_eq!(frecency[0].path, "frequent");
    }

    #[test]
    fn recent_ranking_breaks_timestamp_ties_by_path() {
        let now = Utc.timestamp_opt(1_000_000, 0).unwrap();
        let state = AppState {
            history: vec![
                history_entry("oldest", 100, 1),
                history_entry("beta", 200, 1),
                history_entry("alpha", 200, 1),
                history_entry("newest", 300, 1),
            ],
            ..AppState::default()
        };

        let paths: Vec<&str> = ranked_history_entries(&state, &Config::default(), now)
            .iter()
            .map(|entry| entry.path.as_str())
            .collect();
        assert_eq!(paths, vec!["newest", "alpha", "beta", "oldest"]);
    }

    #[test]
    fn interactive_jump_records_selected_path_as_most_recent() {
        let mut state = AppState {
            history: vec![
                history_entry("selected", 100, 3),
                history_entry("other", 200, 1),
            ],
            ..AppState::default()
        };

        record_jump(&mut state, Some("current"), "selected");

        assert_eq!(state.history.last().unwrap().path, "selected");
        assert_eq!(state.history.last().unwrap().visit_count, 4);
        assert_eq!(state.undo_stack.back().map(String::as_str), Some("current"));
    }

    #[test]
    fn pruning_only_removes_missing_entries_older_than_ninety_days() {
        let now = Utc.timestamp_opt(20_000_000, 0).unwrap();
        let test_root = env::temp_dir().join(format!(
            "j-cmd-prune-{}-{}",
            std::process::id(),
            Utc::now().timestamp_nanos_opt().unwrap()
        ));
        fs::create_dir_all(&test_root).unwrap();
        let old = now - Duration::days(MISSING_HISTORY_RETENTION_DAYS + 1);
        let recent = now - Duration::days(MISSING_HISTORY_RETENTION_DAYS - 1);
        let mut state = AppState {
            history: vec![
                HistoryEntry {
                    path: test_root.to_string_lossy().into_owned(),
                    last_visited: old,
                    visit_count: 1,
                },
                HistoryEntry {
                    path: test_root.join("missing-old").to_string_lossy().into_owned(),
                    last_visited: old,
                    visit_count: 1,
                },
                HistoryEntry {
                    path: test_root
                        .join("missing-recent")
                        .to_string_lossy()
                        .into_owned(),
                    last_visited: recent,
                    visit_count: 1,
                },
            ],
            ..AppState::default()
        };

        assert_eq!(prune_stale_missing_history(&mut state, now), 1);
        assert_eq!(state.history.len(), 2);
        assert!(state.history.iter().any(|entry| entry.path == test_root.to_string_lossy()));
        assert!(state
            .history
            .iter()
            .any(|entry| entry.path.ends_with("missing-recent")));
        fs::remove_dir_all(test_root).unwrap();
    }

    #[test]
    fn sort_history_by_recency_places_newest_last() {
        let mut history = vec![
            history_entry("newest", 300, 1),
            history_entry("oldest", 100, 1),
            history_entry("middle", 200, 1),
        ];

        assert!(!history_is_sorted_by_recency(&history));
        sort_history_by_recency(&mut history);
        assert!(history_is_sorted_by_recency(&history));

        let paths: Vec<&str> = history.iter().map(|entry| entry.path.as_str()).collect();
        assert_eq!(paths, vec!["oldest", "middle", "newest"]);
    }

    #[test]
    fn revisiting_history_entry_moves_it_to_newest_position() {
        let mut state = AppState {
            history: vec![
                history_entry("first", 100, 1),
                history_entry("second", 200, 1),
            ],
            ..AppState::default()
        };

        add_to_history(&mut state, "first");

        let paths: Vec<&str> = state
            .history
            .iter()
            .map(|entry| entry.path.as_str())
            .collect();
        assert_eq!(paths, vec!["second", "first"]);
        assert_eq!(state.history.last().unwrap().visit_count, 2);
    }

    #[test]
    fn search_prefers_most_recently_used_matching_directory() {
        let test_root = env::temp_dir().join(format!(
            "j-cmd-history-order-{}-{}",
            std::process::id(),
            Utc::now().timestamp_nanos_opt().unwrap()
        ));
        let older = test_root.join("older").join("project");
        let newer = test_root.join("newer").join("project");
        fs::create_dir_all(&older).unwrap();
        fs::create_dir_all(&newer).unwrap();

        let mut state = AppState {
            history: vec![
                history_entry(newer.to_str().unwrap(), 200, 1),
                history_entry(older.to_str().unwrap(), 100, 1),
            ],
            ..AppState::default()
        };
        sort_history_by_recency(&mut state.history);

        let found = search_history(&state, "project", &Config::default());

        assert_eq!(found.as_deref(), newer.to_str());
        fs::remove_dir_all(test_root).unwrap();
    }

    #[test]
    fn history_list_filters_final_directory_names_and_keeps_recent_first() {
        let state = AppState {
            history: vec![
                history_entry("C:\\path\\fgha\\abc-old", 100, 1),
                history_entry("C:\\path\\abcde\\fgha", 200, 1),
                history_entry("D:\\path\\fgha\\abcde", 300, 1),
            ],
            ..AppState::default()
        };

        let candidates = history_list_candidates(&state, "ABC");

        assert_eq!(
            candidates,
            vec!["D:\\path\\fgha\\abcde", "C:\\path\\fgha\\abc-old"]
        );
        assert!(!candidates.contains(&"C:\\path\\abcde\\fgha".to_string()));
    }

    #[test]
    fn history_list_is_limited_to_twenty_most_recent_entries() {
        let state = AppState {
            history: (0..25)
                .map(|index| history_entry(&format!("history-{index}"), index, 1))
                .collect(),
            ..AppState::default()
        };

        let candidates = history_list_candidates(&state, "");

        assert_eq!(candidates.len(), HISTORY_LIST_LIMIT);
        assert_eq!(candidates.first().unwrap(), "history-24");
        assert_eq!(candidates.last().unwrap(), "history-5");
    }

    #[test]
    fn removing_selected_history_path_is_case_insensitive() {
        let mut state = AppState {
            history: vec![
                history_entry("C:\\work\\first", 100, 1),
                history_entry("C:\\work\\second", 200, 1),
            ],
            ..AppState::default()
        };

        assert!(remove_history_path(&mut state, "c:\\WORK\\FIRST"));
        assert_eq!(state.history.len(), 1);
        assert_eq!(state.history[0].path, "C:\\work\\second");
        assert!(!remove_history_path(&mut state, "C:\\work\\missing"));
    }
}
