use std::io;
use std::sync::Arc;
use tokio::sync::Mutex;

use reqwest::Client;
use serde_json::Value;
use ratatui::{
    crossterm::event::{self, KeyCode, KeyEventKind, Event, KeyEvent},
    buffer::Buffer,
    layout::{Rect, Constraint, Layout},
    style::{Style, Stylize},
    symbols::border,
    text::Line,
    widgets::{Block, Widget, Table, Row, Cell, Paragraph},
    Terminal, Frame,
};
use ratatui::backend::CrosstermBackend;
mod app;

#[tokio::main]
async fn main() -> io::Result<()> {
    // Initialize terminal
    let stdout = io::stdout();
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;
    crossterm::terminal::enable_raw_mode()?;

    // Create app state
    let app = Arc::new(Mutex::new(app::App::default()));
    
    // Run app
    let result = app::App::run(app, &mut terminal).await;
    
    // Cleanup
    crossterm::terminal::disable_raw_mode()?;
    terminal.clear()?;
    terminal.show_cursor()?;
    
    result
}
