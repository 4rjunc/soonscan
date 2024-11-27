use std::sync::Arc;
use std::io;
use tokio::sync::Mutex;

use reqwest::Client;
use serde_json::Value;

use ratatui::crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::{
    backend::CrosstermBackend,
    buffer::Buffer,
    layout::{Constraint, Layout, Rect},
    prelude::Alignment,
    style::{Style, Stylize},
    symbols::border,
    text::Line,
    widgets::{Block, Cell, Clear, Paragraph, Row, Table, Widget},
    Frame, Terminal,
};

// RPC Client
use solana_client::rpc_client::RpcClient;
use solana_sdk::{pubkey::Pubkey, signature::Signature};
use solana_transaction_status_client_types::{
    EncodedTransaction::Json, UiMessage::Raw, UiTransactionEncoding,
};
use std::str::FromStr;

const DEVNET_RPC: &str = "https://rpc.devnet.soo.network/rpc";
const TESTNET_RPC: &str = "https://rpc.testnet.soo.network/rpc";

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RpcNetwork {
    Devnet,
    Testnet,
}

impl RpcNetwork {
    // Method to get the RPC URL for the current network
    pub fn get_url(&self) -> &'static str {
        match self {
            RpcNetwork::Devnet => DEVNET_RPC,
            RpcNetwork::Testnet => TESTNET_RPC,
        }
    }

    // Method to display the network name
    pub fn name(&self) -> &'static str {
        match self {
            RpcNetwork::Devnet => "Devnet",
            RpcNetwork::Testnet => "Testnet",
        }
    }
}

#[derive(Debug)]
pub struct App {
    pub query: String,
    pub input_mode: InputMode,
    pub slot_info: Option<i64>,
    pub transaction_info: Option<i64>,
    pub supply_info: Option<Value>,
    pub json_response: Option<Value>,
    pub address_sign: Option<Value>,
    pub exit: bool,
    pub show_popup: bool,
    pub current_rpc_network: RpcNetwork,  // Changed from String to RpcNetwork
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
            address_sign: None,
            exit: false,
            show_popup: false,
            current_rpc_network: RpcNetwork::Devnet,
            client: Client::new(),
        }
    }
}

impl App {
    //toggle RPCs
     pub fn toggle_rpc_network(&mut self) {
        // Toggle between Devnet and Testnet
        self.current_rpc_network = match self.current_rpc_network {
            RpcNetwork::Devnet => RpcNetwork::Testnet,
            RpcNetwork::Testnet => RpcNetwork::Devnet,
        };
    }

    pub fn get_current_rpc_url(&self) -> &str {
        self.current_rpc_network.get_url()
    }        

    //Fetch Intial Blockchain data
    pub async fn fetch_initial_blockchain_data(
        &mut self,
    ) -> Result<(), Box<dyn std::error::Error>> {
        
        let current_rpc_url = self.get_current_rpc_url();
        // Fetch slot Info
        let slot_payload = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "getSlot",
        });

        let slot_response = self
            .client
            .post(current_rpc_url)
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

        let current_rpc_url = self.get_current_rpc_url();
        let supply_response = self
            .client
            .post(current_rpc_url)
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
            Layout::horizontal([Constraint::Percentage(40),Constraint::Percentage(20), Constraint::Percentage(40)])
                .split(chunks[0]);

        // Toggle with the N button
        let input_title = match self.current_rpc_network {
            RpcNetwork::Devnet => format!(" SOONSCAN {} ", " 🌐 Devnet ".green()),
            RpcNetwork::Testnet => format!(" SOONSCAN {} ", " 🌐 Testnet ".blue()),
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

        frame.render_widget(instructions, bottom_layout[2]);

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
                Line::from(vec![" n      : Toggle between Devnet and Testnet".blue()]),
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
                    KeyCode::Char('n') => {
                        let mut app = app.lock().await;
                        app.toggle_rpc_network();
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
                    // Handle paste events (Ctrl+V)
                    KeyCode::Char('v') => {
                        let mut app = app.lock().await;
                        if matches!(app.input_mode, InputMode::Editing)
                            && key_event.modifiers.contains(event::KeyModifiers::CONTROL)
                        {
                            if let Ok(clipboard_content) = cli_clipboard::get_contents() {
                                app.query.push_str(&clipboard_content);
                            }
                        } else if matches!(app.input_mode, InputMode::Editing) {
                            app.query.push('v');
                        }
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
        // Define the RPC URL
        let url = DEVNET_RPC;
        let client = RpcClient::new(url.to_string());

        // Check if the query is a valid public key
        if let Ok(pubkey) = Pubkey::from_str(&self.query) {
            // println!("Valid public key detected: {}", pubkey);

            // Fetch account information using Solana RPC client
            match client.get_account(&pubkey) {
                Ok(account) => {
                    // println!("Account found: {:?}", account);
                    let account_info = serde_json::json!({
                        "lamports": account.lamports,
                        "owner": account.owner.to_string(),
                        "space": account.data.len(),
                        "executable": account.executable,
                    });
                    self.json_response = Some(account_info);

                    // Fetch signatures related to an account
                    match client.get_signatures_for_address(&pubkey) {
                        Ok(signatures) => {
                            self.address_sign = Some(serde_json::json!(signatures));
                        }
                        Err(err) => {
                            eprintln!("Failed to fetch signatures: {}", err);
                            self.address_sign = None;
                        }
                    }
                }
                Err(err) => {
                    eprintln!("Failed to fetch account info: {}", err);
                    self.json_response = None;
                    self.address_sign = None;
                }
            }

        } else if let Ok(signature) = Signature::from_str(&self.query) {
            // println!("Valid transaction signature detected: {}", signature);
            // Fetch transaction details using Solana RPC client
            match client.get_transaction(&signature, UiTransactionEncoding::Json) {
                Ok(transaction) => {
                    let transaction_info = serde_json::json!({
                        "slot": transaction.slot,
                        "blockTime": transaction.block_time,
                        "meta": {
                            "status": transaction.transaction.meta.as_ref().map(|m| format!("{:?}", m.status)),
                            "err": transaction.transaction.meta.as_ref().and_then(|m| m.err.clone()),
                            "fee": transaction.transaction.meta.as_ref().map(|m| m.fee).unwrap_or(0),
                            "preBalances": transaction.transaction.meta.as_ref().map(|m| m.pre_balances.clone()),
                            "postBalances": transaction.transaction.meta.as_ref().map(|m| m.post_balances.clone()),
                            "signatures": match &transaction.transaction.transaction {
                                                    Json(ui_transaction) => ui_transaction.signatures.clone(),
                                                    _ => vec![]
                            },
                            "accountKeys": match &transaction.transaction.transaction {
                                                    Json(ui_transaction) => match &ui_transaction.message {
                                                        Raw(raw_message) => raw_message.account_keys.clone(),
                                                        _ => vec![]
                                                    },
                                                    _ => vec![]
                                                },
                                                "recentBlockhash": match &transaction.transaction.transaction {
                                                    Json(ui_transaction) => match &ui_transaction.message {
                                                        Raw(raw_message) => raw_message.recent_blockhash.clone(),
                                                        _ => String::new()
                                                    },
                                                    _ => String::new()
                                                },
                                                "instructions": match &transaction.transaction.transaction {
                                                    Json(ui_transaction) => match &ui_transaction.message {
                                                        Raw(raw_message) => raw_message.instructions.clone(),
                                                        _ => vec![]
                                                    },
                                                    _ => vec![]
                                                },
                            "logMessages": transaction.transaction.meta.as_ref().and_then(|m| Some(m.log_messages.clone())),
                            "computeUnitsConsumed": transaction.transaction.meta.as_ref().and_then(|m| Some(m.compute_units_consumed.clone()))
                        },
                    });
                    self.json_response = Some(transaction_info);
                }
                Err(err) => {
                    eprintln!("Failed to fetch transaction info: {}", err);
                    self.json_response = None;
                }
            }
        } else {
            eprintln!("Query is neither a valid public key nor a transaction signature.");
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
                                format!(
                                    "{} / {}",
                                    self.format_longnumber(circulating_supply),
                                    self.format_longnumber(total_supply)
                                )
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
        // println!("Address Signatures: {:?}", self.address_sign);
            if let Some(response_obj) = json_response.as_object() {
                if response_obj.contains_key("lamports") {
                    // This is an account response
                    rows.extend(vec![
                        Row::new(vec![
                            Cell::from("Type:").bold(),
                            Cell::from("Account Info".blue()),
                        ]),
                        Row::new(vec![
                            Cell::from("Balance (SOL):").bold(),
                            Cell::from(
                                format!(
                                    "◎ {:.9}",
                                    response_obj
                                        .get("lamports")
                                        .and_then(|l| l.as_u64())
                                        .unwrap_or(0) as f64
                                        / 1_000_000_000.0
                                )
                                .yellow(),
                            ),
                        ]),
                        Row::new(vec![
                            Cell::from("Allocated Data Size:").bold(),
                            Cell::from(
                                format!(
                                    "{} byte(s)",
                                    response_obj
                                        .get("space")
                                        .and_then(|s| s.as_u64())
                                        .unwrap_or(0)
                                )
                                .yellow(),
                            ),
                        ]),
                        Row::new(vec![
                            Cell::from("Assigned Program Id:").bold(),
                            Cell::from(
                                response_obj
                                    .get("owner")
                                    .and_then(|o| o.as_str())
                                    .map(|owner| {
                                        if owner == "11111111111111111111111111111111" {
                                            "System Program".to_string()
                                        } else {
                                            owner.to_string()
                                        }
                                    })
                                    .unwrap_or("N/A".to_string())
                                    .green(),
                            ),
                        ]),
                        Row::new(vec![
                            Cell::from("Executable:").bold(),
                            Cell::from(
                                if response_obj
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
                    ]);


if let Some(address_sign) = &self.address_sign {
    // Check if the value inside `address_sign` is an array
    if let Some(address_signatures) = address_sign.as_array() {
        // Iterate over the array of signatures

            rows.push(Row::new(vec![
                Cell::from(" "),
            ]));


            rows.push(Row::new(vec![
                Cell::from("Transaction History").bold(),
            ]));


            rows.push(Row::new(vec![
                Cell::from(" "),
            ]));

            rows.push(Row::new(vec![
                Cell::from("Transaction").bold(),
                Cell::from("Block").bold(),
                Cell::from("Timestamp").bold(),
                Cell::from("Result").bold(),

            ]));


        for signature_info in address_signatures {
            // Extract relevant fields from each signature info object
            let signature = signature_info
                .get("signature")
                .and_then(|s| s.as_str())
                .unwrap_or("N/A");
            
            let slot = signature_info
                .get("slot")
                .and_then(|s| s.as_u64())
                .unwrap_or(0);

            let block_time = signature_info
                .get("blockTime")
                                    .and_then(|bt| bt.as_u64())
                                    .map_or("N/A".to_string(), |time| {
                                        self.format_timestamp(time as i64)

                                    });

            let confirmation_status = signature_info
                .get("confirmationStatus")
                .and_then(|s| s.as_str())
                .unwrap_or("Unknown");



            // Create rows for each signature's details
            rows.push(Row::new(vec![
                Cell::from(format!("{}...", &signature[0..23]).yellow()),

                Cell::from(format!("{}", self.format_longnumber(slot as i64)).to_string().blue()),

                Cell::from(block_time.yellow()),
                
                Cell::from(confirmation_status.green()),
            ]));
        }
    }
}


                } else if response_obj.contains_key("slot") {
                    // This is a transaction response
                    // println!("Transaction Data: {:?}", self.json_response);
                    rows.extend(vec![
                        Row::new(vec![
                            Cell::from("Type:").bold(),
                            Cell::from("Transaction Info".blue()),
                        ]),
                        Row::new(vec![
                            Cell::from("Slot:").bold(),
                            Cell::from(
                                response_obj
                                    .get("slot")
                                    .and_then(|s| s.as_u64())
                                    .map_or("N/A".to_string(), |slot| {
                                        self.format_longnumber(slot as i64)
                                    })
                                    .yellow(),
                            ),
                        ]),
                        Row::new(vec![
                            Cell::from("Block Time:").bold(),
                            Cell::from(
                                response_obj
                                    .get("blockTime")
                                    .and_then(|bt| bt.as_u64())
                                    .map_or("N/A".to_string(), |time| {
                                        self.format_timestamp(time as i64)
                                    })
                                    .yellow(),
                            ),
                        ]),
                        Row::new(vec![
                            Cell::from("Fee (SOL):").bold(),
                            Cell::from(
                                response_obj
                                    .get("meta")
                                    .and_then(|meta| meta.get("fee"))
                                    .and_then(|f| f.as_u64())
                                    .map_or("N/A".to_string(), |fee| {
                                        format!("◎ {:.9}", fee as f64 / 1_000_000_000.0)
                                    })
                                    .yellow(),
                            ),
                        ]),
                        Row::new(vec![
                            Cell::from("Status:").bold(),
                            Cell::from(
                                response_obj
                                    .get("meta")
                                    .and_then(|meta| meta.get("status"))
                                    .and_then(|status| {
                                        if let Some(status_str) = status.as_str() {
                                            // Handle the 'Ok(())' status
                                            if status_str == "Ok(())" {
                                                Some("SUCCESS".to_string())
                                            } else {
                                                Some("Err".to_string())
                                            }
                                        } else {
                                            None
                                        }
                                    })
                                    .unwrap_or("Unknown".to_string())
                                    .green(),
                            ),
                        ]),
                        Row::new(vec![
                            Cell::from("Signatures:").bold(),
                            Cell::from(format!("{}...", &self.query[0..24])).red(),
                        ]),
                    ]);
                } else {
                    // Handle unknown or unsupported response type
                    rows.push(Row::new(vec![
                        Cell::from("Error:").bold(),
                        Cell::from("Unsupported response type.".red()),
                    ]));
                }
            }
        } else if !self.query.is_empty() {
            rows.push(Row::new(vec![
                Cell::from("Status:").bold(),
                Cell::from("Loading...".yellow()),
            ]));
        }

        let widths = [Constraint::Length(40), Constraint::Percentage(20), Constraint::Percentage(15), Constraint::Percentage(15)];

        let table = Table::new(rows, &widths).block(block).column_spacing(2);

        table.render(area, buf);
    }
}

fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let popup_width = area.width * percent_x / 100;
    let popup_height = area.height * percent_y / 100;
    let popup_x = (area.width - popup_width) / 2;
    let popup_y = (area.height - popup_height) / 2;

    Rect::new(popup_x, popup_y, popup_width, popup_height)
}
