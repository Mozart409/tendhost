//! Event handling for terminal and application events

use std::time::Duration;

use crossterm::event::{self, Event as CrosstermEvent, KeyCode, KeyEvent, KeyModifiers};
use tokio::sync::mpsc;

use crate::action::Action;

/// Terminal event types
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum Event {
    /// Terminal key event
    Key(KeyEvent),
    /// Terminal resize
    Resize(u16, u16),
    /// Tick for animations
    Tick,
}

/// Event handler that polls for terminal events
pub struct EventHandler {
    /// Event sender
    sender: mpsc::UnboundedSender<Event>,
    /// Event receiver
    receiver: mpsc::UnboundedReceiver<Event>,
    /// Tick rate
    tick_rate: Duration,
}

impl EventHandler {
    /// Create a new event handler
    pub fn new(tick_rate: Duration) -> Self {
        let (sender, receiver) = mpsc::unbounded_channel();
        Self {
            sender,
            receiver,
            tick_rate,
        }
    }

    /// Start the event loop in a background task
    pub fn start(&self) {
        let sender = self.sender.clone();
        let tick_rate = self.tick_rate;

        tokio::spawn(async move {
            let mut last_tick = std::time::Instant::now();

            loop {
                // Calculate timeout until next tick
                let timeout = tick_rate
                    .checked_sub(last_tick.elapsed())
                    .unwrap_or(Duration::ZERO);

                // Poll for events
                if event::poll(timeout).unwrap_or(false) {
                    match event::read() {
                        Ok(CrosstermEvent::Key(key)) => {
                            if sender.send(Event::Key(key)).is_err() {
                                break;
                            }
                        }
                        Ok(CrosstermEvent::Resize(w, h)) => {
                            if sender.send(Event::Resize(w, h)).is_err() {
                                break;
                            }
                        }
                        _ => {}
                    }
                }

                // Send tick event
                if last_tick.elapsed() >= tick_rate {
                    if sender.send(Event::Tick).is_err() {
                        break;
                    }
                    last_tick = std::time::Instant::now();
                }
            }
        });
    }

    /// Receive the next event
    pub async fn next(&mut self) -> Option<Event> {
        self.receiver.recv().await
    }
}

/// Convert a key event to an action
pub fn key_to_action(key: KeyEvent, search_active: bool) -> Action {
    if search_active {
        match key.code {
            KeyCode::Esc | KeyCode::Enter => Action::Back,
            KeyCode::Backspace => Action::SearchBackspace,
            KeyCode::Char(c) => Action::SearchInput(c),
            _ => Action::None,
        }
    } else {
        match key.code {
            // Quit
            KeyCode::Char('q') => Action::Quit,
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => Action::Quit,

            // Navigation
            KeyCode::Up | KeyCode::Char('k') => Action::Up,
            KeyCode::Down | KeyCode::Char('j') => Action::Down,
            KeyCode::Char('g') => Action::First,
            KeyCode::Char('G') => Action::Last,
            KeyCode::Enter => Action::Select,
            KeyCode::Esc => Action::Back,
            KeyCode::Tab => Action::ToggleFocus,

            // Actions
            KeyCode::Char('u') => Action::TriggerUpdate,
            KeyCode::Char('U') => Action::TriggerFleetUpdate,
            KeyCode::Char('r') => Action::TriggerReboot,
            KeyCode::Char('R') => Action::RetryHost,
            KeyCode::Char('a') => Action::AcknowledgeFailure,
            KeyCode::Char('i') => Action::RefreshInventory,

            // Help and search
            KeyCode::Char('?') => Action::Help,
            KeyCode::Char('/') => Action::StartSearch,

            _ => Action::None,
        }
    }
}
