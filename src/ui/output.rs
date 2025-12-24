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

//! Output formatting for NixBoost.

use crate::cli::args::OutputFormat;
use crate::core::types::{Package, SearchResult};
use comfy_table::{Table, presets::UTF8_FULL, presets::ASCII_BORDERS_ONLY_CONDENSED};
use console::style;
use serde::Serialize;

/// Output formatter
pub struct Output {
    format: OutputFormat,
    colors: bool,
}

impl Output {
    /// Create a new output formatter
    pub fn new(format: OutputFormat) -> Self {
        Self {
            format,
            colors: true,
        }
    }

    /// Disable colors
    pub fn no_colors(mut self, disable: bool) -> Self {
        if disable {
            self.colors = false;
        }
        self
    }

    /// Print packages
    pub fn print_packages(&self, packages: &[Package]) {
        match self.format {
            OutputFormat::Human => self.print_packages_human(packages),
            OutputFormat::Json => self.print_json(packages),
            OutputFormat::Plain => self.print_packages_plain(packages),
        }
    }

    /// Print search results
    pub fn print_search_results(&self, results: &[SearchResult]) {
        match self.format {
            OutputFormat::Human => self.print_search_human(results),
            OutputFormat::Json => {
                let packages: Vec<&Package> = results.iter().map(|r| &r.package).collect();
                self.print_json(&packages);
            }
            OutputFormat::Plain => self.print_search_plain(results),
        }
    }

    /// Print packages in human-readable format
    fn print_packages_human(&self, packages: &[Package]) {
        for pkg in packages {
            if self.colors {
                println!(
                    "{}/{} {}\n    {}",
                    style(&pkg.source).cyan().bold(),
                    style(&pkg.name).bold(),
                    style(&pkg.version).green(),
                    pkg.description
                );
            } else {
                println!(
                    "{}/{} {}\n    {}",
                    pkg.source, pkg.name, pkg.version, pkg.description
                );
            }
        }
    }

    /// Print packages in plain format
    fn print_packages_plain(&self, packages: &[Package]) {
        for pkg in packages {
            println!("{} {} - {}", pkg.name, pkg.version, pkg.description);
        }
    }

    /// Print search results in human-readable format
    fn print_search_human(&self, results: &[SearchResult]) {
        for result in results {
            let pkg = &result.package;
            if self.colors {
                println!(
                    "{}/{} {}\n    {}",
                    style(&pkg.source).cyan().bold(),
                    style(&pkg.name).bold(),
                    style(&pkg.version).green(),
                    pkg.description
                );
            } else {
                println!(
                    "{}/{} {}\n    {}",
                    pkg.source, pkg.name, pkg.version, pkg.description
                );
            }
        }
    }

    /// Print search results in plain format
    fn print_search_plain(&self, results: &[SearchResult]) {
        for result in results {
            let pkg = &result.package;
            println!("{} {} - {}", pkg.name, pkg.version, pkg.description);
        }
    }

    /// Print as JSON
    fn print_json<T: Serialize + ?Sized>(&self, data: &T) {
        if let Ok(json) = serde_json::to_string_pretty(data) {
            println!("{}", json);
        }
    }

    /// Print a table
    pub fn print_table(&self, headers: Vec<&str>, rows: Vec<Vec<String>>) {
        match self.format {
            OutputFormat::Human | OutputFormat::Plain => {
                let mut table = Table::new();
                if self.format == OutputFormat::Human {
                    table.load_preset(UTF8_FULL);
                } else {
                    table.load_preset(ASCII_BORDERS_ONLY_CONDENSED);
                }
                table.set_header(headers);
                for row in rows {
                    table.add_row(row);
                }
                println!("{}", table);
            }
            OutputFormat::Json => {
                // Convert to JSON array of objects
                let objects: Vec<_> = rows
                    .iter()
                    .map(|row| {
                        headers
                            .iter()
                            .zip(row.iter())
                            .map(|(h, v)| (h.to_string(), v.clone()))
                            .collect::<std::collections::HashMap<_, _>>()
                    })
                    .collect();
                self.print_json(&objects);
            }
        }
    }

    /// Print an error message
    pub fn error(&self, message: &str) {
        if self.colors {
            eprintln!("{} {}", style("error:").red().bold(), message);
        } else {
            eprintln!("error: {}", message);
        }
    }

    /// Print a warning message
    pub fn warn(&self, message: &str) {
        if self.colors {
            eprintln!("{} {}", style("warning:").yellow().bold(), message);
        } else {
            eprintln!("warning: {}", message);
        }
    }

    /// Print an info message
    pub fn info(&self, message: &str) {
        if self.colors {
            println!("{} {}", style("::").bold().cyan(), message);
        } else {
            println!(":: {}", message);
        }
    }

    /// Print a success message
    pub fn success(&self, message: &str) {
        if self.colors {
            println!("{} {}", style("✓").green().bold(), message);
        } else {
            println!("+ {}", message);
        }
    }

    /// Print installed packages list
    pub fn print_installed(&self, packages: &[String]) {
        match self.format {
            OutputFormat::Human => {
                println!("{}", style(":: installed packages:").bold());
                for pkg in packages {
                    println!("   {}", pkg);
                }
            }
            OutputFormat::Json => self.print_json(packages),
            OutputFormat::Plain => {
                for pkg in packages {
                    println!("{}", pkg);
                }
            }
        }
    }
}

impl Default for Output {
    fn default() -> Self {
        Self::new(OutputFormat::Human)
    }
}

/// Helper function to print styled messages
pub fn print_header(msg: &str) {
    println!("{}", style(format!(":: {}", msg)).bold());
}

pub fn print_error(msg: &str) {
    eprintln!("{} {}", style("error:").red().bold(), msg);
}

pub fn print_warning(msg: &str) {
    println!("{} {}", style("!").yellow().bold(), msg);
}

pub fn print_success(msg: &str) {
    println!("{}", style(format!("✓ {}", msg)).green());
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::types::PackageSource;

    #[test]
    fn test_output_formats() {
        let output = Output::new(OutputFormat::Plain).no_colors(true);
        // Just ensure it doesn't panic
        let packages = vec![
            Package::new("test", "1.0", "A test package"),
        ];
        output.print_packages(&packages);
    }
}
