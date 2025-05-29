use std::{
    io::{self},
    sync::{Arc, Mutex},
};

use ansi_to_tui::IntoText;
use crossterm::event;
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Layout, Rect},
    style::Stylize,
    widgets::{Block, Widget},
    DefaultTerminal, Frame,
};
use tracing_subscriber::{filter::ParseError, fmt::MakeWriter, EnvFilter};
use tui_textarea::{Input, Key, TextArea};

/// A list of preset filters to make it easier to explore the filter syntax.
///
/// The UI allows you to select a preset filter with the up/down arrow keys.
const PRESET_FILTERS: &[&str] = &[
    "trace",
    "debug",
    "info",
    "warn",
    "error",
    "[with_fields]",
    "[with_fields{foo}]",
    "[with_fields{bar}]",
    "[with_fields{foo=42}]",
    "[with_fields{bar=bar}]",
    "[with_fields{foo=99}]",
    "[with_fields{bar=nope}]",
    "[with_fields{nonexistent}]",
    "other_crate=info",
    "other_crate=debug",
    "trace,other_crate=warn",
    "warn,other_crate=info",
];

fn main() -> io::Result<()> {
    let terminal = ratatui::init();
    let result = App::new().run(terminal);
    ratatui::restore();
    result
}

struct App {
    filter: TextArea<'static>,
    preset_index: usize,
    exit: bool,
    log_widget: Result<LogWidget, ParseError>,
}

impl App {
    /// Creates a new instance of the application, ready to run
    fn new() -> Self {
        let mut filter = TextArea::new(vec![PRESET_FILTERS[0].to_string()]);
        let title = "Env Filter Explorer. <Esc> to quit, <Up>/<Down> to select preset";
        filter.set_block(Block::bordered().title(title));
        Self {
            filter,
            preset_index: 0,
            exit: false,
            log_widget: Ok(LogWidget::default()),
        }
    }

    /// The application's main loop until the user exits.
    fn run(mut self, mut terminal: DefaultTerminal) -> io::Result<()> {
        while !self.exit {
            self.log_widget = self.evaluate_filter();
            terminal.draw(|frame| self.render(frame))?;
            self.handle_event()?;
        }
        Ok(())
    }

    /// Render the application with a filter input area and a log output area.
    fn render(&self, frame: &mut Frame) {
        let layout = Layout::vertical([Constraint::Length(3), Constraint::Fill(1)]);
        let [filter_area, main_area] = layout.areas(frame.area());
        frame.render_widget(&self.filter, filter_area);
        match &self.log_widget {
            Ok(log_widget) => frame.render_widget(log_widget, main_area),
            Err(error) => frame.render_widget(error.to_string().red(), main_area),
        }
    }

    /// Handles a single terminal event (e.g. mouse, keyboard, resize).
    fn handle_event(&mut self) -> io::Result<()> {
        let event = event::read()?;
        let input = Input::from(event);
        match input.key {
            Key::Enter => return Ok(()), // ignore new lines
            Key::Esc => self.exit = true,
            Key::Up => self.select_previous_preset(),
            Key::Down => self.select_next_preset(),
            _ => self.add_input(input),
        }
        Ok(())
    }

    /// Selects the previous preset filter in the list.
    fn select_previous_preset(&mut self) {
        self.select_preset(self.preset_index.saturating_sub(1));
    }

    /// Selects the next preset filter in the list.
    fn select_next_preset(&mut self) {
        self.select_preset((self.preset_index + 1).min(PRESET_FILTERS.len() - 1));
    }

    /// Selects a preset filter by index and updates the filter text area.
    fn select_preset(&mut self, index: usize) {
        self.preset_index = index;
        self.filter.select_all();
        self.filter.delete_line_by_head();
        self.filter.insert_str(PRESET_FILTERS[self.preset_index]);
    }

    /// Handles normal keyboard input by adding it to the filter text area.
    fn add_input(&mut self, input: Input) {
        self.filter.input(input);
    }

    /// Evaluates the current filter and returns a log widget with the filtered logs or an error.
    fn evaluate_filter(&mut self) -> Result<LogWidget, ParseError> {
        let filter = self.filter.lines()[0].to_string();
        let env_filter = EnvFilter::builder().parse(filter)?;
        let log_widget = LogWidget::default();
        let subscriber = tracing_subscriber::fmt()
            .with_env_filter(env_filter)
            .with_writer(log_widget.clone())
            .finish();
        tracing::subscriber::with_default(subscriber, || {
            simulate_logging();
            other_crate_span();
        });
        Ok(log_widget)
    }
}

/// A writer that collects logs into a buffer and can be displayed as a widget.
#[derive(Clone, Default, Debug)]
struct LogWidget {
    buffer: Arc<Mutex<Vec<u8>>>,
}

impl io::Write for LogWidget {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.buffer.lock().unwrap().write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.buffer.lock().unwrap().flush()
    }
}

impl<'a> MakeWriter<'a> for LogWidget {
    type Writer = Self;

    fn make_writer(&'a self) -> Self::Writer {
        self.clone()
    }
}

impl Widget for &LogWidget {
    /// Displays the logs that have been collected in the buffer.
    ///
    /// If the buffer is empty, it displays "No matching logs".
    fn render(self, area: Rect, buf: &mut Buffer) {
        let buffer = self.buffer.lock().unwrap();
        let string = String::from_utf8_lossy(&buffer).to_string();
        if string.is_empty() {
            "No matching logs".render(area, buf);
            return;
        }
        string
            .into_text() // convert a string with ANSI escape codes into ratatui Text
            .unwrap_or_else(|err| format!("Error parsing output: {err}").into())
            .render(area, buf);
    }
}

#[tracing::instrument]
fn simulate_logging() {
    tracing::info!("This is an info message");
    tracing::error!("This is an error message");
    tracing::warn!("This is a warning message");
    tracing::debug!("This is a debug message");
    tracing::trace!("This is a trace message");

    with_fields(42, "bar");
    with_fields(99, "nope");

    trace_span();
    debug_span();
    info_span();
    warn_span();
    error_span();
}

#[tracing::instrument]
fn with_fields(foo: u32, bar: &'static str) {
    tracing::info!(foo, bar, "This is an info message with fields");
}

#[tracing::instrument(level = "trace")]
fn trace_span() {
    tracing::error!("Error message inside a span with trace level");
    tracing::info!("Info message inside a span with trace level");
    tracing::trace!("Trace message inside a span with trace level");
}

#[tracing::instrument]
fn debug_span() {
    tracing::error!("Error message inside a span with debug level");
    tracing::info!("Info message inside a span with debug level");
    tracing::debug!("Debug message inside a span with debug level");
}

#[tracing::instrument]
fn info_span() {
    tracing::error!("Error message inside a span with info level");
    tracing::info!("Info message inside a span with info level");
    tracing::debug!("Debug message inside a span with info level");
}

#[tracing::instrument]
fn warn_span() {
    tracing::error!("Error message inside a span with warn level");
    tracing::info!("Info message inside a span with warn level");
    tracing::debug!("Debug message inside a span with warn level");
}

#[tracing::instrument]
fn error_span() {
    tracing::error!("Error message inside a span with error level");
    tracing::info!("Info message inside a span with error level");
    tracing::debug!("Debug message inside a span with error level");
}

#[tracing::instrument(target = "other_crate")]
fn other_crate_span() {
    tracing::error!(target: "other_crate", "An error message from another crate");
    tracing::warn!(target: "other_crate", "A warning message from another crate");
    tracing::info!(target: "other_crate", "An info message from another crate");
    tracing::debug!(target: "other_crate", "A debug message from another crate");
    tracing::trace!(target: "other_crate", "A trace message from another crate");
}
