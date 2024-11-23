use std::io;

use reqwest::Client;
use serde_json::Value;

use ratatui::{
    crossterm::event::{self, KeyCode, KeyEventKind, Event, KeyEvent},
    buffer::Buffer,
    layout::{Rect, Constraint},
    style::Stylize,
    symbols::border,
    text::Line,
    widgets::{Block, Widget, Table, Row, Cell},
    DefaultTerminal, Frame,
};

// This will store the state of the application
#[derive(Debug, Default)]
pub struct App {
    pub query: String,
    pub json_response: Option<Value>, // Stores the response from the blockchain query
    pub exit: bool,
}

impl App {
    pub async fn run(&mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        self.fetch_transaction().await.expect("Failed to fetch transaction");

        while !self.exit {
            terminal.draw(|frame| self.draw(frame))?;
            self.handle_events()?;
        }
        Ok(())
    }

    fn draw(&self, frame: &mut Frame) {
        frame.render_widget(self, frame.area());
    }

    fn handle_events(&mut self) -> io::Result<()> {
        match event::read()? {
            Event::Key(key_events) if key_events.kind == KeyEventKind::Press => {
                self.handle_key_event(key_events);
            }
            _ => {}
        };
        Ok(())
    }

    fn handle_key_event(&mut self, key_events: KeyEvent) {
        match key_events.code {
            KeyCode::Char('q') => self.exit(),
            _ => {}
        }
    }

    fn exit(&mut self) {
        self.exit = true;
    }

    async fn fetch_transaction(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // JSON payload for the blockchain query
        let payload = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "getTransaction",
            "params": [
                self.query,
                "json"
            ]
        });

        // Create HTTP client
        let client = Client::new();

        // Perform the POST request
        let response = client
            .post("https://rpc.devnet.soo.network/rpc")
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await?;

        if response.status().is_success() {
            // Parse and store the JSON response
            self.json_response = Some(response.json().await?);
        } else {
            eprintln!("Request failed with status: {}", response.status());
            self.json_response = None;
        }

        Ok(())
    }
}

impl Widget for &App {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let title = Line::from(" SOONSCAN ".bold().red());
        let instruction = Line::from(vec![
            " Quit ".into(),
            "<Q> ".blue().bold(),
        ]);

        let block = Block::bordered()
            .title(title.centered())
            .title_bottom(instruction.centered())
            .border_set(border::THICK);

        // Header displaying the Transaction Hash
        let header = vec![
            Row::new(vec![
                Cell::from("Transaction Hash:").bold(),
                Cell::from(self.query.clone().yellow()),
            ]),
        ];

        // Parse the JSON response into rows for displaying key-value pairs
        let rows = if let Some(json_response) = &self.json_response {
            let result = json_response.get("result").and_then(|r| r.as_object());
            if let Some(result_obj) = result {
                let mut rows = vec![];

                // Extract relevant fields from the result
                if let Some(block_time) = result_obj.get("blockTime") {
                    rows.push(Row::new(vec![
                        Cell::from("Block Time:").bold(),
                        Cell::from(block_time.to_string().yellow()),
                    ]));
                }
                if let Some(meta) = result_obj.get("meta").and_then(|m| m.as_object()) {
                     if let Some(fee) = meta.get("fee") {
                        let fee_sol = fee.as_u64().unwrap_or(0) as f64 / 1_000_000_000.0; // Convert lamports to SOL
                        rows.push(Row::new(vec![
                            Cell::from("Fee (SOL):").bold(),
                            Cell::from(format!("◎ {:.8}", fee_sol).yellow()),
                        ]));
                    }
                    if let Some(status) = meta.get("status") {
                        rows.push(Row::new(vec![
                            Cell::from("Status:").bold(),
                            Cell::from(status.to_string().yellow()),
                        ]));
                    }
                    if let Some(compute_units) = meta.get("computeUnitsConsumed") {
                        rows.push(Row::new(vec![
                            Cell::from("Compute Units Consumed:").bold(),
                            Cell::from(compute_units.to_string().yellow()),
                        ]));
                    }
                }

                // Extract signatures
                if let Some(signatures) = result_obj.get("signatures") {
                    if let Some(signature_arr) = signatures.as_array() {
                        for signature in signature_arr {
                            rows.push(Row::new(vec![
                                Cell::from("Signature:").bold(),
                                Cell::from(signature.to_string().yellow()),
                            ]));
                        }
                    }
                }

                rows
            } else {
                vec![Row::new(vec![Cell::from("Invalid JSON response.")])]
            }
        } else {
            vec![Row::new(vec![Cell::from("No response available.".bold().red())])]
        };

        // Define column widths
        let widths = [
            Constraint::Length(20), // Key column width
            Constraint::Percentage(80), // Value column takes remaining space
        ];

        // Create the table
        let table = Table::new(header.into_iter().chain(rows), &widths)
            .block(block)
            .column_spacing(2);

        table.render(area, buf);
    }
}


