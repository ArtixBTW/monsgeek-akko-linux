//! Reusable TUI widgets shared across tabs.

use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Clear, List, ListItem, ListState},
    Frame,
};

/// A modal, centered single-choice list selector over arbitrary values.
///
/// Generic over the payload `T` each row carries, so it can drive any
/// enum-like choice — key trigger mode, LED mode, a key to bind, an audio
/// style, etc. Navigation is decoupled from rendering so the selection logic
/// is unit-testable without a [`Frame`].
#[derive(Debug, Clone)]
pub(crate) struct PopupSelect<T> {
    title: String,
    items: Vec<(String, T)>,
    /// Indices into `items` matching the current `filter` (all of them when empty).
    /// Navigation and selection operate over this view.
    filtered: Vec<usize>,
    /// Case-insensitive substring typed by the user to narrow a long list.
    filter: String,
    state: ListState,
}

impl<T> PopupSelect<T> {
    /// Build a selector from `(label, value)` rows. The first row is
    /// preselected.
    pub(crate) fn new(title: impl Into<String>, items: Vec<(String, T)>) -> Self {
        let filtered = (0..items.len()).collect();
        let mut state = ListState::default();
        if !items.is_empty() {
            state.select(Some(0));
        }
        Self {
            title: title.into(),
            items,
            filtered,
            filter: String::new(),
            state,
        }
    }

    /// Recompute the filtered view and clamp the selection into it.
    fn refilter(&mut self) {
        let needle = self.filter.to_ascii_lowercase();
        self.filtered = self
            .items
            .iter()
            .enumerate()
            .filter(|(_, (label, _))| {
                needle.is_empty() || label.to_ascii_lowercase().contains(&needle)
            })
            .map(|(i, _)| i)
            .collect();
        if self.filtered.is_empty() {
            self.state.select(None);
        } else {
            let cur = self
                .state
                .selected()
                .unwrap_or(0)
                .min(self.filtered.len() - 1);
            self.state.select(Some(cur));
        }
    }

    /// Append a character to the filter and re-narrow the list.
    pub(crate) fn push_filter(&mut self, c: char) {
        self.filter.push(c);
        self.refilter();
    }

    /// Drop the last filter character and re-widen the list.
    pub(crate) fn pop_filter(&mut self) {
        self.filter.pop();
        self.refilter();
    }

    /// Move the selection up one row (saturating at the top).
    pub(crate) fn up(&mut self) {
        if self.filtered.is_empty() {
            return;
        }
        let i = self.state.selected().unwrap_or(0);
        self.state.select(Some(i.saturating_sub(1)));
    }

    /// Move the selection down one row (saturating at the bottom).
    pub(crate) fn down(&mut self) {
        if self.filtered.is_empty() {
            return;
        }
        let i = self.state.selected().unwrap_or(0);
        self.state
            .select(Some((i + 1).min(self.filtered.len() - 1)));
    }

    /// The value of the currently highlighted row, if any.
    pub(crate) fn selected(&self) -> Option<&T> {
        let &idx = self.filtered.get(self.state.selected()?)?;
        self.items.get(idx).map(|(_, v)| v)
    }

    /// Replace the title, for selectors whose header reflects state built up while
    /// the popup is open.
    pub(crate) fn set_title(&mut self, title: impl Into<String>) {
        self.title = title.into();
    }

    /// Preselect the first (visible) row whose value satisfies `pred` (no-op if
    /// none match), so the popup opens on the current value.
    pub(crate) fn select_where(&mut self, pred: impl Fn(&T) -> bool) {
        if let Some(pos) = self.filtered.iter().position(|&i| pred(&self.items[i].1)) {
            self.state.select(Some(pos));
        }
    }

    /// Render centered over `area`, sized to content and clamped to `area`.
    pub(crate) fn render(&mut self, f: &mut Frame, area: Rect) {
        let title = if self.filter.is_empty() {
            self.title.clone()
        } else {
            format!("{} /{}", self.title, self.filter)
        };
        let content_w = self
            .filtered
            .iter()
            .map(|&i| self.items[i].0.len())
            .max()
            .unwrap_or(0)
            .max(title.len()) as u16;
        // +2 borders, +2 for the "> " highlight symbol.
        let width = (content_w + 4).min(area.width);
        let height = (self.filtered.len() as u16 + 2).min(area.height);
        let x = area.x + area.width.saturating_sub(width) / 2;
        let y = area.y + area.height.saturating_sub(height) / 2;
        let popup = Rect::new(x, y, width, height);

        let items: Vec<ListItem> = self
            .filtered
            .iter()
            .map(|&i| ListItem::new(self.items[i].0.as_str()))
            .collect();
        let list = List::new(items)
            .block(Block::default().borders(Borders::ALL).title(title))
            .highlight_style(
                Style::default()
                    .bg(Color::Blue)
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("> ");

        f.render_widget(Clear, popup);
        f.render_stateful_widget(list, popup, &mut self.state);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> PopupSelect<u8> {
        PopupSelect::new("t", vec![("a".into(), 1), ("b".into(), 2), ("c".into(), 3)])
    }

    #[test]
    fn navigation_saturates_at_both_ends() {
        let mut p = sample();
        assert_eq!(p.selected(), Some(&1));
        p.up();
        assert_eq!(p.selected(), Some(&1)); // saturates at top
        p.down();
        p.down();
        assert_eq!(p.selected(), Some(&3));
        p.down();
        assert_eq!(p.selected(), Some(&3)); // saturates at bottom
    }

    #[test]
    fn select_where_preselects_matching_row() {
        let mut p = sample();
        p.select_where(|&v| v == 2);
        assert_eq!(p.selected(), Some(&2));
        p.select_where(|&v| v == 99); // no match → unchanged
        assert_eq!(p.selected(), Some(&2));
    }

    #[test]
    fn empty_selector_has_no_selection() {
        let p: PopupSelect<u8> = PopupSelect::new("t", vec![]);
        assert_eq!(p.selected(), None);
    }

    #[test]
    fn filter_narrows_navigates_and_clears() {
        let mut p = PopupSelect::new(
            "t",
            vec![
                ("Alpha".into(), 1),
                ("Beta".into(), 2),
                ("Gamma".into(), 3),
                ("Bravo".into(), 4),
            ],
        );
        // Case-insensitive substring; navigation stays inside the filtered view.
        p.push_filter('b');
        assert_eq!(p.selected(), Some(&2)); // "Beta"
        p.down();
        assert_eq!(p.selected(), Some(&4)); // "Bravo"
        p.down();
        assert_eq!(p.selected(), Some(&4)); // saturates within the filtered rows
                                            // No match → nothing selectable.
        p.push_filter('z');
        assert_eq!(p.selected(), None);
        // Backspacing restores the wider list.
        p.pop_filter();
        p.pop_filter();
        assert_eq!(p.selected(), Some(&1)); // "Alpha", filter empty again
    }
}
