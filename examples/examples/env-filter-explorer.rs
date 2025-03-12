use std::{
    fmt,
    io::{self},
    sync::{Arc, Mutex},
};

use ansi_to_tui::IntoText;
use crossterm::event;
use ratatui::{
    layout::{Constraint, Layout},
    text::Text,
    widgets::Block,
    DefaultTerminal, Frame,
};
use tracing_subscriber::fmt::MakeWriter;
use tui_textarea::{Input, Key, TextArea};

fn main() -> io::Result<()> {
    let terminal = ratatui::init();
    let result = run(terminal);
    ratatui::restore();
    result
}

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

fn run(mut terminal: DefaultTerminal) -> io::Result<()> {
    let mut textarea = TextArea::new(vec!["trace".to_string()]);
    let title = "Env Filter Explorer. <Esc> to quit, <Up>/<Down> to select preset";
    textarea.set_block(Block::bordered().title(title));
    let mut preset_index: usize = 0;
    loop {
        terminal.draw(|frame| render(frame, &textarea))?;
        match event::read()?.into() {
            Input {
                key: Key::Enter, ..
            } => {}
            Input { key: Key::Esc, .. } => break Ok(()),
            Input { key: Key::Up, .. } => reset_preset(&mut textarea, &mut preset_index, -1),
            Input { key: Key::Down, .. } => reset_preset(&mut textarea, &mut preset_index, 1),
            input => {
                textarea.input(input);
            }
        }
    }
}

fn reset_preset(textarea: &mut TextArea<'_>, preset_index: &mut usize, offset: isize) {
    *preset_index = preset_index
        .saturating_add_signed(offset)
        .min(PRESET_FILTERS.len() - 1);
    let input = PRESET_FILTERS[*preset_index];
    textarea.select_all();
    textarea.delete_line_by_head();
    textarea.insert_str(input);
}

fn render(frame: &mut Frame, textarea: &TextArea) {
    let layout = Layout::vertical([Constraint::Length(3), Constraint::Fill(1)]);
    let [top, body] = layout.areas(frame.area());
    frame.render_widget(textarea, top);
    let filter = textarea.lines()[0].to_string();

    let Ok(env_filter) = tracing_subscriber::EnvFilter::builder().parse(filter) else {
        let text = Text::from("Error parsing filter");
        frame.render_widget(text, body);
        return;
    };
    let writer = StringWriter::default();
    let collector = tracing_subscriber::fmt()
        .with_env_filter(env_filter)
        .with_writer(writer.clone())
        .finish();

    tracing::collect::with_default(collector, simulate_logging);
    let output = writer.to_string();
    let text = output
        .into_text()
        .unwrap_or(Text::from("Error parsing output"));
    frame.render_widget(text, body);
}

#[tracing::instrument]
fn simulate_logging() {
    tracing::info!("This is an info message");
    tracing::error!("This is an error message");
    tracing::warn!("This is a warning message");
    tracing::debug!("This is a debug message");
    tracing::trace!("This is a trace message");

    other_crate();
    trace_span();
    with_fields(42, "bar");
    with_fields(99, "nope");
}

#[tracing::instrument(target = "other_crate")]
fn other_crate() {
    tracing::error!(
        target: "other_crate",
        "This is an error message from another crate"
    );
    tracing::warn!(
        target: "other_crate",
        "This is a warning message from another crate"
    );
    tracing::info!(
        target: "other_crate",
        "This is an info message from another crate"
    );
    tracing::debug!(
        target: "other_crate",
        "This is a debug message from another crate"
    );
    tracing::trace!(
        target: "other_crate",
        "This is a trace message from another crate"
    );
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

#[derive(Clone, Default, Debug)]
struct StringWriter {
    buffer: Arc<Mutex<Vec<u8>>>,
}

impl fmt::Display for StringWriter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let buffer = self.buffer.lock().unwrap();
        let string = String::from_utf8_lossy(&buffer);
        write!(f, "{}", string)
    }
}

impl io::Write for StringWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.buffer.lock().unwrap().write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.buffer.lock().unwrap().flush()
    }
}

impl<'a> MakeWriter<'a> for StringWriter {
    type Writer = Self;

    fn make_writer(&'a self) -> Self::Writer {
        self.clone()
    }
}
