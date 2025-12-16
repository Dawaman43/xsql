use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use crossterm::ExecutableCommand;
use crossterm::event::{self, Event, KeyCode};
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::prelude::*;
use ratatui::widgets::*;

use crate::{Dialect, convert_dir, convert_file};

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
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PickKind {
    Input,
    OutputDir,
}

struct Picker {
    kind: PickKind,
    cwd: PathBuf,
    entries: Vec<PathBuf>,
    selected: usize,
    status: String,
}

impl Picker {
    fn new_at(kind: PickKind, cwd: PathBuf) -> Self {
        let mut picker = Self {
            kind,
            cwd,
            entries: vec![],
            selected: 0,
            status: String::new(),
        };
        picker.refresh();
        picker
    }

    fn refresh(&mut self) {
        let mut dirs = Vec::<PathBuf>::new();
        let mut files = Vec::<PathBuf>::new();

        match fs::read_dir(&self.cwd) {
            Ok(rd) => {
                for entry in rd.flatten() {
                    let path = entry.path();
                    if path.is_dir() {
                        dirs.push(path);
                    } else if self.kind == PickKind::Input {
                        if path
                            .extension()
                            .and_then(|s| s.to_str())
                            .is_some_and(|ext| ext.eq_ignore_ascii_case("sql"))
                        {
                            files.push(path);
                        }
                    }
                }
            }
            Err(e) => {
                self.status = format!("Cannot read {}: {e}", self.cwd.display());
            }
        }

        dirs.sort();
        files.sort();

        let mut entries = Vec::with_capacity(1 + dirs.len() + files.len());
        entries.push(self.cwd.join(".."));
        entries.extend(dirs);
        if self.kind == PickKind::Input {
            entries.extend(files);
        }

        self.entries = entries;
        self.selected = self.selected.min(self.entries.len().saturating_sub(1));
    }

    fn selected_path(&self) -> Option<&Path> {
        self.entries.get(self.selected).map(|p| p.as_path())
    }

    fn move_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
    }

    fn move_down(&mut self) {
        if self.selected + 1 < self.entries.len() {
            self.selected += 1;
        }
    }

    fn go_parent(&mut self) {
        if let Some(parent) = self.cwd.parent() {
            self.cwd = parent.to_path_buf();
            self.selected = 0;
            self.refresh();
        }
    }

    fn enter_dir(&mut self, dir: &Path) {
        self.cwd = dir.to_path_buf();
        self.selected = 0;
        self.refresh();
    }
}

struct Model {
    screen: Screen,
    focus: Focus,
    input: Option<PathBuf>,
    output_dir: Option<PathBuf>,
    from: Dialect,
    to: Dialect,
    summary: String,
    status: String,
    picker: Picker,
    confirm_create_output_dir: Option<PathBuf>,
}

impl Default for Model {
    fn default() -> Self {
        let project_root = find_project_root();
        Self {
            screen: Screen::Main,
            focus: Focus::Input,
            input: None,
            output_dir: None,
            from: Dialect::Mysql,
            to: Dialect::Postgres,
            summary: "Mode: (pick input)".to_string(),
            status: "Enter: pick • Tab: next • i/o open pickers • r run • x swap • Esc quit"
                .to_string(),
            picker: Picker::new_at(PickKind::Input, project_root),
            confirm_create_output_dir: None,
        }
    }
}

fn maybe_focus_run(model: &mut Model) {
    if model.input.is_some() && model.output_dir.is_some() {
        model.focus = Focus::Run;
    }
}

fn find_project_root() -> PathBuf {
    let start = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let mut cur = start.as_path();
    loop {
        let git = cur.join(".git");
        if git.is_dir() {
            return cur.to_path_buf();
        }
        match cur.parent() {
            Some(parent) => cur = parent,
            None => return start,
        }
    }
}

fn nearest_existing_dir(path: &Path) -> PathBuf {
    if path.is_dir() {
        return path.to_path_buf();
    }
    let mut cur = path;
    loop {
        if cur.is_dir() {
            return cur.to_path_buf();
        }
        match cur.parent() {
            Some(parent) => cur = parent,
            None => return PathBuf::from("."),
        }
    }
}

fn dialect_label(d: Dialect) -> &'static str {
    match d {
        Dialect::Mysql => "mysql",
        Dialect::Postgres => "postgres",
        Dialect::Sqlite => "sqlite",
    }
}

fn next_focus(f: Focus) -> Focus {
    match f {
        Focus::Input => Focus::OutputDir,
        Focus::OutputDir => Focus::From,
        Focus::From => Focus::To,
        Focus::To => Focus::Run,
        Focus::Run => Focus::Input,
    }
}

fn prev_focus(f: Focus) -> Focus {
    match f {
        Focus::Input => Focus::Run,
        Focus::OutputDir => Focus::Input,
        Focus::From => Focus::OutputDir,
        Focus::To => Focus::From,
        Focus::Run => Focus::To,
    }
}

fn next_dialect(d: Dialect) -> Dialect {
    match d {
        Dialect::Mysql => Dialect::Postgres,
        Dialect::Postgres => Dialect::Sqlite,
        Dialect::Sqlite => Dialect::Mysql,
    }
}

fn prev_dialect(d: Dialect) -> Dialect {
    match d {
        Dialect::Mysql => Dialect::Sqlite,
        Dialect::Postgres => Dialect::Mysql,
        Dialect::Sqlite => Dialect::Postgres,
    }
}

fn input_mode_label(input: Option<&PathBuf>) -> &'static str {
    match input {
        Some(p) if p.is_dir() => "folder",
        Some(_) => "file",
        None => "(pick input)",
    }
}

fn is_sql_file(path: &Path) -> bool {
    path.extension()
        .and_then(|s| s.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("sql"))
}

fn count_sql_files_recursive(dir: &Path) -> Result<usize, String> {
    if !dir.is_dir() {
        return Ok(0);
    }
    let mut count = 0usize;
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        let rd =
            fs::read_dir(&d).map_err(|e| format!("failed to read dir {}: {e}", d.display()))?;
        for entry in rd {
            let entry = entry.map_err(|e| e.to_string())?;
            let p = entry.path();
            if p.is_dir() {
                stack.push(p);
            } else if is_sql_file(&p) {
                count += 1;
            }
        }
    }
    Ok(count)
}

fn update_summary(model: &mut Model) {
    let Some(input) = model.input.as_ref() else {
        model.summary = "Mode: (pick input)".to_string();
        return;
    };

    if input.is_dir() {
        let n = count_sql_files_recursive(input).unwrap_or(0);
        let out = model
            .output_dir
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "(pick output folder)".to_string());
        model.summary = format!("Mode: folder • {n} .sql files • Output: {out}");
    } else {
        let out_dir = model
            .output_dir
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "(pick output folder)".to_string());
        model.summary = format!(
            "Mode: file • Output folder: {out_dir} • Output name: <stem>.{}.sql",
            dialect_label(model.to)
        );
    }
}

fn run_conversion_now(model: &mut Model) {
    update_summary(model);
    let Some(input) = model.input.clone() else {
        model.status = "Pick an input first".to_string();
        return;
    };
    let Some(output_dir) = model.output_dir.clone() else {
        model.status = "Pick an output folder first".to_string();
        return;
    };

    let result = if input.is_dir() {
        convert_dir(model.from, model.to, &input, &output_dir)
    } else {
        let stem = input
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("schema");
        let out_file = output_dir.join(format!("{}.{}.sql", stem, dialect_label(model.to)));
        convert_file(model.from, model.to, &input, &out_file)
    };

    model.status = match result {
        Ok(()) => "Conversion complete".to_string(),
        Err(e) => format!("Error: {e}"),
    };
}

fn run_conversion(model: &mut Model) {
    update_summary(model);
    let Some(output_dir) = model.output_dir.clone() else {
        model.status = "Pick an output folder first".to_string();
        return;
    };

    if !output_dir.exists() {
        model.confirm_create_output_dir = Some(output_dir);
        model.status =
            "Output folder does not exist. Press y to create + run, n to cancel".to_string();
        return;
    }

    model.confirm_create_output_dir = None;
    run_conversion_now(model);
}

fn format_path_opt(p: &Option<PathBuf>) -> String {
    p.as_ref()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| "(not set)".to_string())
}

fn default_output_dir_for_input(input: &Path, to: Dialect) -> PathBuf {
    if input.is_dir() {
        let name = input
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("schemas");
        let parent = input.parent().unwrap_or_else(|| Path::new("."));
        parent.join(format!("{}_to_{}", name, dialect_label(to)))
    } else {
        input
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .to_path_buf()
    }
}

fn ui_picker(frame: &mut Frame, model: &Model) {
    let area = frame.area();
    let outer = Block::default()
        .borders(Borders::ALL)
        .title(match model.picker.kind {
            PickKind::Input => "Pick input (.sql file or folder)",
            PickKind::OutputDir => "Pick output folder",
        });

    let inner = outer.inner(area);
    frame.render_widget(outer, area);
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([Constraint::Min(1), Constraint::Length(3)])
        .split(inner);

    let items: Vec<ListItem> = model
        .picker
        .entries
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let label = if i == 0 {
                "..".to_string()
            } else {
                let name = p
                    .file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or("?")
                    .to_string();
                if p.is_dir() {
                    format!("{}/", name)
                } else {
                    name
                }
            };
            ListItem::new(label)
        })
        .collect();

    let mut state = ListState::default();
    state.select(Some(model.picker.selected));

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(model.picker.cwd.display().to_string()),
        )
        .highlight_style(Style::default().add_modifier(Modifier::BOLD))
        .highlight_symbol("> ");

    frame.render_stateful_widget(list, chunks[0], &mut state);

    let help = match model.picker.kind {
        PickKind::Input => {
            "↑/↓ move • Enter open dir / pick file • Space select folder • Backspace up • Esc cancel"
        }
        PickKind::OutputDir => "↑/↓ move • Enter/Space select folder • Backspace up • Esc cancel",
    };
    let footer = Paragraph::new(format!(
        "{}{}",
        help,
        if model.picker.status.is_empty() {
            ""
        } else {
            " • "
        }
    ))
    .block(Block::default().borders(Borders::ALL).title("Keys"));
    frame.render_widget(footer, chunks[1]);
}

fn ui(frame: &mut Frame, model: &Model) {
    if model.screen != Screen::Main {
        ui_picker(frame, model);
        return;
    }

    let area = frame.area();

    let layout = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
        ])
        .split(area);

    let highlight = Style::default().add_modifier(Modifier::BOLD);

    let input = Paragraph::new(format_path_opt(&model.input))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Input (Enter to pick)")
                .title_bottom(format!("Mode: {}", input_mode_label(model.input.as_ref()))),
        )
        .style(if model.focus == Focus::Input {
            highlight
        } else {
            Style::default()
        });

    let output = Paragraph::new(format_path_opt(&model.output_dir))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Output folder (Enter to pick)"),
        )
        .style(if model.focus == Focus::OutputDir {
            highlight
        } else {
            Style::default()
        });

    let from = Paragraph::new(dialect_label(model.from))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("From dialect (←/→ or ↑/↓)"),
        )
        .style(if model.focus == Focus::From {
            highlight
        } else {
            Style::default()
        });

    let to = Paragraph::new(dialect_label(model.to))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("To dialect (←/→ or ↑/↓)"),
        )
        .style(if model.focus == Focus::To {
            highlight
        } else {
            Style::default()
        });

    let run = Paragraph::new("Press Enter to run")
        .block(Block::default().borders(Borders::ALL).title("Run"))
        .style(if model.focus == Focus::Run {
            highlight
        } else {
            Style::default()
        });

    frame.render_widget(input, layout[0]);
    frame.render_widget(output, layout[1]);
    frame.render_widget(from, layout[2]);
    frame.render_widget(to, layout[3]);

    frame.render_widget(run, layout[4]);

    let status_text = if model.confirm_create_output_dir.is_some() {
        format!("{}\n{}", model.summary, model.status)
    } else {
        format!("{}\n{}", model.summary, model.status)
    };

    let status =
        Paragraph::new(status_text).block(Block::default().borders(Borders::ALL).title("Status"));
    frame.render_widget(status, layout[5]);
}

fn open_picker(model: &mut Model, kind: PickKind) {
    let project_root = find_project_root();
    let start_dir = match kind {
        PickKind::Input => project_root,
        PickKind::OutputDir => {
            if let Some(output_dir) = model.output_dir.as_ref() {
                nearest_existing_dir(output_dir)
            } else if let Some(input) = model.input.as_ref() {
                let suggested = default_output_dir_for_input(input, model.to);
                nearest_existing_dir(&suggested)
            } else {
                project_root
            }
        }
    };

    model.picker = Picker::new_at(kind, start_dir);
    model.screen = match kind {
        PickKind::Input => Screen::PickInput,
        PickKind::OutputDir => Screen::PickOutputDir,
    };
    model.status = match kind {
        PickKind::Input => "Pick an input file or folder".to_string(),
        PickKind::OutputDir => "Pick an output folder".to_string(),
    };
}

fn picker_confirm(model: &mut Model) {
    let Some(sel) = model.picker.selected_path().map(|p| p.to_path_buf()) else {
        return;
    };

    // First entry is always ".."
    if model.picker.selected == 0 {
        model.picker.go_parent();
        return;
    }

    match model.picker.kind {
        PickKind::Input => {
            if sel.is_dir() {
                // Enter navigates into folders in input picker.
                model.picker.enter_dir(&sel);
            } else {
                model.input = Some(sel.clone());
                model.output_dir = Some(default_output_dir_for_input(&sel, model.to));
                model.screen = Screen::Main;
                model.status =
                    "Picked input file. Output folder auto-set (Enter to run, Tab to change)."
                        .to_string();
                update_summary(model);
                maybe_focus_run(model);
            }
        }
        PickKind::OutputDir => {
            if sel.is_dir() {
                model.output_dir = Some(sel);
                model.screen = Screen::Main;
                model.status = "Picked output folder".to_string();
                update_summary(model);
                maybe_focus_run(model);
            } else {
                model.status = "Select a folder".to_string();
            }
        }
    }
}

fn picker_select_folder(model: &mut Model) {
    // Space selects current highlighted folder in input picker.
    let Some(sel) = model.picker.selected_path().map(|p| p.to_path_buf()) else {
        return;
    };

    if model.picker.selected == 0 {
        model.picker.go_parent();
        return;
    }

    if !sel.is_dir() {
        model.status = "Select a folder".to_string();
        return;
    }

    match model.picker.kind {
        PickKind::Input => {
            model.input = Some(sel.clone());
            model.output_dir = Some(default_output_dir_for_input(&sel, model.to));
            model.screen = Screen::Main;
            model.status =
                "Picked input folder. Output folder auto-set (Enter to run, Tab to change)."
                    .to_string();
            update_summary(model);
            maybe_focus_run(model);
        }
        PickKind::OutputDir => {
            model.output_dir = Some(sel);
            model.screen = Screen::Main;
            model.status = "Picked output folder".to_string();
            update_summary(model);
            maybe_focus_run(model);
        }
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

    let res = loop {
        terminal
            .draw(|f| ui(f, &model))
            .map_err(|e| e.to_string())?;

        if event::poll(Duration::from_millis(50)).map_err(|e| e.to_string())? {
            match event::read().map_err(|e| e.to_string())? {
                Event::Key(key) => {
                    if key.code == KeyCode::Esc {
                        if model.screen != Screen::Main {
                            model.screen = Screen::Main;
                            model.status = "Cancelled picker".to_string();
                        } else {
                            break Ok(());
                        }
                        continue;
                    }

                    if model.screen != Screen::Main {
                        match key.code {
                            KeyCode::Up => model.picker.move_up(),
                            KeyCode::Down => model.picker.move_down(),
                            KeyCode::Backspace => model.picker.go_parent(),
                            KeyCode::Enter => picker_confirm(&mut model),
                            KeyCode::Char(' ') | KeyCode::Char('s') => {
                                picker_select_folder(&mut model)
                            }
                            _ => {}
                        }
                        continue;
                    }

                    if let Some(dir) = model.confirm_create_output_dir.clone() {
                        match key.code {
                            KeyCode::Char('y') | KeyCode::Char('Y') => {
                                match fs::create_dir_all(&dir) {
                                    Ok(()) => {
                                        model.confirm_create_output_dir = None;
                                        model.status =
                                            "Created output folder. Running…".to_string();
                                        run_conversion_now(&mut model);
                                    }
                                    Err(e) => {
                                        model.confirm_create_output_dir = None;
                                        model.status =
                                            format!("Failed to create {}: {e}", dir.display());
                                    }
                                }
                            }
                            KeyCode::Char('n') | KeyCode::Char('N') => {
                                model.confirm_create_output_dir = None;
                                model.status = "Cancelled".to_string();
                            }
                            _ => {}
                        }
                        continue;
                    }

                    match key.code {
                        KeyCode::Tab => model.focus = next_focus(model.focus),
                        KeyCode::BackTab => model.focus = prev_focus(model.focus),
                        KeyCode::Char('i') | KeyCode::Char('I') => {
                            open_picker(&mut model, PickKind::Input)
                        }
                        KeyCode::Char('o') | KeyCode::Char('O') => {
                            open_picker(&mut model, PickKind::OutputDir)
                        }
                        KeyCode::Char('r') | KeyCode::Char('R') => run_conversion(&mut model),
                        KeyCode::Char('x') | KeyCode::Char('X') => {
                            std::mem::swap(&mut model.from, &mut model.to);
                            model.status = "Swapped dialects".to_string();
                            update_summary(&mut model);
                        }
                        KeyCode::Enter => {
                            if model.focus == Focus::Run {
                                run_conversion(&mut model);
                            } else {
                                match model.focus {
                                    Focus::Input => open_picker(&mut model, PickKind::Input),
                                    Focus::OutputDir => {
                                        open_picker(&mut model, PickKind::OutputDir)
                                    }
                                    _ => model.focus = next_focus(model.focus),
                                }
                            }
                        }
                        KeyCode::Left | KeyCode::Up => match model.focus {
                            Focus::From => model.from = prev_dialect(model.from),
                            Focus::To => model.to = prev_dialect(model.to),
                            _ => {}
                        },
                        KeyCode::Right | KeyCode::Down => match model.focus {
                            Focus::From => model.from = next_dialect(model.from),
                            Focus::To => model.to = next_dialect(model.to),
                            _ => {}
                        },
                        _ => {}
                    }
                }
                Event::Resize(_, _) => {}
                _ => {}
            }
        }
    };

    disable_raw_mode().map_err(|e| e.to_string())?;
    let mut stdout = std::io::stdout();
    stdout
        .execute(LeaveAlternateScreen)
        .map_err(|e| e.to_string())?;

    res
}
