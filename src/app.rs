use crate::diff::Diff;
use crate::render::{build_rows, FileView};
use crate::theme::Theme;

pub struct App {
    base_diff: Diff,
    combined_diff: Option<Diff>,
    pub show_untracked: bool,
    pub file_index: usize,
    pub scroll: usize,
    pub theme: Theme,
    view_cache: Option<ViewCache>,
}

struct ViewCache {
    file_index: usize,
    show_untracked: bool,
    left_width: usize,
    right_width: usize,
    view: FileView,
}

impl App {
    pub fn new(base_diff: Diff) -> Self {
        App {
            base_diff,
            combined_diff: None,
            show_untracked: false,
            file_index: 0,
            scroll: 0,
            theme: Theme::github_dark(),
            view_cache: None,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.current_diff().files.is_empty()
    }

    pub fn current_file_name(&self) -> String {
        if self.is_empty() {
            return "".to_string();
        }
        let file = &self.current_diff().files[self.file_index];
        if file.old_path == file.new_path {
            file.new_path.clone()
        } else {
            format!("{} -> {}", file.old_path, file.new_path)
        }
    }

    pub fn file_count(&self) -> usize {
        self.current_diff().files.len()
    }

    pub fn line_digits(&self) -> (usize, usize) {
        if self.is_empty() {
            return (1, 1);
        }
        let file = &self.current_diff().files[self.file_index];
        (digits(file.max_old_line), digits(file.max_new_line))
    }

    pub fn view(&mut self, left_width: usize, right_width: usize) -> Option<&FileView> {
        if self.is_empty() {
            return None;
        }

        let needs_rebuild = match &self.view_cache {
            Some(cache) => {
                cache.file_index != self.file_index
                    || cache.show_untracked != self.show_untracked
                    || cache.left_width != left_width
                    || cache.right_width != right_width
            }
            None => true,
        };

        if needs_rebuild {
            let file = &self.current_diff().files[self.file_index];
            let view = build_rows(file, left_width, right_width);
            self.view_cache = Some(ViewCache {
                file_index: self.file_index,
                show_untracked: self.show_untracked,
                left_width,
                right_width,
                view,
            });
        }

        Some(&self.view_cache.as_ref().expect("view cache").view)
    }

    pub fn scroll_by(&mut self, delta: i32, max_height: usize, total_rows: usize) {
        if total_rows == 0 {
            self.scroll = 0;
            return;
        }
        let max_scroll = total_rows.saturating_sub(max_height);
        let next = (self.scroll as i32 + delta).clamp(0, max_scroll as i32) as usize;
        self.scroll = next;
    }

    pub fn jump_to_start(&mut self) {
        self.scroll = 0;
    }

    pub fn jump_to_end(&mut self, max_height: usize, total_rows: usize) {
        if total_rows == 0 {
            self.scroll = 0;
            return;
        }
        self.scroll = total_rows.saturating_sub(max_height);
    }

    pub fn next_file(&mut self) {
        if self.file_count() == 0 {
            return;
        }
        self.file_index = (self.file_index + 1).min(self.file_count() - 1);
        self.scroll = 0;
        self.view_cache = None;
    }

    pub fn prev_file(&mut self) {
        if self.file_count() == 0 {
            return;
        }
        if self.file_index > 0 {
            self.file_index -= 1;
        }
        self.scroll = 0;
        self.view_cache = None;
    }

    pub fn jump_to_file(&mut self, index: usize) {
        if self.file_count() == 0 {
            return;
        }
        let target = index.min(self.file_count().saturating_sub(1));
        self.file_index = target;
        self.scroll = 0;
        self.view_cache = None;
    }

    pub fn next_hunk(&mut self, left_width: usize, right_width: usize) {
        let current_scroll = self.scroll;
        let Some(view) = self.view(left_width, right_width) else {
            return;
        };
        if view.hunk_starts.is_empty() {
            return;
        }
        let mut next = None;
        for &start in &view.hunk_starts {
            if start > current_scroll {
                next = Some(start);
                break;
            }
        }
        if let Some(start) = next {
            self.scroll = start;
        }
    }

    pub fn prev_hunk(&mut self, left_width: usize, right_width: usize) {
        let current_scroll = self.scroll;
        let Some(view) = self.view(left_width, right_width) else {
            return;
        };
        if view.hunk_starts.is_empty() {
            return;
        }
        let mut prev = None;
        for &start in &view.hunk_starts {
            if start < current_scroll {
                prev = Some(start);
            }
        }
        if let Some(start) = prev {
            self.scroll = start;
        }
    }

    pub fn toggle_untracked(&mut self) {
        if self.combined_diff.is_none() {
            return;
        }
        self.show_untracked = !self.show_untracked;
        let count = self.file_count();
        if self.file_index >= count {
            self.file_index = count.saturating_sub(1);
        }
        self.scroll = 0;
        self.view_cache = None;
    }

    fn current_diff(&self) -> &Diff {
        if self.show_untracked {
            if let Some(combined) = &self.combined_diff {
                return combined;
            }
        }
        &self.base_diff
    }

    pub fn has_untracked(&self) -> bool {
        self.combined_diff.is_some()
    }

    pub fn set_untracked(&mut self, untracked: Diff) {
        if untracked.files.is_empty() {
            self.combined_diff = None;
            return;
        }
        self.combined_diff = Some(merge_diffs(self.base_diff.clone(), untracked));
    }
}

fn digits(value: u32) -> usize {
    let value = value.max(1);
    value.to_string().len()
}

fn merge_diffs(mut base: Diff, extra: Diff) -> Diff {
    base.files.extend(extra.files);
    base
}
