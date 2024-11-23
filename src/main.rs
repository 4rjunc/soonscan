use std::io;
use std::env;

use crossterm::terminal;
use ratatui::{
    crossterm::event::{self, KeyCode, KeyEventKind, Event, KeyEvent},
    buffer::Buffer,
    layout::Rect,
    style::Stylize,
    symbols::border,
    text::{Line, Text},
    widgets::{Block, Paragraph, Widget},
    DefaultTerminal, Frame,
};

// this is will store the state of the application
#[derive(Debug, Default)]
pub struct App {
    query: String,
    counter: u8,
    exit: bool,
}

impl App {
    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> io::Result<()>{
        while !self.exit {
            terminal.draw(|frame| self.draw(frame))?;
            self.handle_events()?;
        }
        Ok(())
    }

    fn draw(&self, frame: &mut Frame){
        frame.render_widget(self, frame.area());
    }

    fn handle_events(&mut self) -> io::Result<()>{
        match event::read()?{
            Event::Key(key_events) if key_events.kind == KeyEventKind::Press => {
                self.handle_key_event(key_events);
            }
            _=>{}
        };
        Ok(())
    }

    fn handle_key_event(&mut self, key_events: KeyEvent){
        match key_events.code {
            KeyCode::Char('q') => self.exit(),
            KeyCode::Left => self.decrement_counter(),
            KeyCode::Right => self.increment_counter(),
            _=> {} 
        }
    }

    fn exit(&mut self){
        self.exit = true;
    }

    fn decrement_counter(&mut self){
        self.counter -= 1;
    }

    fn increment_counter(&mut self){
        self.counter += 1;
    }

    fn handle_query(&self){
        println!("Querying blockchain for :{}", self.query);
    }
    
}

impl Widget for &App {
    fn render(self, area: Rect, buf: &mut Buffer){
        let title = Line::from(" SOONSCAN ".bold().red());
        let instruction = Line::from(vec![
            " Decrement ".into(),
            "<Left>".blue().bold(),
            " Increment ".into(),
            "<Right>".blue().bold(),
            " Quit ".into(),
            "<Q> ".blue().bold(),
        ]);
        let block = Block::bordered()
            .title(title.centered())
            .title_bottom(instruction.centered())
            .border_set(border::THICK);

        let counter_text = Text::from(vec![Line::from(vec![
            "Value:".into(),
            self.counter.to_string().yellow(),
        ])]);

        let query_text = Text::from(vec![Line::from(vec![
            "Query:".into(),
            self.query.clone().yellow(),
        ])]);

        Paragraph::new(query_text)
            .centered()
            .block(block)
            .render(area, buf)
    }
    
}

fn main() -> io::Result<()> {

    let args: Vec<String> = env::args().collect();

    // Example: Take command-line argument for initial query (address or transaction hash)
    let initial_query = if args.len() > 1 {
        args[1].clone() // Use the provided argument (address or hash)
    } else {
        String::new() // Default to an empty string if no argument is provided
    };

    let mut terminal = ratatui::init();
    let mut app = App {
        query: initial_query,
        ..App::default()
    };

    let app_result = app.run(&mut terminal);
    ratatui::restore();
    app_result
}

//testing 
#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::style::Style;

    #[test]
    fn render() {
        let app = App::default();
        let mut buf = Buffer::empty(Rect::new(0, 0, 50, 4));

        app.render(buf.area, &mut buf);

        let mut expected = Buffer::with_lines(vec![
            "┏━━━━━━━━━━━━━ Counter App Tutorial ━━━━━━━━━━━━━┓",
            "┃                    Value: 0                    ┃",
            "┃                                                ┃",
            "┗━ Decrement <Left> Increment <Right> Quit <Q> ━━┛",
        ]);
        let title_style = Style::new().bold();
        let counter_style = Style::new().yellow();
        let key_style = Style::new().blue().bold();
        expected.set_style(Rect::new(14, 0, 22, 1), title_style);
        expected.set_style(Rect::new(28, 1, 1, 1), counter_style);
        expected.set_style(Rect::new(13, 3, 6, 1), key_style);
        expected.set_style(Rect::new(30, 3, 7, 1), key_style);
        expected.set_style(Rect::new(43, 3, 4, 1), key_style);

        assert_eq!(buf, expected);
    }

    #[test]
    fn handle_key_event() -> io::Result<()> {
        let mut app = App::default();
        app.handle_key_event(KeyCode::Right.into());
        assert_eq!(app.counter, 1);

        app.handle_key_event(KeyCode::Left.into());
        assert_eq!(app.counter, 0);

        let mut app = App::default();
        app.handle_key_event(KeyCode::Char('q').into());
        assert!(app.exit);

        Ok(())
    }
}
