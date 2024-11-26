use std::sync::Arc;
use std::{fmt::format, io};
use tokio::sync::Mutex;

use reqwest::Client;
use serde_json::Value;

use ratatui::crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::{
    backend::CrosstermBackend,
    buffer::Buffer,
    layout::{Constraint, Layout, Rect},
    prelude::Alignment,
    style::{palette::tailwind, Color, Style, Stylize},
    symbols::border,
    text::{Line, Span},
    widgets::{Block, Cell, Clear, Gauge, Paragraph, Row, Table, Widget},
    Frame, Terminal,
};

const GAUGE2_COLOR: Color = tailwind::GREEN.c800;

#[derive(Debug)]
pub struct App {
    pub query: String,
    pub input_mode: InputMode,
    pub slot_info: Option<i64>,
    pub transaction_info: Option<i64>,
    pub supply_info: Option<Value>,
    pub json_response: Option<Value>,
    pub exit: bool,
    pub show_popup: bool,
    client: Client,
}

#[derive(Debug)]
pub enum InputMode {
    Normal,
    Editing,
}

impl Default for App {
    fn default() -> Self {
        Self {
            query: String::new(),
            input_mode: InputMode::Normal,
            slot_info: None,
            transaction_info: None,
            supply_info: None,
            json_response: None,
            exit: false,
            show_popup: false,
            client: Client::new(),
        }
    }
}

impl App {
    //Fetch Intial Blockchain data
    pub async fn fetch_initial_blockchain_data(
        &mut self,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Fetch slot Info
        let slot_payload = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "getSlot",
        });

        let slot_response = self
            .client
            .post("https://rpc.devnet.soo.network/rpc")
            .header("Content-Type", "application/json")
            .json(&slot_payload)
            .send()
            .await?;

        if slot_response.status().is_success() {
            let slot_json: Value = slot_response.json().await?;
            self.slot_info = slot_json.get("result").and_then(|r| r.as_i64());
        }

        // Fetch Supply Info
        let supply_payload = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "getSupply"
        });

        let supply_response = self
            .client
            .post("https://rpc.devnet.soo.network/rpc")
            .header("Content-Type", "application/json")
            .json(&supply_payload)
            .send()
            .await?;

        if supply_response.status().is_success() {
            let supply_json: Value = supply_response.json().await?;
            self.supply_info = supply_json.get("result").cloned();
        }

        // to get transaction count
        let transcation_payload = serde_json::json!({
            "jsonrpc":"2.0",
            "id":1,
            "method":"getTransactionCount"
        });

        let transaction_response = self
            .client
            .post("https://rpc.devnet.soo.network/rpc")
            .header("Content-Type", "application/json")
            .json(&transcation_payload)
            .send()
            .await?;

        if transaction_response.status().is_success() {
            let transaction_json: Value = transaction_response.json().await?;
            self.transaction_info = transaction_json.get("result").and_then(|r| r.as_i64());
        }

        Ok(())
    }

    pub async fn run(
        app: Arc<Mutex<App>>,
        terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    ) -> io::Result<()> {
        // Fetch initial data
        {
            let mut app = app.lock().await;
            app.fetch_initial_blockchain_data()
                .await
                .unwrap_or_else(|e| eprintln!("Error fetching initial data: {}", e));
        }

        loop {
            {
                let app = app.lock().await;
                if app.exit {
                    break;
                }
                terminal.draw(|frame| app.draw(frame))?;
            }

            if let Ok(should_break) = App::handle_events(Arc::clone(&app)).await {
                if should_break {
                    break;
                }
            }
        }
        Ok(())
    }

    fn draw(&self, frame: &mut Frame) {
        let chunks = Layout::vertical([
            Constraint::Length(3), // Input field
            Constraint::Min(1),    // Results area
        ])
        .split(frame.area());

        // Create a layout for bottom instructions
        let bottom_layout =
            Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(chunks[0]);

        // Render input field
        let input_title = match self.input_mode {
            InputMode::Normal => " SOONSCAN ".red().bold(),
            InputMode::Editing => " SOONSCAN ".red().bold(),
        };

        let input = Paragraph::new(self.query.as_str())
            .style(match self.input_mode {
                InputMode::Normal => Style::default(),
                InputMode::Editing => Style::default().yellow(),
            })
            .block(Block::bordered().title(input_title));

        frame.render_widget(input, chunks[0]);

        // Bottom right instructions
        let instructions = Paragraph::new(match self.input_mode {
            InputMode::Normal => " Press 'e' to edit ".blue().bold(),
            InputMode::Editing => " Enter: Submit, Esc: Cancel ".blue().bold(),
        })
        .alignment(Alignment::Right);

        frame.render_widget(instructions, bottom_layout[1]);

        // Render results area
        frame.render_widget(self, chunks[1]);
        // Render popup if active
        if self.show_popup {
            let popup_area = centered_rect(60, 40, frame.area());
            let popup_block = Block::bordered()
                .title("SoonScan - Help & Guide")
                .border_style(Style::default().red());

            let help_text = vec![
                Line::from(vec![" Retrieve transaction information".blue()]),
                Line::from(vec![
                    " View account balances, transaction status, and more".blue()
                ]),
                Line::from(vec!["".into()]),
                Line::from(vec![" ⌨️ Keystrokes:".blue().bold()]),
                Line::from(vec![" e      : Enter edit mode for query input".blue()]),
                Line::from(vec![" Enter  : Submit query (account/transaction)".blue()]),
                Line::from(vec![" Esc    : Cancel editing/close popup".blue()]),
                Line::from(vec![" Ctrl+V : Paste content from clipboard".blue()]),
                Line::from(vec![" ?      : Toggle this help popup".blue()]),
                Line::from(vec![" q      : Quit application".blue()]),
            ];

            let popup_text = Paragraph::new(help_text)
                .block(popup_block)
                .wrap(ratatui::widgets::Wrap { trim: true });

            frame.render_widget(Clear, popup_area);
            frame.render_widget(popup_text, popup_area);
        }
    }

    async fn handle_events(app: Arc<Mutex<App>>) -> io::Result<bool> {
        if let Event::Key(key_event) = event::read()? {
            if key_event.kind == KeyEventKind::Press {
                match key_event.code {
                    KeyCode::Char('q') => {
                        let mut app = app.lock().await;
                        app.exit = true;
                        return Ok(true);
                    }
                    KeyCode::Char('e') => {
                        let mut app = app.lock().await;
                        if matches!(app.input_mode, InputMode::Normal) {
                            app.input_mode = InputMode::Editing;
                        }
                    }
                    KeyCode::Esc => {
                        let mut app = app.lock().await;
                        app.input_mode = InputMode::Normal;
                    }
                    KeyCode::Enter => {
                        let mut app = app.lock().await;
                        if matches!(app.input_mode, InputMode::Editing) {
                            app.input_mode = InputMode::Normal;
                            if !app.query.is_empty() {
                                app.fetch_data()
                                    .await
                                    .unwrap_or_else(|e| eprintln!("Error: {}", e));
                            }
                        }
                    }
                    KeyCode::Char('?') => {
                        let mut app = app.lock().await;
                        app.show_popup = !app.show_popup;
                    }
                    KeyCode::Char(c) => {
                        let mut app = app.lock().await;
                        if matches!(app.input_mode, InputMode::Editing) {
                            app.query.push(c);
                        }
                    }
                    KeyCode::Backspace => {
                        let mut app = app.lock().await;
                        if matches!(app.input_mode, InputMode::Editing) {
                            app.query.pop();
                        }
                    }
                    _ => {}
                }
            }
        }
        Ok(false)
    }

    async fn fetch_data(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let method = if self.query.len() == 44 {
            "getAccountInfo"
        } else {
            "getTransaction"
        };

        let payload = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": method,
            "params": [self.query, {"encoding": "base58"}]
        });

        let response = self
            .client
            .post("https://rpc.devnet.soo.network/rpc")
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await?;

        if response.status().is_success() {
            self.json_response = Some(response.json().await?);
        } else {
            eprintln!("Request failed with status: {}", response.status());
            self.json_response = None;
        }

        Ok(())
    }

    fn format_timestamp(&self, timestamp: i64) -> String {
        use chrono::{DateTime, TimeZone, Utc};
        let dt: DateTime<Utc> = Utc.timestamp_opt(timestamp, 0).unwrap();
        dt.format("%Y-%m-%d %H:%M:%S UTC").to_string()
    }

    fn format_longnumber(&self, number: i64) -> String {
        use std::fmt::Write;

        let mut formatted = String::new();
        let number_str = number.abs().to_string();
        let len = number_str.len();

        for (i, c) in number_str.chars().enumerate() {
            if i > 0 && (len - i) % 3 == 0 {
                write!(&mut formatted, ",").unwrap();
            }
            write!(&mut formatted, "{}", c).unwrap();
        }

        if number < 0 {
            format!("-{}", formatted)
        } else {
            formatted
        }
    }
}

impl Widget for &App {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let instruction = Line::from(vec![
            " Quit ".into(),
            "<Q> ".blue().bold(),
            " | ".into(),
            " Help ".into(),
            " ? ".blue().bold(),
        ]);
        let block = Block::bordered()
            .title_bottom(instruction.centered())
            .border_set(border::THICK);

        let mut rows = vec![];

        // Show blockchain data when no query is done!
        if self.query.is_empty() {
            if let Some(slot_info) = self.slot_info {
                rows.push(Row::new(vec![
                    Cell::from("Network").bold(),
                    Cell::from("SoonScan Devnet").bold(),
                ]));

                rows.push(Row::new(vec![
                    Cell::from("Slot:").bold(),
                    Cell::from(self.format_longnumber(slot_info).yellow()),
                ]));
            }

            if let Some(supply_info) = &self.supply_info {
                if let Some(value) = supply_info.get("value") {
                    let total_supply = value.get("total").and_then(|t| t.as_i64()).unwrap_or(0);
                    let circulating_supply = value
                        .get("circulating")
                        .and_then(|c| c.as_i64())
                        .unwrap_or(0);

                    // Calculate the percentage of circulating supply
                    let circulating_percentage = if total_supply > 0 {
                        (circulating_supply as f64 / total_supply as f64) * 100.0
                    } else {
                        0.0
                    };

                    rows.extend(vec![
                        Row::new(vec![
                            Cell::from("Circulating Supply:").bold(),
                            Cell::from(
                                format!("{} / {}", self.format_longnumber(circulating_supply), self.format_longnumber(total_supply))
                                    .green(),
                            ),
                        ]),
                        Row::new(vec![
                            Cell::from("Circulating Percentage:").bold(),
                            Cell::from(
                                format!("{:.1}% is circulating", circulating_percentage).green(),
                            ),
                        ]),
                    ]);
                }
            }

            if let Some(transaction_info) = self.transaction_info {
                rows.push(Row::new(vec![
                    Cell::from("Transaction count:").bold(),
                    Cell::from(self.format_longnumber(transaction_info).yellow()),
                ]));
            }
        } else if let Some(json_response) = &self.json_response {
            if let Some(result) = json_response.get("result").and_then(|r| r.as_object()) {
                // Account Info Response
                if let Some(value) = result.get("value").and_then(|v| v.as_object()) {
                    rows.extend(vec![
                        Row::new(vec![
                            Cell::from("Type:").bold(),
                            Cell::from("Account Info".blue()),
                        ]),
                        Row::new(vec![
                            Cell::from("Balance:").bold(),
                            Cell::from(
                                format!(
                                    "◎ {:.9}",
                                    value.get("lamports").and_then(|l| l.as_u64()).unwrap_or(0)
                                        as f64
                                        / 1_000_000_000.0
                                )
                                .yellow(),
                            ),
                        ]),
                        Row::new(vec![
                            Cell::from("Owner:").bold(),
                            Cell::from(
                                value
                                    .get("owner")
                                    .and_then(|o| o.as_str())
                                    .unwrap_or("N/A")
                                    .yellow(),
                            ),
                        ]),
                        Row::new(vec![
                            Cell::from("Executable:").bold(),
                            Cell::from(
                                if value
                                    .get("executable")
                                    .and_then(|e| e.as_bool())
                                    .unwrap_or(false)
                                {
                                    "Yes".green()
                                } else {
                                    "No".red()
                                },
                            ),
                        ]),
                        Row::new(vec![
                            Cell::from("Space:").bold(),
                            Cell::from(
                                value
                                    .get("space")
                                    .and_then(|s| s.as_u64())
                                    .unwrap_or(0)
                                    .to_string()
                                    .yellow(),
                            ),
                        ]),
                    ]);
                }
                // Transaction Response
                else if let Some(meta) = result.get("meta").and_then(|m| m.as_object()) {
                    rows.extend(vec![
                        Row::new(vec![
                            Cell::from("Type:").bold(),
                            Cell::from("Transaction".blue()),
                        ]),
                        Row::new(vec![
                            Cell::from("Block Time:").bold(),
                            Cell::from(
                                result
                                    .get("blockTime")
                                    .and_then(|t| t.as_i64())
                                    .map(|t| self.format_timestamp(t))
                                    .unwrap_or_else(|| "N/A".to_string())
                                    .yellow(),
                            ),
                        ]),
                        Row::new(vec![
                            Cell::from("Status:").bold(),
                            Cell::from(if meta.get("status").and_then(|s| s.get("Ok")).is_some() {
                                "Success".green()
                            } else {
                                "Failed".red()
                            }),
                        ]),
                        Row::new(vec![
                            Cell::from("Fee:").bold(),
                            Cell::from(
                                format!(
                                    "◎ {:.9}",
                                    meta.get("fee").and_then(|f| f.as_u64()).unwrap_or(0) as f64
                                        / 1_000_000_000.0
                                )
                                .yellow(),
                            ),
                        ]),
                        Row::new(vec![
                            Cell::from("Compute Units:").bold(),
                            Cell::from(
                                meta.get("computeUnitsConsumed")
                                    .and_then(|c| c.as_u64())
                                    .map(|c| c.to_string())
                                    .unwrap_or_else(|| "N/A".to_string())
                                    .yellow(),
                            ),
                        ]),
                    ]);

                    // Add balance changes
                    if let (Some(pre), Some(post)) = (
                        meta.get("preBalances").and_then(|b| b.as_array()),
                        meta.get("postBalances").and_then(|b| b.as_array()),
                    ) {
                        for (i, (pre_bal, post_bal)) in pre.iter().zip(post.iter()).enumerate() {
                            let pre_value = pre_bal.as_u64().unwrap_or(0) as f64 / 1_000_000_000.0;
                            let post_value =
                                post_bal.as_u64().unwrap_or(0) as f64 / 1_000_000_000.0;
                            let change = post_value - pre_value;

                            rows.push(Row::new(vec![
                                Cell::from(format!("Balance Change {}:", i)).bold(),
                                Cell::from(
                                    format!(
                                        "◎ {:.9} → ◎ {:.9} (Δ {:.9})",
                                        pre_value, post_value, change
                                    )
                                    .yellow(),
                                ),
                            ]));
                        }
                    }
                }
            }
        } else if !self.query.is_empty() {
            rows.push(Row::new(vec![
                Cell::from("Status:").bold(),
                Cell::from("Loading...".yellow()),
            ]));
        }

        let widths = [Constraint::Length(20), Constraint::Percentage(80)];

        let table = Table::new(rows, &widths).block(block).column_spacing(2);

        table.render(area, buf);
    }
}

// Helper to create centered rectangle for popup
fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let popup_width = area.width * percent_x / 100;
    let popup_height = area.height * percent_y / 100;
    let popup_x = (area.width - popup_width) / 2;
    let popup_y = (area.height - popup_height) / 2;

    Rect::new(popup_x, popup_y, popup_width, popup_height)
}
