use std::{collections::HashSet, io, path::PathBuf};

use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap},
    Frame, Terminal,
};

use crate::parser::{ValidatedFunctionSignature, ValidatedReturn};
use crate::{
    codegen::{generate, SelectedFunction},
    parser::parse_b_file,
};

const C_ACCENT: Color = Color::Cyan;
const C_SELECTED: Color = Color::Yellow;
const C_DIM: Color = Color::DarkGray;
const C_OK: Color = Color::Green;
const C_ERR: Color = Color::Red;
const C_WARN: Color = Color::LightYellow;

#[derive(Debug, Clone, PartialEq)]
enum Screen {
    FilePicker,
    Prefix,
    FunctionPicker,
    Rename,
    OutputName,
    Confirm,
    Done,
}

struct FilePicker {
    files: Vec<PathBuf>,
    list_state: ListState,
    selected: HashSet<usize>,
}

struct FunctionEntry {
    file: PathBuf,
    sig: ValidatedFunctionSignature,
    selected: bool,
    export_name: String,
    renamed: bool,
}

struct FunctionPicker {
    entries: Vec<FunctionEntry>,
    list_state: ListState,
}

pub struct App {
    screen: Screen,
    file_picker: FilePicker,
    fn_picker: FunctionPicker,
    rename_idx: Option<usize>,
    rename_buf: String,
    prefix_buf: String,
    output_stem: String,
    output_dir: PathBuf,
    status: String,
}

impl App {
    pub fn new(files: Vec<PathBuf>, output_dir: PathBuf) -> Self {
        let mut fp_state = ListState::default();
        if !files.is_empty() {
            fp_state.select(Some(0));
        }

        App {
            screen: Screen::FilePicker,
            file_picker: FilePicker {
                files,
                list_state: fp_state,
                selected: HashSet::new(),
            },
            fn_picker: FunctionPicker {
                entries: Vec::new(),
                list_state: ListState::default(),
            },
            rename_idx: None,
            rename_buf: String::new(),
            prefix_buf: String::new(),
            output_stem: "output".to_string(),
            output_dir,
            status: String::new(),
        }
    }

    pub fn handle_key(&mut self, key: KeyCode, modifiers: KeyModifiers) {
        if key == KeyCode::Char('q') && modifiers == KeyModifiers::CONTROL {
            self.screen = Screen::Done;
            return;
        }

        match self.screen {
            Screen::FilePicker => self.handle_file_picker(key),
            Screen::Prefix => self.handle_prefix(key),
            Screen::FunctionPicker => self.handle_fn_picker(key),
            Screen::Rename => self.handle_rename(key),
            Screen::OutputName => self.handle_output_name(key),
            Screen::Confirm => self.handle_confirm(key),
            Screen::Done => {}
        }
    }

    fn handle_file_picker(&mut self, key: KeyCode) {
        let fp = &mut self.file_picker;
        match key {
            KeyCode::Down | KeyCode::Char('j') => {
                let next = fp
                    .list_state
                    .selected()
                    .map(|i| (i + 1).min(fp.files.len().saturating_sub(1)))
                    .unwrap_or(0);
                fp.list_state.select(Some(next));
            }
            KeyCode::Up | KeyCode::Char('k') => {
                let prev = fp
                    .list_state
                    .selected()
                    .map(|i| i.saturating_sub(1))
                    .unwrap_or(0);
                fp.list_state.select(Some(prev));
            }
            KeyCode::Char(' ') => {
                if let Some(i) = fp.list_state.selected() {
                    if fp.selected.contains(&i) {
                        fp.selected.remove(&i);
                    } else {
                        fp.selected.insert(i);
                    }
                }
            }
            KeyCode::Enter => {
                if fp.selected.is_empty() {
                    self.status = "Select at least one file (Space to toggle).".to_string();
                    return;
                }
                self.load_functions();

                self.screen = Screen::Prefix;
            }
            KeyCode::Char('a') => {
                if fp.selected.len() == fp.files.len() {
                    fp.selected.clear();
                } else {
                    fp.selected = (0..fp.files.len()).collect();
                }
            }
            _ => {}
        }
    }

    fn handle_prefix(&mut self, key: KeyCode) {
        match key {
            KeyCode::Enter => {
                self.apply_prefix();
                self.screen = Screen::FunctionPicker;
                self.status = if self.prefix_buf.is_empty() {
                    "No prefix set — you must rename every function before proceeding.".to_string()
                } else {
                    format!(
                        "Prefix '{}' applied. You may still rename individual functions.",
                        self.prefix_buf
                    )
                };
            }
            KeyCode::Esc => {
                self.screen = Screen::FilePicker;
            }
            KeyCode::Backspace => {
                self.prefix_buf.pop();
            }
            KeyCode::Char(c) => {
                self.prefix_buf.push(c);
            }
            _ => {}
        }
    }

    fn apply_prefix(&mut self) {
        let prefix = self.prefix_buf.trim().to_string();
        for e in self.fn_picker.entries.iter_mut() {
            e.export_name = if prefix.is_empty() {
                e.sig.name.clone()
            } else {
                format!("{}{}", prefix, e.sig.name)
            };
            e.renamed = !prefix.is_empty();
        }
    }

    fn load_functions(&mut self) {
        let fp = &self.file_picker;
        let mut entries: Vec<FunctionEntry> = Vec::new();

        for &idx in &fp.selected {
            let path = &fp.files[idx];
            match std::fs::read_to_string(path) {
                Ok(src) => {
                    let sigs = parse_b_file(&src);
                    for sig in sigs {
                        let export_name = sig.name.clone();
                        entries.push(FunctionEntry {
                            file: path.clone(),
                            sig,
                            selected: false,
                            export_name,
                            renamed: false,
                        });
                    }
                }
                Err(e) => {
                    self.status = format!("Error reading {:?}: {}", path, e);
                }
            }
        }

        if entries.is_empty() {
            self.status = "No function signatures found in selected files.".to_string();
        }

        let mut ls = ListState::default();
        if !entries.is_empty() {
            ls.select(Some(0));
        }
        self.fn_picker = FunctionPicker {
            entries,
            list_state: ls,
        };

        self.prefix_buf.clear();
    }

    fn handle_fn_picker(&mut self, key: KeyCode) {
        let fp = &mut self.fn_picker;
        match key {
            KeyCode::Down | KeyCode::Char('j') => {
                let n = fp.entries.len();
                if n > 0 {
                    let next = fp
                        .list_state
                        .selected()
                        .map(|i| (i + 1).min(n - 1))
                        .unwrap_or(0);
                    fp.list_state.select(Some(next));
                }
            }
            KeyCode::Up | KeyCode::Char('k') => {
                let prev = fp
                    .list_state
                    .selected()
                    .map(|i| i.saturating_sub(1))
                    .unwrap_or(0);
                fp.list_state.select(Some(prev));
            }
            KeyCode::Char(' ') => {
                if let Some(i) = fp.list_state.selected() {
                    let entry = &fp.entries[i];
                    if !entry.renamed {
                        self.status = format!(
                            "'{}' must be renamed before it can be selected (press r).",
                            entry.sig.name
                        );
                        return;
                    }
                    fp.entries[i].selected = !fp.entries[i].selected;
                    self.status.clear();
                }
            }
            KeyCode::Char('r') => {
                if let Some(i) = fp.list_state.selected() {
                    self.rename_idx = Some(i);
                    self.rename_buf = fp.entries[i].export_name.clone();
                    self.screen = Screen::Rename;
                }
            }
            KeyCode::Char('p') => {
                self.screen = Screen::Prefix;
                self.status.clear();
            }
            KeyCode::Char('a') => {
                let all_renamed_selected =
                    fp.entries.iter().filter(|e| e.renamed).all(|e| e.selected);

                let unrenamed: Vec<&str> = fp
                    .entries
                    .iter()
                    .filter(|e| !e.renamed)
                    .map(|e| e.sig.name.as_str())
                    .collect();

                if !unrenamed.is_empty() {
                    self.status = format!(
                        "Cannot select all — {} function(s) still need renaming.",
                        unrenamed.len()
                    );
                }

                for e in fp.entries.iter_mut() {
                    if e.renamed {
                        e.selected = !all_renamed_selected;
                    }
                }
            }
            KeyCode::Enter => {
                let unrenamed_selected: Vec<&str> = fp
                    .entries
                    .iter()
                    .filter(|e| e.selected && !e.renamed)
                    .map(|e| e.sig.name.as_str())
                    .collect();

                if !unrenamed_selected.is_empty() {
                    self.status = format!(
                        "{} selected function(s) still need renaming.",
                        unrenamed_selected.len()
                    );
                    return;
                }

                let any = fp.entries.iter().any(|e| e.selected);
                if !any {
                    self.status = "Select at least one function (Space to toggle).".to_string();
                    return;
                }
                self.status = String::new();
                self.screen = Screen::OutputName;
            }
            KeyCode::Esc => {
                self.status = String::new();
                self.screen = Screen::FilePicker;
            }
            _ => {}
        }
    }

    fn handle_rename(&mut self, key: KeyCode) {
        match key {
            KeyCode::Enter => {
                if let Some(i) = self.rename_idx {
                    let name = self.rename_buf.trim().to_string();
                    if name.is_empty() {
                        self.status = "Name cannot be empty.".to_string();
                    } else if self.fn_picker.entries[i].sig.name == name {
                        self.status = "Name needs to be different.".to_string();
                    } else {
                        self.fn_picker.entries[i].export_name = name;
                        self.fn_picker.entries[i].renamed = true;
                        self.rename_idx = None;
                        self.screen = Screen::FunctionPicker;
                        self.status.clear();
                    }
                }
            }
            KeyCode::Esc => {
                self.rename_idx = None;
                self.screen = Screen::FunctionPicker;
            }
            KeyCode::Backspace => {
                self.rename_buf.pop();
            }
            KeyCode::Char(c) => {
                self.rename_buf.push(c);
            }
            _ => {}
        }
    }

    fn handle_output_name(&mut self, key: KeyCode) {
        match key {
            KeyCode::Enter => {
                let stem = self.output_stem.trim().to_string();
                if stem.is_empty() {
                    self.status = "Output name cannot be empty.".to_string();
                } else {
                    self.output_stem = stem;
                    self.screen = Screen::Confirm;
                }
            }
            KeyCode::Esc => {
                self.screen = Screen::FunctionPicker;
            }
            KeyCode::Backspace => {
                self.output_stem.pop();
            }
            KeyCode::Char(c) => {
                self.output_stem.push(c);
            }
            _ => {}
        }
    }

    fn handle_confirm(&mut self, key: KeyCode) {
        match key {
            KeyCode::Enter => {
                self.do_generate();
                self.screen = Screen::Done;
            }
            KeyCode::Esc => {
                self.screen = Screen::OutputName;
            }
            _ => {}
        }
    }

    fn do_generate(&mut self) {
        let selections: Vec<SelectedFunction> = self
            .fn_picker
            .entries
            .iter()
            .filter(|e| e.selected)
            .map(|e| SelectedFunction {
                sig: e.sig.clone(),
                export_name: e.export_name.clone(),
            })
            .collect();

        let header_name = format!("{}.h", self.output_stem);
        let files = generate(&selections, &header_name);

        let h_path = self.output_dir.join(&header_name);
        let c_path = self.output_dir.join(format!("{}.c", self.output_stem));

        match (
            std::fs::write(&h_path, &files.header),
            std::fs::write(&c_path, &files.source),
        ) {
            (Ok(()), Ok(())) => {
                self.status = format!("Generated {:?} and {:?}", h_path, c_path);
            }
            (Err(e), _) | (_, Err(e)) => {
                self.status = format!("Write error: {}", e);
            }
        }
    }

    pub fn is_done(&self) -> bool {
        self.screen == Screen::Done
    }

    pub fn render(&mut self, frame: &mut Frame) {
        match self.screen.clone() {
            Screen::FilePicker => self.render_file_picker(frame),
            Screen::Prefix => self.render_prefix(frame),
            Screen::FunctionPicker => self.render_fn_picker(frame),
            Screen::Rename => {
                self.render_fn_picker(frame);
                self.render_rename_popup(frame);
            }
            Screen::OutputName => self.render_output_name(frame),
            Screen::Confirm => self.render_confirm(frame),
            Screen::Done => self.render_done(frame),
        }
    }

    fn base_layout(frame: &Frame) -> (ratatui::layout::Rect, ratatui::layout::Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(3), Constraint::Length(2)])
            .split(frame.area());
        (chunks[0], chunks[1])
    }

    fn render_status(&self, frame: &mut Frame, area: ratatui::layout::Rect) {
        let style = if self.status.contains("Error")
            || self.status.contains("cannot")
            || self.status.contains("must")
            || self.status.contains("need")
        {
            Style::default().fg(C_ERR)
        } else if self.status.contains("applied") {
            Style::default().fg(C_OK)
        } else {
            Style::default().fg(C_DIM)
        };
        let p = Paragraph::new(self.status.clone())
            .style(style)
            .alignment(Alignment::Left);
        frame.render_widget(p, area);
    }

    fn render_file_picker(&mut self, frame: &mut Frame) {
        let (main, status_area) = Self::base_layout(frame);
        self.render_status(frame, status_area);

        let items: Vec<ListItem> = self
            .file_picker
            .files
            .iter()
            .enumerate()
            .map(|(i, p)| {
                let check = if self.file_picker.selected.contains(&i) {
                    Span::styled("[✓] ", Style::default().fg(C_OK))
                } else {
                    Span::styled("[ ] ", Style::default().fg(C_DIM))
                };
                let name = Span::raw(
                    p.file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_string(),
                );
                ListItem::new(Line::from(vec![check, name]))
            })
            .collect();

        let list = List::new(items)
            .block(
                Block::default()
                    .title(" 📂  Select .b files  [Space=toggle  a=all  Enter=next  Ctrl+q=quit] ")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(C_ACCENT)),
            )
            .highlight_style(Style::default().fg(C_SELECTED).add_modifier(Modifier::BOLD))
            .highlight_symbol("▶ ");

        frame.render_stateful_widget(list, main, &mut self.file_picker.list_state);
    }

    fn render_prefix(&mut self, frame: &mut Frame) {
        let (main, status_area) = Self::base_layout(frame);
        self.render_status(frame, status_area);

        let fn_count = self.fn_picker.entries.len();
        let preview_lines: String = self
            .fn_picker
            .entries
            .iter()
            .take(6)
            .map(|e| {
                let new_name = if self.prefix_buf.trim().is_empty() {
                    e.sig.name.clone()
                } else {
                    format!("{}{}", self.prefix_buf.trim(), e.sig.name)
                };
                format!("  {} → {}\n", e.sig.name, new_name)
            })
            .collect();

        let ellipsis = if fn_count > 6 {
            format!("  … and {} more\n", fn_count - 6)
        } else {
            String::new()
        };

        let text = format!(
            "Enter a prefix to apply to all {} function export names.\n\
             Leave empty to skip — you will then rename each function individually.\n\
             \n\
             Prefix: > {}_\n\
             \n\
             Preview:\n\
             {}{}\n\
             [Enter=apply  Esc=back to file picker]",
            fn_count, self.prefix_buf, preview_lines, ellipsis,
        );

        let p = Paragraph::new(text)
            .block(
                Block::default()
                    .title(" 🏷  Global prefix  [Enter=apply  Esc=back] ")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(C_ACCENT)),
            )
            .wrap(Wrap { trim: false });
        frame.render_widget(p, main);
    }

    fn render_fn_picker(&mut self, frame: &mut Frame) {
        let (main, status_area) = Self::base_layout(frame);
        self.render_status(frame, status_area);

        let items: Vec<ListItem> = self
            .fn_picker
            .entries
            .iter()
            .map(|e| {
                let check = if e.selected {
                    Span::styled("[✓] ", Style::default().fg(C_OK))
                } else {
                    Span::styled("[ ] ", Style::default().fg(C_DIM))
                };
                let file = Span::styled(
                    format!(
                        "{}: ",
                        e.file.file_name().unwrap_or_default().to_string_lossy()
                    ),
                    Style::default().fg(C_DIM),
                );
                let sig = Span::styled(
                    format_sig_short(&e.sig),
                    if e.renamed {
                        Style::default().fg(Color::White)
                    } else {
                        Style::default().fg(C_WARN)
                    },
                );
                let alias = if e.export_name != e.sig.name {
                    Span::styled(format!(" → {}", e.export_name), Style::default().fg(C_OK))
                } else if !e.renamed {
                    Span::styled(
                        " ⚠ needs rename",
                        Style::default().fg(C_ERR).add_modifier(Modifier::BOLD),
                    )
                } else {
                    Span::raw("")
                };
                ListItem::new(Line::from(vec![check, file, sig, alias]))
            })
            .collect();

        let unread_count = self.fn_picker.entries.iter().filter(|e| !e.renamed).count();
        let title = if unread_count > 0 {
            format!(
                " ƒ  Select functions — ⚠ {} need renaming  [Space=toggle  r=rename  p=prefix  a=all  Enter=next  Esc=back] ",
                unread_count
            )
        } else {
            " ƒ  Select functions  [Space=toggle  r=rename  p=prefix  a=all  Enter=next  Esc=back] "
                .to_string()
        };

        let list = List::new(items)
            .block(
                Block::default()
                    .title(title)
                    .borders(Borders::ALL)
                    .border_style(if unread_count > 0 {
                        Style::default().fg(C_WARN)
                    } else {
                        Style::default().fg(C_ACCENT)
                    }),
            )
            .highlight_style(Style::default().fg(C_SELECTED).add_modifier(Modifier::BOLD))
            .highlight_symbol("▶ ");

        frame.render_stateful_widget(list, main, &mut self.fn_picker.list_state);
    }

    fn render_rename_popup(&self, frame: &mut Frame) {
        let area = centered_rect(50, 7, frame.area());
        frame.render_widget(Clear, area);

        let name = self
            .rename_idx
            .and_then(|i| self.fn_picker.entries.get(i))
            .map(|e| e.sig.name.as_str())
            .unwrap_or("?");

        let text = format!("Rename '{}'\n\n> {}_", name, self.rename_buf);
        let p = Paragraph::new(text)
            .block(
                Block::default()
                    .title(" function  [Enter=confirm  Esc=cancel] ")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(C_SELECTED)),
            )
            .wrap(Wrap { trim: false });
        frame.render_widget(p, area);
    }

    fn render_output_name(&mut self, frame: &mut Frame) {
        let (main, status_area) = Self::base_layout(frame);
        self.render_status(frame, status_area);

        let text = format!(
            "Output file stem (without extension):\n\n> {}_\n\nWill create:\n  {}.h\n  {}.c",
            self.output_stem, self.output_stem, self.output_stem,
        );
        let p = Paragraph::new(text)
            .block(
                Block::default()
                    .title(" 💾  Output name  [Enter=next  Esc=back] ")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(C_ACCENT)),
            )
            .wrap(Wrap { trim: false });
        frame.render_widget(p, main);
    }

    fn render_confirm(&mut self, frame: &mut Frame) {
        let (main, status_area) = Self::base_layout(frame);
        self.render_status(frame, status_area);

        let selected: Vec<&FunctionEntry> = self
            .fn_picker
            .entries
            .iter()
            .filter(|e| e.selected)
            .collect();

        let mut lines: Vec<Line> = vec![
            Line::from(Span::styled(
                format!("Output: {}.c / {}.h", self.output_stem, self.output_stem),
                Style::default().fg(C_ACCENT).add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "Functions to export:",
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::UNDERLINED),
            )),
        ];

        for e in &selected {
            let alias = if e.export_name != e.sig.name {
                format!(" (exported as '{}')", e.export_name)
            } else {
                String::new()
            };
            lines.push(Line::from(Span::styled(
                format!("  • {}{}", format_sig_short(&e.sig), alias),
                Style::default().fg(Color::White),
            )));
        }

        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "Press Enter to generate, Esc to go back.",
            Style::default().fg(C_OK).add_modifier(Modifier::BOLD),
        )));

        let p = Paragraph::new(Text::from(lines))
            .block(
                Block::default()
                    .title(" ✅  Confirm  [Enter=generate  Esc=back] ")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(C_OK)),
            )
            .wrap(Wrap { trim: false });
        frame.render_widget(p, main);
    }

    fn render_done(&self, frame: &mut Frame) {
        let p = Paragraph::new(format!(
            "✅  Done!\n\n{}\n\nPress any key or Ctrl+q to exit.",
            self.status
        ))
        .block(
            Block::default()
                .title(" b_extractor ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(C_OK)),
        )
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: false });
        frame.render_widget(p, frame.area());
    }
}

pub fn format_sig_short(sig: &ValidatedFunctionSignature) -> String {
    let params = sig
        .original_params
        .iter()
        .map(|p| format!("{}: {}", p.name, p.ty))
        .collect::<Vec<_>>()
        .join(", ");
    match &sig.return_type {
        ValidatedReturn::StringReturn(_) => {
            format!("fun {}({})", sig.name, params)
        }
        ValidatedReturn::Numeric(ret) => {
            format!("fun {}({}) -> {}", sig.name, params, ret)
        }
    }
}

fn centered_rect(percent_x: u16, height: u16, r: ratatui::layout::Rect) -> ratatui::layout::Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length((r.height.saturating_sub(height)) / 2),
            Constraint::Length(height),
            Constraint::Min(0),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

pub fn run(files: Vec<PathBuf>, output_dir: PathBuf) -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new(files, output_dir);

    loop {
        terminal.draw(|f| app.render(f))?;

        if let Event::Key(key) = event::read()? {
            app.handle_key(key.code, key.modifiers);
        }

        if app.is_done() {
            break;
        }
    }

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if !app.status.is_empty() {
        println!("{}", app.status);
    }

    Ok(())
}
