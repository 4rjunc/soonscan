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

#[derive(Debug, Default)]
pub struct App {
    pub query: String,
    pub input: String,
    pub json_response: Option<Value>,
    pub exit: bool,
}

impl App {
    pub async fn run(&mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        self.fetch_data().await.expect("Failed to fetch data");

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

        let client = Client::new();
        let response = client
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

        let mut rows = vec![
            Row::new(vec![
                Cell::from("Query:").bold(),
                Cell::from(self.query.clone().yellow()),
            ]),
        ];

        if let Some(json_response) = &self.json_response {
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
                            Cell::from(format!("◎ {:.9}", value.get("lamports").and_then(|l| l.as_u64()).unwrap_or(0) as f64 / 1_000_000_000.0).yellow()),
                        ]),
                        Row::new(vec![
                            Cell::from("Owner:").bold(),
                            Cell::from(value.get("owner").and_then(|o| o.as_str()).unwrap_or("N/A").yellow()),
                        ]),
                        Row::new(vec![
                            Cell::from("Executable:").bold(),
                            Cell::from(if value.get("executable").and_then(|e| e.as_bool()).unwrap_or(false) {
                                "Yes".green()
                            } else {
                                "No".red()
                            }),
                        ]),
                        Row::new(vec![
                            Cell::from("Space:").bold(),
                            Cell::from(value.get("space").and_then(|s| s.as_u64()).unwrap_or(0).to_string().yellow()),
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
                            Cell::from(result.get("blockTime")
                                .and_then(|t| t.as_i64())
                                .map(|t| self.format_timestamp(t))
                                .unwrap_or_else(|| "N/A".to_string())
                                .yellow()),
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
                            Cell::from(format!("◎ {:.9}", meta.get("fee").and_then(|f| f.as_u64()).unwrap_or(0) as f64 / 1_000_000_000.0).yellow()),
                        ]),
                        Row::new(vec![
                            Cell::from("Compute Units:").bold(),
                            Cell::from(meta.get("computeUnitsConsumed")
                                .and_then(|c| c.as_u64())
                                .map(|c| c.to_string())
                                .unwrap_or_else(|| "N/A".to_string())
                                .yellow()),
                        ]),
                    ]);

                    // Add balance changes
                    if let (Some(pre), Some(post)) = (
                        meta.get("preBalances").and_then(|b| b.as_array()),
                        meta.get("postBalances").and_then(|b| b.as_array()),
                    ) {
                        for (i, (pre_bal, post_bal)) in pre.iter().zip(post.iter()).enumerate() {
                            let pre_value = pre_bal.as_u64().unwrap_or(0) as f64 / 1_000_000_000.0;
                            let post_value = post_bal.as_u64().unwrap_or(0) as f64 / 1_000_000_000.0;
                            let change = post_value - pre_value;
                            
                            rows.push(Row::new(vec![
                                Cell::from(format!("Balance Change {}:", i)).bold(),
                                Cell::from(format!("◎ {:.9} → ◎ {:.9} (Δ {:.9})",
                                    pre_value,
                                    post_value,
                                    change
                                ).yellow()),
                            ]));
                        }
                    }
                }
            }
        } else {
            rows.push(Row::new(vec![
                Cell::from("Status:").bold(),
                Cell::from("No response available".red()),
            ]));
        }

        let widths = [
            Constraint::Length(20),
            Constraint::Percentage(80),
        ];

        let table = Table::new(rows, &widths)
            .block(block)
            .column_spacing(2);

        table.render(area, buf);
    }
}
