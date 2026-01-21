use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

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

#[derive(Debug, Clone, Serialize, Deserialize)]
struct HistoryEntry {
    path: String,
    last_visited: DateTime<Utc>,
    visit_count: u32,
}

#[derive(Debug, Serialize, Deserialize)]
struct AppState {
    history: Vec<HistoryEntry>,
    undo_stack: VecDeque<String>,
    redo_stack: VecDeque<String>,
    current_dir: Option<String>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            history: Vec::new(),
            undo_stack: VecDeque::new(),
            redo_stack: VecDeque::new(),
            current_dir: None,
        }
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct Aliases {
    #[serde(flatten)]
    map: std::collections::HashMap<String, String>,
}

fn get_config_dir() -> PathBuf {
    let config_dir = dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".config")
        .join("j");
    fs::create_dir_all(&config_dir).ok();
    config_dir
}

fn get_state_path() -> PathBuf {
    get_config_dir().join("state.json")
}

fn get_aliases_path() -> PathBuf {
    get_config_dir().join("aliases.json")
}

fn load_state() -> AppState {
    let path = get_state_path();
    if path.exists() {
        fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    } else {
        AppState::default()
    }
}

fn save_state(state: &AppState) -> io::Result<()> {
    let path = get_state_path();
    let json = serde_json::to_string_pretty(state)?;
    fs::write(path, json)
}

fn load_aliases() -> Aliases {
    let path = get_aliases_path();
    if path.exists() {
        fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    } else {
        Aliases::default()
    }
}

fn save_aliases(aliases: &Aliases) -> io::Result<()> {
    let path = get_aliases_path();
    let json = serde_json::to_string_pretty(aliases)?;
    fs::write(path, json)
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

fn search_history(state: &AppState, keyword: &str) -> Option<String> {
    let tokens: Vec<String> = split_path(keyword)
        .iter()
        .map(|s| s.to_lowercase())
        .collect();
    
    if tokens.is_empty() {
        return None;
    }
    
    let last_token = tokens.last().unwrap();
    
    for entry in state.history.iter().rev() {
        let path = Path::new(&entry.path);
        if let Some(dir_name) = path.file_name().and_then(|n| n.to_str()) {
            if dir_name.to_lowercase() == *last_token {
                if tokens.len() > 1 {
                    let path_lower = entry.path.to_lowercase();
                    let other_tokens_match = tokens[..tokens.len()-1].iter()
                        .all(|t| split_path(&path_lower).iter().any(|part| part.contains(t.as_str())));
                    if other_tokens_match && Path::new(&entry.path).is_dir() {
                        return Some(entry.path.clone());
                    }
                } else if Path::new(&entry.path).is_dir() {
                    return Some(entry.path.clone());
                }
            }
        }
    }
    
    for entry in state.history.iter().rev() {
        let path = Path::new(&entry.path);
        if let Some(dir_name) = path.file_name().and_then(|n| n.to_str()) {
            if dir_name.to_lowercase().contains(last_token.as_str()) {
                if tokens.len() > 1 {
                    let path_lower = entry.path.to_lowercase();
                    let other_tokens_match = tokens[..tokens.len()-1].iter()
                        .all(|t| split_path(&path_lower).iter().any(|part| part.contains(t.as_str())));
                    if other_tokens_match && Path::new(&entry.path).is_dir() {
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

fn add_to_history(state: &mut AppState, path: &str) {
    let path = path.to_string();
    
    if let Some(entry) = state.history.iter_mut().find(|e| e.path.eq_ignore_ascii_case(&path)) {
        entry.last_visited = Utc::now();
        entry.visit_count += 1;
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
    }
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

fn main() {
    setup_console();
    
    let args: Vec<String> = env::args().collect();
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
            save_state(&state).ok();
            output_path(&home);
        }
        return;
    }
    
    let arg = &args[1];
    
    match arg.as_str() {
        "-c" => {
            if let Some(ref cur) = current_dir {
                add_to_history(&mut state, cur);
                save_state(&state).ok();
                eprintln!("Recorded: {}", cur);
            }
            return;
        }
        "-x" => {
            if let Some(ref cur) = current_dir {
                let before_len = state.history.len();
                state.history.retain(|e| !e.path.eq_ignore_ascii_case(cur));
                if state.history.len() < before_len {
                    save_state(&state).ok();
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
                save_state(&state).ok();
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
                save_state(&state).ok();
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
            let count = args.get(2).and_then(|s| s.parse::<usize>().ok()).unwrap_or(20);
            for (i, entry) in state.history.iter().rev().take(count).enumerate() {
                eprintln!("{:2}. {} ({} visits)", i + 1, entry.path, entry.visit_count);
            }
            return;
        }
        "-xa" => {
            state.history.clear();
            state.undo_stack.clear();
            state.redo_stack.clear();
            save_state(&state).ok();
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
        "--version" | "-V" => {
            eprintln!("j {}", env!("CARGO_PKG_VERSION"));
            return;
        }
        "--help" | "-h" => {
            eprintln!("j {} - Fast directory jump", env!("CARGO_PKG_VERSION"));
            eprintln!();
            eprintln!("Usage:");
            eprintln!("  j                  Jump to home directory");
            eprintln!("  j <keyword>        Jump to directory matching keyword");
            eprintln!("  j !<alias>         Jump to aliased directory");
            eprintln!("  j ~<path>          Jump to path under home directory");
            eprintln!("  j -c               Record current directory to history");
            eprintln!("  j -x               Remove current directory from history");
            eprintln!("  j -xa              Clear all history");
            eprintln!("  j -                Go back (Undo)");
            eprintln!("  j +                Go forward (Redo)");
            eprintln!("  j .                Jump to last visited directory");
            eprintln!("  j -l [N]           List history (default 20)");
            eprintln!("  j -N               Jump to Nth history entry (e.g., j -1, j -5)");
            eprintln!("  j -a <name>        Create alias for current directory");
            eprintln!("  j -ar <name>       Remove alias");
            eprintln!("  j -al              List aliases");
            eprintln!();
            eprintln!("Examples:");
            eprintln!("  j src              Jump to directory containing 'src'");
            eprintln!("  j proj/src         Jump to path containing 'proj' and 'src'");
            #[cfg(windows)]
            eprintln!("  j d:src            Search 'src' in D: drive");
            eprintln!("  j -l 10            Show 10 history entries");
            eprintln!("  j -3               Jump to 3rd history entry");
            eprintln!("  j -a work          Create alias !work for current directory");
            eprintln!("  j !work            Jump to !work");
            return;
        }
        _ => {}
    }
    
    if arg.starts_with('-') {
        if let Ok(num) = arg[1..].parse::<usize>() {
            if num > 0 {
                let index = num - 1;
                let history_len = state.history.len();
                if index < history_len {
                    let target_path = state.history[history_len - 1 - index].path.clone();
                    if Path::new(&target_path).is_dir() {
                        if let Some(ref cur) = current_dir {
                            push_undo(&mut state, cur);
                        }
                        add_to_history(&mut state, &target_path);
                        save_state(&state).ok();
                        println!("{}", target_path);
                    } else {
                        eprintln!("Directory not found: {}", target_path);
                    }
                } else {
                    eprintln!("History entry {} does not exist (history size: {})", num, history_len);
                }
                return;
            }
        }
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
                save_state(&state).ok();
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
                save_state(&state).ok();
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
            save_state(&state).ok();
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
                    save_state(&state).ok();
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
                save_state(&state).ok();
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
            save_state(&state).ok();
            println!("{}", found);
            return;
        }
        
        eprintln!("Directory not found on {}: {}", drive, search_term);
        return;
    }
    
    if let Some(path) = try_local_path(arg) {
        if let Some(ref cur) = current_dir {
            push_undo(&mut state, cur);
        }
        add_to_history(&mut state, path.to_str().unwrap_or(""));
        save_state(&state).ok();
        output_path(&path);
        return;
    }
    
    if let Some(path) = search_history(&state, arg) {
        if let Some(ref cur) = current_dir {
            push_undo(&mut state, cur);
        }
        add_to_history(&mut state, &path);
        save_state(&state).ok();
        println!("{}", path);
        return;
    }
    
    eprintln!("Directory not found: {}", arg);
}
