#![allow(
    clippy::collapsible_if,
    clippy::redundant_closure,
    deprecated,
    dead_code
)]

use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use crossterm::ExecutableCommand;
use crossterm::event::{self, Event, KeyCode};
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};

use ratatui::Frame;
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::Style;
use ratatui::widgets::ListState;
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Focus {
    Input,
    OutputDir,
    From,
    To,
    Run,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Screen {
    Main,
    PickInput,
    PickOutputDir,
    ConfirmDir,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PickKind {
    File,
    Dir,
}

struct Picker {
    cwd: PathBuf,
    all_items: Vec<PathBuf>,
    items: Vec<PathBuf>,
    state: ListState,
    kind: PickKind,
    query: String,
    searching: bool,
}

impl Picker {
    fn new_at(start: PathBuf, kind: PickKind) -> Self {
        let mut p = Picker {
            cwd: start,
            all_items: Vec::new(),
            items: Vec::new(),
            state: ListState::default(),
            kind,
            query: String::new(),
            searching: false,
        };
        p.refresh();
        p
    }

    fn refresh(&mut self) {
        let mut entries = vec![];
        if let Ok(read) = fs::read_dir(&self.cwd) {
            for e in read.flatten() {
                entries.push(e.path());
            }
        }
        entries.sort_by_key(|p| (p.is_file(), p.file_name().map(|s| s.to_os_string())));
        self.all_items = entries;
        self.apply_filter();
        if !self.items.is_empty() {
            self.state.select(Some(0));
        } else {
            self.state.select(None);
        }
    }

    fn move_up(&mut self) {
        let i = self.state.selected().unwrap_or(0);
        if i == 0 {
            let last = self.items.len().saturating_sub(1);
            self.state.select(Some(last));
        } else {
            self.state.select(Some(i - 1));
        }
    }

    fn move_down(&mut self) {
        let i = self.state.selected().unwrap_or(0);
        if i + 1 >= self.items.len() {
            self.state.select(Some(0));
        } else {
            self.state.select(Some(i + 1));
        }
    }

    fn go_parent(&mut self) {
        if let Some(parent) = self.cwd.parent() {
            self.cwd = parent.to_path_buf();
            self.query.clear();
            self.searching = false;
            self.refresh();
        }
    }

    fn page_up(&mut self) {
        let i = self.state.selected().unwrap_or(0);
        let step = 10usize;
        let new = i.saturating_sub(step);
        self.state.select(Some(new));
    }

    fn page_down(&mut self) {
        let i = self.state.selected().unwrap_or(0);
        let step = 10usize;
        let new = std::cmp::min(i + step, self.items.len().saturating_sub(1));
        self.state.select(Some(new));
    }

    fn select_first(&mut self) {
        if !self.items.is_empty() {
            self.state.select(Some(0));
        }
    }

    fn select_last(&mut self) {
        if !self.items.is_empty() {
            self.state.select(Some(self.items.len() - 1));
        }
    }

    fn enter(&mut self) {
        if let Some(idx) = self.state.selected() {
            if let Some(p) = self.items.get(idx) {
                if p.is_dir() {
                    self.cwd = p.clone();
                    self.query.clear();
                    self.searching = false;
                    self.refresh();
                }
            }
        }
    }

    fn selected_path(&self) -> Option<PathBuf> {
        match self.state.selected() {
            Some(i) => self.items.get(i).cloned(),
            None => None,
        }
    }

    fn apply_filter(&mut self) {
        if self.query.is_empty() {
            self.items = self.all_items.clone();
        } else {
            let q = self.query.to_lowercase();
            self.items = self
                .all_items
                .iter()
                .filter(|p| {
                    p.file_name()
                        .and_then(|s| s.to_str())
                        .map(|n| n.to_lowercase().contains(&q))
                        .unwrap_or(false)
                })
                .cloned()
                .collect();
        }
        if !self.items.is_empty() {
            self.state.select(Some(0));
        } else {
            self.state.select(None);
        }
    }

    fn start_search(&mut self) {
        self.searching = true;
        self.query.clear();
    }

    fn stop_search(&mut self) {
        self.searching = false;
        self.query.clear();
        self.apply_filter();
    }

    fn append_query(&mut self, ch: char) {
        self.query.push(ch);
        self.apply_filter();
    }

    fn pop_query(&mut self) {
        self.query.pop();
        self.apply_filter();
    }
}

struct Model {
    screen: Screen,
    focus: Focus,
    input: Option<PathBuf>,
    output_dir: Option<PathBuf>,
    from: crate::Dialect,
    to: crate::Dialect,
    dry_run: bool,
    status: String,
    picker: Option<Picker>,
    summary: Vec<(PathBuf, PathBuf)>,
    pending_dir_selection: Option<PathBuf>,
}

impl Default for Model {
    fn default() -> Self {
        Model {
            screen: Screen::Main,
            focus: Focus::Input,
            input: None,
            output_dir: None,
            from: crate::Dialect::Mysql,
            to: crate::Dialect::Postgres,
            dry_run: true,
            status: "Ready".to_string(),
            picker: None,
            summary: Vec::new(),
            pending_dir_selection: None,
        }
    }
}

fn dialect_label(d: crate::Dialect) -> &'static str {
    match d {
        crate::Dialect::Mysql => "mysql",
        crate::Dialect::Postgres => "postgres",
        crate::Dialect::Sqlite => "sqlite",
    }
}

fn maybe_focus_run(model: &mut Model) {
    if model.input.is_some() && model.output_dir.is_some() {
        model.focus = Focus::Run;
    }
}

fn find_project_root() -> PathBuf {
    if let Ok(mut cwd) = std::env::current_dir() {
        for _ in 0..20 {
            if cwd.join(".git").exists() || cwd.join("Cargo.toml").exists() {
                return cwd;
            }
            if !cwd.pop() {
                break;
            }
        }
    }
    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

fn nearest_existing_dir(p: &Path) -> PathBuf {
    let mut cur = p.to_path_buf();
    while !cur.exists() {
        if !cur.pop() {
            break;
        }
    }
    if cur.exists() && cur.is_dir() {
        cur
    } else {
        find_project_root()
    }
}

fn update_summary(model: &mut Model) {
    model.summary.clear();
    if let (Some(inpath), Some(outdir)) = (&model.input, &model.output_dir) {
        match crate::plan_conversion(model.from, model.to, inpath, outdir) {
            Ok(list) => {
                model.summary = list;
                model.status = format!("Plan: {} files", model.summary.len());
            }
            Err(e) => model.status = format!("Plan error: {e}"),
        }
    }
}

fn run_conversion_now(model: &mut Model) {
    model.status = "Running...".to_string();
    if model.input.is_none() || model.output_dir.is_none() {
        model.status = "Select input and output first".to_string();
        return;
    }
    let inpath = model.input.as_ref().unwrap().clone();
    let outdir = model.output_dir.as_ref().unwrap().clone();
    if model.dry_run {
        update_summary(model);
        model.status = format!("Dry-run: {} files", model.summary.len());
        return;
    }

    // perform conversion
    let res = if inpath.is_dir() {
        crate::convert_dir(model.from, model.to, &inpath, &outdir)
    } else {
        // single file -> compute output file path
        let stem = inpath
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("schema");
        let out = outdir.join(format!("{}.{}.sql", stem, dialect_label(model.to)));
        crate::convert_file(model.from, model.to, &inpath, &out)
    };

    match res {
        Ok(()) => model.status = "Conversion complete".to_string(),
        Err(e) => model.status = format!("Conversion failed: {e}"),
    }
}

pub fn run_tui() -> Result<(), String> {
    enable_raw_mode().map_err(|e| e.to_string())?;
    let mut stdout = std::io::stdout();
    stdout
        .execute(EnterAlternateScreen)
        .map_err(|e| e.to_string())?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend).map_err(|e| e.to_string())?;

    let mut model = Model::default();

    // default pickers anchored at project root
    let project = find_project_root();
    model.picker = None;
    model.input = None;
    model.output_dir = Some(project.clone());

    let tick_rate = Duration::from_millis(150);
    loop {
        terminal
            .draw(|f| ui(f, &mut model))
            .map_err(|e| e.to_string())?;

        // handle input
        if crossterm::event::poll(tick_rate).map_err(|e| e.to_string())? {
            match event::read().map_err(|e| e.to_string())? {
                Event::Key(key) => match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => {
                        // exit
                        break;
                    }
                    KeyCode::Char('i') | KeyCode::Char('I') => {
                        let start = model.input.clone().unwrap_or_else(find_project_root);
                        model.picker =
                            Some(Picker::new_at(nearest_existing_dir(&start), PickKind::File));
                        model.screen = Screen::PickInput;
                    }
                    KeyCode::Char('o') | KeyCode::Char('O') => {
                        let start = model.output_dir.clone().unwrap_or_else(find_project_root);
                        model.picker =
                            Some(Picker::new_at(nearest_existing_dir(&start), PickKind::Dir));
                        model.screen = Screen::PickOutputDir;
                    }
                    KeyCode::Char('r') | KeyCode::Char('R') => {
                        run_conversion_now(&mut model);
                    }
                    KeyCode::Char('x') | KeyCode::Char('X') => {
                        std::mem::swap(&mut model.from, &mut model.to);
                        model.status = "Swapped dialects".to_string();
                        update_summary(&mut model);
                    }
                    KeyCode::Char('d') | KeyCode::Char('D') => {
                        model.dry_run = !model.dry_run;
                        model.status = if model.dry_run {
                            "Dry-run enabled".to_string()
                        } else {
                            "Dry-run disabled".to_string()
                        };
                        update_summary(&mut model);
                    }
                    KeyCode::Left => {
                        model.focus = match model.focus {
                            Focus::Input => Focus::Run,
                            Focus::OutputDir => Focus::Input,
                            Focus::From => Focus::OutputDir,
                            Focus::To => Focus::From,
                            Focus::Run => Focus::To,
                        }
                    }
                    KeyCode::Right => {
                        model.focus = match model.focus {
                            Focus::Input => Focus::OutputDir,
                            Focus::OutputDir => Focus::From,
                            Focus::From => Focus::To,
                            Focus::To => Focus::Run,
                            Focus::Run => Focus::Input,
                        }
                    }
                    _ => {}
                },
                Event::Mouse(_) => {}
                Event::FocusGained | Event::FocusLost => {}
                _ => {}
            }
        }

        // inner picker and confirm handling
        match model.screen {
            Screen::PickInput | Screen::PickOutputDir => {
                if let Some(ref mut picker) = model.picker {
                    if crossterm::event::poll(Duration::from_millis(10))
                        .map_err(|e| e.to_string())?
                    {
                        if let Event::Key(k) = event::read().map_err(|e| e.to_string())? {
                            match k.code {
                                KeyCode::Char('q') | KeyCode::Esc if !picker.searching => {
                                    model.picker = None;
                                    model.screen = Screen::Main;
                                }
                                KeyCode::Up => picker.move_up(),
                                KeyCode::Down => picker.move_down(),
                                KeyCode::PageUp => picker.page_up(),
                                KeyCode::PageDown => picker.page_down(),
                                KeyCode::Home => picker.select_first(),
                                KeyCode::End => picker.select_last(),
                                KeyCode::Backspace | KeyCode::Left if !picker.searching => {
                                    picker.go_parent()
                                }
                                KeyCode::Char('/') => {
                                    picker.start_search();
                                }
                                KeyCode::Char(c) if picker.searching => {
                                    picker.append_query(c);
                                }
                                KeyCode::Backspace if picker.searching => picker.pop_query(),
                                KeyCode::Esc if picker.searching => picker.stop_search(),
                                KeyCode::Right | KeyCode::Enter => {
                                    // If selected is a file and picking files, accept it on Enter
                                    if let Some(idx) = picker.state.selected() {
                                        if let Some(p) = picker.items.get(idx) {
                                            if p.is_file() {
                                                if model.screen == Screen::PickInput {
                                                    model.input = Some(p.clone());
                                                    model.picker = None;
                                                    model.screen = Screen::Main;
                                                    maybe_focus_run(&mut model);
                                                    update_summary(&mut model);
                                                    continue;
                                                }
                                            }
                                        }
                                    }
                                    picker.enter()
                                }
                                KeyCode::Char('s') | KeyCode::Char('S') => {
                                    // select
                                    if let Some(sel) = picker.selected_path() {
                                        if sel.is_dir() {
                                            // confirm whether to convert entire folder or pick individual files
                                            model.pending_dir_selection = Some(sel.clone());
                                            model.picker = None;
                                            model.screen = Screen::ConfirmDir;
                                        } else {
                                            match model.screen {
                                                Screen::PickInput => model.input = Some(sel),
                                                Screen::PickOutputDir => {
                                                    model.output_dir = Some(sel)
                                                }
                                                _ => {}
                                            }
                                            model.picker = None;
                                            model.screen = Screen::Main;
                                            maybe_focus_run(&mut model);
                                            update_summary(&mut model);
                                        }
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                } else {
                    model.screen = Screen::Main;
                }
            }
            Screen::ConfirmDir => {
                if let Some(dir) = model.pending_dir_selection.clone() {
                    if crossterm::event::poll(Duration::from_millis(10))
                        .map_err(|e| e.to_string())?
                    {
                        if let Event::Key(k) = event::read().map_err(|e| e.to_string())? {
                            match k.code {
                                KeyCode::Char('a') | KeyCode::Char('A') => {
                                    // select entire folder -> set input dir and default output to <dir>/xsql
                                    model.input = Some(dir.clone());
                                    model.output_dir = Some(dir.join("xsql"));
                                    model.pending_dir_selection = None;
                                    model.screen = Screen::Main;
                                    maybe_focus_run(&mut model);
                                    update_summary(&mut model);
                                }
                                KeyCode::Char('f') | KeyCode::Char('F') => {
                                    // pick individual files inside this folder
                                    model.picker =
                                        Some(Picker::new_at(dir.clone(), PickKind::File));
                                    model.pending_dir_selection = None;
                                    model.screen = Screen::PickInput;
                                }
                                KeyCode::Char('q') | KeyCode::Esc => {
                                    model.pending_dir_selection = None;
                                    model.screen = Screen::Main;
                                }
                                _ => {}
                            }
                        }
                    }
                } else {
                    model.screen = Screen::Main;
                }
            }
            Screen::Main => {}
        }
    }

    // restore terminal
    disable_raw_mode().map_err(|e| e.to_string())?;
    terminal
        .backend_mut()
        .execute(LeaveAlternateScreen)
        .map_err(|e| e.to_string())?;
    terminal.show_cursor().map_err(|e| e.to_string())?;
    Ok(())
}

fn ui(f: &mut Frame<'_>, model: &mut Model) {
    let size = f.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            [
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Min(3),
            ]
            .as_ref(),
        )
        .split(size);

    // top row: input / output
    let top = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
        .split(chunks[0]);

    let input_text = match &model.input {
        Some(p) => p.display().to_string(),
        None => "(none)".to_string(),
    };
    let out_text = match &model.output_dir {
        Some(p) => p.display().to_string(),
        None => "(none)".to_string(),
    };

    let input_par =
        Paragraph::new(input_text).block(Block::default().borders(Borders::ALL).title("Input (i)"));
    let out_par =
        Paragraph::new(out_text).block(Block::default().borders(Borders::ALL).title("Output (o)"));
    f.render_widget(input_par, top[0]);
    f.render_widget(out_par, top[1]);

    // middle: dialects and actions
    let mid = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(
            [
                Constraint::Percentage(30),
                Constraint::Percentage(30),
                Constraint::Percentage(40),
            ]
            .as_ref(),
        )
        .split(chunks[1]);

    let from = Paragraph::new(dialect_label(model.from)).block(
        Block::default()
            .borders(Borders::ALL)
            .title("From (x to swap)"),
    );
    let to = Paragraph::new(dialect_label(model.to))
        .block(Block::default().borders(Borders::ALL).title("To"));
    let run = Paragraph::new(format!("Run (r) - dry: {}", model.dry_run)).block(
        Block::default()
            .borders(Borders::ALL)
            .title("Run (r/d toggle)"),
    );
    f.render_widget(from, mid[0]);
    f.render_widget(to, mid[1]);
    f.render_widget(run, mid[2]);

    // bottom: status, summary, picker or confirm dialog
    match model.screen {
        Screen::Main => {
            let status = Paragraph::new(model.status.clone())
                .block(Block::default().borders(Borders::ALL).title("Status"));
            f.render_widget(status, chunks[2]);
        }
        Screen::PickInput | Screen::PickOutputDir => {
            if let Some(ref mut picker) = model.picker {
                let areas = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([Constraint::Length(1), Constraint::Min(3)].as_ref())
                    .split(chunks[2]);

                let hint = if picker.searching {
                    format!(
                        "Search: {} (type to filter, Backspace to erase, Esc to exit)",
                        picker.query
                    )
                } else {
                    "Picker - / search, Enter open/select, s select, q cancel".to_string()
                };

                let hint_par = Paragraph::new(hint).block(Block::default().borders(Borders::NONE));
                f.render_widget(hint_par, areas[0]);

                let items: Vec<ListItem> = picker
                    .items
                    .iter()
                    .map(|p| ListItem::new(p.file_name().and_then(|s| s.to_str()).unwrap_or("?")))
                    .collect();
                let mut list =
                    List::new(items).block(Block::default().borders(Borders::ALL).title(
                        "Files/Dirs - Use arrows, PgUp/PgDn, Home/End, Backspace to parent",
                    ));
                list = list.style(Style::default());
                f.render_stateful_widget(list, areas[1], &mut picker.state);
            }
        }
        Screen::ConfirmDir => {
            if let Some(ref d) = model.pending_dir_selection {
                let msg = format!(
                    "Directory selected: {} — press (a) all files -> output: {}/xsql, (f) pick individual files, (q) cancel",
                    d.display(),
                    d.display()
                );
                let par = Paragraph::new(msg).block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("Confirm directory"),
                );
                f.render_widget(par, chunks[2]);
            } else {
                let status = Paragraph::new("No directory selected")
                    .block(Block::default().borders(Borders::ALL).title("Status"));
                f.render_widget(status, chunks[2]);
            }
        }
    }
}
