// NixBoost - High-performance NixOS package manager frontend
// Copyright (C) 2025 nacreousdawn596, compiledkernel-idk and NixBoost contributors
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

//! Progress bar management for NixBoost.

use indicatif::{ProgressBar, ProgressStyle, MultiProgress};
use std::time::Duration;

/// Progress bar manager
pub struct ProgressManager {
    multi: MultiProgress,
}

impl ProgressManager {
    /// Create a new progress manager
    pub fn new() -> Self {
        Self {
            multi: MultiProgress::new(),
        }
    }

    /// Create a spinner for indeterminate operations
    pub fn spinner(&self, message: &str) -> ProgressBar {
        let pb = self.multi.add(ProgressBar::new_spinner());
        pb.set_style(
            ProgressStyle::with_template("{prefix:.bold.dim} {spinner} {msg}")
                .unwrap()
                .tick_chars("⠁⠂⠄⡀⢀⠠⠐⠈ ")
        );
        pb.set_message(message.to_string());
        pb.enable_steady_tick(Duration::from_millis(80));
        pb
    }

    /// Create a progress bar for determinate operations
    pub fn bar(&self, total: u64, message: &str) -> ProgressBar {
        let pb = self.multi.add(ProgressBar::new(total));
        pb.set_style(
            ProgressStyle::with_template(
                "{prefix:.bold.dim} [{bar:40.cyan/blue}] {pos}/{len} {msg}"
            )
            .unwrap()
            .progress_chars("█▓▒░")
        );
        pb.set_message(message.to_string());
        pb
    }

    /// Create a download progress bar
    pub fn download(&self, total: u64, filename: &str) -> ProgressBar {
        let pb = self.multi.add(ProgressBar::new(total));
        pb.set_style(
            ProgressStyle::with_template(
                "{prefix:.bold.dim} {spinner} [{bar:30.green/dim}] {bytes}/{total_bytes} ({eta}) {msg}"
            )
            .unwrap()
            .progress_chars("━━╺")
            .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏")
        );
        pb.set_prefix("↓");
        pb.set_message(filename.to_string());
        pb.enable_steady_tick(Duration::from_millis(100));
        pb
    }

    /// Create a simple status spinner
    pub fn status(&self, message: &str) -> ProgressBar {
        let pb = self.multi.add(ProgressBar::new_spinner());
        pb.set_style(
            ProgressStyle::with_template("{spinner:.cyan} {msg}")
                .unwrap()
                .tick_chars("⣾⣽⣻⢿⡿⣟⣯⣷")
        );
        pb.set_message(message.to_string());
        pb.enable_steady_tick(Duration::from_millis(100));
        pb
    }

    /// Get the multi-progress for manual management
    pub fn multi(&self) -> &MultiProgress {
        &self.multi
    }
}

impl Default for ProgressManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Create a simple spinner (standalone)
pub fn spinner(message: &str) -> ProgressBar {
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::with_template("{prefix:.bold.dim} {spinner} {msg}")
            .unwrap()
            .tick_chars("⠁⠂⠄⡀⢀⠠⠐⠈ ")
    );
    pb.set_message(message.to_string());
    pb.enable_steady_tick(Duration::from_millis(80));
    pb
}

/// Create a simple progress bar (standalone)
pub fn bar(total: u64) -> ProgressBar {
    let pb = ProgressBar::new(total);
    pb.set_style(
        ProgressStyle::with_template(
            "{prefix:.bold.dim} [{bar:40.cyan/blue}] {pos}/{len} {msg}"
        )
        .unwrap()
        .progress_chars("█▓▒░")
    );
    pb
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_progress_manager() {
        let pm = ProgressManager::new();
        let spinner = pm.spinner("test");
        spinner.finish_and_clear();
    }
}
