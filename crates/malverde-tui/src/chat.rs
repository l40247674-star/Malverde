use std::io;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;

use crossterm::{
    event::{self, Event as CrosstermEvent, KeyCode, KeyEvent, KeyModifiers},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::{
    backend::CrosstermBackend,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Terminal,
};

use malverde_agent::{Agent, SilentObserver};
use malverde_core::SessionId;

enum WorkerMsg {
    Done { ok: bool, text: String },
}

/// TUI de chat: historial + input, con el agente corriendo en un hilo aparte.
pub struct ChatTui {
    agent: Arc<Agent>,
    session: SessionId,
    lines: Vec<Line<'static>>,
    input: String,
    running: bool,
    busy: bool,
    rx: Receiver<WorkerMsg>,
    tx: Option<Sender<WorkerMsg>>,
}

impl ChatTui {
    pub fn new(agent: Arc<Agent>, session: SessionId) -> Self {
        let (tx, rx) = channel();
        Self {
            agent,
            session,
            lines: vec![
                Line::from(Span::styled(
                    "Malverde TUI · escribe un mensaje · /new nueva sesión · /tools herramientas · Esc salir",
                    Style::default().fg(Color::DarkGray),
                )),
                Line::from(""),
            ],
            input: String::new(),
            running: true,
            busy: false,
            rx,
            tx: Some(tx),
        }
    }

    pub fn run(&mut self) -> io::Result<()> {
        enable_raw_mode()?;
        io::stdout().execute(EnterAlternateScreen)?;
        let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;

        while self.running {
            while let Ok(msg) = self.rx.try_recv() {
                let WorkerMsg::Done { ok, text } = msg;
                let prefix = if ok { "malverde> " } else { "error> " };
                self.push(&format!("{prefix}{text}"));
                self.busy = false;
            }

            terminal.draw(|f| self.draw(f))?;
            if !event::poll(std::time::Duration::from_millis(100))? {
                continue;
            }
            if let CrosstermEvent::Key(key) = event::read()? {
                self.handle_key(key);
            }
        }

        disable_raw_mode()?;
        io::stdout().execute(LeaveAlternateScreen)?;
        Ok(())
    }

    fn draw(&self, f: &mut ratatui::Frame) {
        use ratatui::layout::Rect;
        let area = f.size();
        let input_height = 3u16;
        let status_height = 1u16;
        let history_height = area.height.saturating_sub(input_height + status_height);

        let history = Paragraph::new(self.lines.clone())
            .wrap(Wrap { trim: false })
            .block(Block::default().borders(Borders::ALL).title(" Malverde "));
        f.render_widget(history, Rect::new(0, 0, area.width, history_height));

        let status = if self.busy { "pensando…" } else { "listo" };
        f.render_widget(
            Paragraph::new(Span::styled(status, Style::default().fg(Color::DarkGray))),
            Rect::new(2, history_height, area.width, status_height),
        );

        let input = Paragraph::new(self.input.as_str())
            .block(Block::default().borders(Borders::ALL).title(" mensaje "));
        f.render_widget(
            input,
            Rect::new(0, history_height + status_height, area.width, input_height),
        );
    }

    fn push(&mut self, text: &str) {
        for l in text.lines() {
            self.lines.push(Line::from(l.to_string()));
        }
        self.lines.push(Line::from(""));
    }

    fn handle_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => self.running = false,
            KeyCode::Esc => self.running = false,
            KeyCode::Enter => self.submit(),
            KeyCode::Backspace => {
                self.input.pop();
            }
            KeyCode::Char(c) => self.input.push(c),
            _ => {}
        }
    }

    fn submit(&mut self) {
        let text = self.input.trim().to_string();
        self.input.clear();
        if text.is_empty() || self.busy {
            return;
        }
        match text.as_str() {
            "/exit" | "/quit" => {
                self.running = false;
                return;
            }
            "/new" => {
                self.session = SessionId::new();
                self.push("[nueva sesión]".into());
                return;
            }
            "/tools" => {
                for name in self.agent.registry.enabled_names() {
                    self.push(&format!("  • {name}"));
                }
                return;
            }
            _ => {}
        }
        self.push(&format!("tú> {text}"));
        if let Some(tx) = self.tx.take() {
            self.busy = true;
            let agent = Arc::clone(&self.agent);
            let session = self.session.clone();
            std::thread::spawn(move || {
                let msg = match agent.run(&session, &text, &SilentObserver) {
                    Ok(res) => WorkerMsg::Done { ok: true, text: res.final_text },
                    Err(e) => WorkerMsg::Done { ok: false, text: e.to_string() },
                };
                let _ = tx.send(msg);
            });
        } else {
            self.push("[error] sin canal del worker; reinicia la TUI".into());
        }
    }
}
