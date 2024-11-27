# SoonScan

SoonScan is a Rust-based Text User Interface (TUI) application for exploring blocks on the Soon blockchain. It provides a simple, interactive way to display and navigate blockchain data in a terminal.

## Features

- **Instant Block Exploration**: Quickly browse and inspect detailed information about blockchain blocks in real-time.
- **Transaction Status Verification**: Instantly verify the current status and details of transactions across the Soon blockchain.
- **Rust Powered**: Built using Ratatui for terminal rendering and async libraries for performance.

## Prerequisites

- Rust (latest stable version)
- Soon Blockchain Node

### Installing Rust

Install Rust using Rustup:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

## Installation

### Clone the Repository

```bash
git clone https://github.com/4rjunc/soonscan.git
cd soonscan
```

### Build the Project

```bash
cargo build --release
```

### Run the Application

```bash
cargo run
```

## Usage

### Keybindings

- **Navigate Rows**:
  - ↑ / k: Move up
  - ↓ / j: Move down
- **Quit Application**:
  - Esc / q

### Dependencies

- `ratatui`: Terminal rendering
- `serde_json`: JSON parsing
- `crossterm`: Terminal input handling

## Contributing

Contributions are welcome! Please follow these steps:

1. Fork the repository
2. Create a new branch:
   ```bash
   git checkout -b feature/your-feature
   ```
3. Commit your changes:
   ```bash
   git commit -m "Add your feature"
   ```
4. Push to the branch:
   ```bash
   git push origin feature/your-feature
   ```
5. Open a pull request

## License

This project is licensed under the MIT License. See the LICENSE file for details.
