use crossterm::{
    event::{self, Event as CrosstermEvent, KeyCode, KeyEvent},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::{
    backend::CrosstermBackend,
    Terminal,
};
use std::io::{self, Stdout};

pub struct App {
    pub running: bool,
}

impl App {
    pub fn new() -> Self {
        Self { running: true }
    }

    pub fn run(&mut self) -> io::Result<()> {
        enable_raw_mode()?;
        io::stdout().execute(EnterAlternateScreen)?;
        
        let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;
        
        while self.running {
            terminal.draw(|f| self.draw(f))?;
            if !self.handle_events()? {
                break;
            }
        }
        
        disable_raw_mode()?;
        io::stdout().execute(LeaveAlternateScreen)?;
        
        Ok(())
    }

    fn draw(&self, f: &mut ratatui::Frame) {
        // Simple dashboard
    }

    fn handle_events(&mut self) -> io::Result<bool> {
        if event::poll(std::time::Duration::from_millis(100))? {
            if let CrosstermEvent::Key(key) = event::read()? {
                return Ok(self.handle_key(key));
            }
        }
        Ok(true)
    }

    fn handle_key(&mut self, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => {
                self.running = false;
                false
            }
            _ => true,
        }
    }
}
