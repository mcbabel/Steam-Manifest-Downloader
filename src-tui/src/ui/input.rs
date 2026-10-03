use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use unicode_width::UnicodeWidthChar;

#[derive(Debug, Clone, Default)]
pub struct TextInput {
    value: String,
    cursor: usize,
    scroll: usize,
    pub masked: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputOutcome {
    Changed,
    Moved,
    Submit,
    Ignored,
}

impl TextInput {
    pub fn new(value: impl Into<String>) -> Self {
        let mut t = TextInput::default();
        t.set(value);
        t
    }

    pub fn masked(mut self) -> Self {
        self.masked = true;
        self
    }

    pub fn value(&self) -> &str {
        &self.value
    }

    pub fn trimmed(&self) -> String {
        self.value.trim().to_string()
    }

    pub fn is_empty(&self) -> bool {
        self.value.trim().is_empty()
    }

    pub fn set(&mut self, value: impl Into<String>) {
        self.value = value.into().replace(['\n', '\r'], "");
        self.cursor = self.len();
        self.scroll = 0;
    }

    pub fn clear(&mut self) {
        self.set("");
    }

    fn len(&self) -> usize {
        self.value.chars().count()
    }

    fn byte_at(&self, char_idx: usize) -> usize {
        self.value
            .char_indices()
            .nth(char_idx)
            .map(|(b, _)| b)
            .unwrap_or(self.value.len())
    }

    pub fn insert_str(&mut self, s: &str) {
        let clean: String = s.chars().filter(|c| !c.is_control()).collect();
        let at = self.byte_at(self.cursor);
        self.value.insert_str(at, &clean);
        self.cursor += clean.chars().count();
    }

    fn delete_range(&mut self, from: usize, to: usize) {
        if from >= to {
            return;
        }
        let a = self.byte_at(from);
        let b = self.byte_at(to);
        self.value.replace_range(a..b, "");
        self.cursor = from;
    }

    fn word_left(&self) -> usize {
        let chars: Vec<char> = self.value.chars().collect();
        let mut i = self.cursor;
        while i > 0 && !chars[i - 1].is_alphanumeric() {
            i -= 1;
        }
        while i > 0 && chars[i - 1].is_alphanumeric() {
            i -= 1;
        }
        i
    }

    fn word_right(&self) -> usize {
        let chars: Vec<char> = self.value.chars().collect();
        let mut i = self.cursor;
        while i < chars.len() && !chars[i].is_alphanumeric() {
            i += 1;
        }
        while i < chars.len() && chars[i].is_alphanumeric() {
            i += 1;
        }
        i
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> InputOutcome {
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        let alt = key.modifiers.contains(KeyModifiers::ALT);
        match key.code {
            KeyCode::Enter => InputOutcome::Submit,
            KeyCode::Char(c) if ctrl => match c {
                'a' => {
                    self.cursor = 0;
                    InputOutcome::Moved
                }
                'e' => {
                    self.cursor = self.len();
                    InputOutcome::Moved
                }
                'u' => {
                    self.delete_range(0, self.cursor);
                    InputOutcome::Changed
                }
                'k' => {
                    self.delete_range(self.cursor, self.len());
                    InputOutcome::Changed
                }
                'w' | 'h' => {
                    if c == 'h' {
                        if self.cursor > 0 {
                            self.delete_range(self.cursor - 1, self.cursor);
                        }
                    } else {
                        let to = self.cursor;
                        self.delete_range(self.word_left(), to);
                    }
                    InputOutcome::Changed
                }
                _ => InputOutcome::Ignored,
            },
            KeyCode::Char(c) if !alt => {
                self.insert_str(&c.to_string());
                InputOutcome::Changed
            }
            KeyCode::Backspace => {
                if ctrl || alt {
                    let to = self.cursor;
                    self.delete_range(self.word_left(), to);
                } else if self.cursor > 0 {
                    self.delete_range(self.cursor - 1, self.cursor);
                }
                InputOutcome::Changed
            }
            KeyCode::Delete => {
                if self.cursor < self.len() {
                    self.delete_range(self.cursor, self.cursor + 1);
                }
                InputOutcome::Changed
            }
            KeyCode::Left => {
                self.cursor = if ctrl {
                    self.word_left()
                } else {
                    self.cursor.saturating_sub(1)
                };
                InputOutcome::Moved
            }
            KeyCode::Right => {
                self.cursor = if ctrl {
                    self.word_right()
                } else {
                    (self.cursor + 1).min(self.len())
                };
                InputOutcome::Moved
            }
            KeyCode::Home => {
                self.cursor = 0;
                InputOutcome::Moved
            }
            KeyCode::End => {
                self.cursor = self.len();
                InputOutcome::Moved
            }
            _ => InputOutcome::Ignored,
        }
    }

    pub fn click(&mut self, col: u16) {
        let mut width = 0usize;
        let mut idx = self.scroll;
        for c in self.display_chars().skip(self.scroll) {
            let w = c.width().unwrap_or(1);
            if width + w > col as usize {
                break;
            }
            width += w;
            idx += 1;
        }
        self.cursor = idx.min(self.len());
    }

    pub fn display_value(&self) -> String {
        self.display_chars().collect()
    }

    fn display_chars(&self) -> impl Iterator<Item = char> + '_ {
        let masked = self.masked;
        self.value
            .chars()
            .map(move |c| if masked { '•' } else { c })
    }

    pub fn view(&mut self, width: u16) -> (String, u16) {
        let width = width.max(1) as usize;
        let chars: Vec<char> = self.display_chars().collect();
        if self.cursor < self.scroll {
            self.scroll = self.cursor;
        }
        loop {
            let w: usize = chars[self.scroll..self.cursor]
                .iter()
                .map(|c| c.width().unwrap_or(1))
                .sum();
            if w < width || self.scroll >= self.cursor {
                break;
            }
            self.scroll += 1;
        }
        let mut out = String::new();
        let mut used = 0usize;
        let mut cursor_col = 0usize;
        for (i, c) in chars.iter().enumerate().skip(self.scroll) {
            if i == self.cursor {
                cursor_col = used;
            }
            let w = c.width().unwrap_or(1);
            if used + w > width {
                break;
            }
            out.push(*c);
            used += w;
        }
        if self.cursor >= chars.len() {
            cursor_col = used;
        }
        (out, cursor_col.min(width - 1) as u16)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    #[test]
    fn edits_at_cursor() {
        let mut t = TextInput::new("hello");
        t.handle_key(key(KeyCode::Left));
        t.handle_key(key(KeyCode::Char('X')));
        assert_eq!(t.value(), "hellXo");
        t.handle_key(key(KeyCode::Home));
        t.handle_key(key(KeyCode::Delete));
        assert_eq!(t.value(), "ellXo");
        t.handle_key(KeyEvent::new(KeyCode::Char('k'), KeyModifiers::CONTROL));
        assert_eq!(t.value(), "");
    }

    #[test]
    fn word_delete_and_unicode() {
        let mut t = TextInput::new("Größe 220 übrig");
        t.handle_key(KeyEvent::new(KeyCode::Char('w'), KeyModifiers::CONTROL));
        assert_eq!(t.value(), "Größe 220 ");
        t.handle_key(key(KeyCode::Backspace));
        assert_eq!(t.value(), "Größe 220");
    }

    #[test]
    fn view_scrolls_to_cursor() {
        let mut t = TextInput::new("0123456789");
        let (text, col) = t.view(5);
        assert_eq!(text, "6789");
        assert_eq!(col, 4);
        t.handle_key(key(KeyCode::Home));
        let (text, col) = t.view(5);
        assert_eq!(text, "01234");
        assert_eq!(col, 0);
    }

    #[test]
    fn masks_secrets() {
        let mut t = TextInput::new("key").masked();
        assert_eq!(t.view(10).0, "•••");
        assert_eq!(t.value(), "key");
    }
}
