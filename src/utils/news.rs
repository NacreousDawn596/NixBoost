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

//! NixOS news fetcher for NixBoost.

use anyhow::Result;
use comfy_table::{Table, presets::UTF8_FULL};
use console::style;

/// Fetch and display NixOS news
pub async fn fetch_nixos_news() -> Result<()> {
    println!("{}", style(":: fetching nixos news...").bold());

    let client = reqwest::Client::new();
    let res = client
        .get("https://nixos.org/blog/feed.xml")
        .send()
        .await?
        .text()
        .await?;

    let channel = rss::Channel::read_from(res.as_bytes())
        .map_err(|e| anyhow::anyhow!("failed to parse rss: {}", e))?;

    let mut table = Table::new();
    table.load_preset(UTF8_FULL);
    table.set_header(vec!["Date", "Title"]);

    for item in channel.items().iter().take(5) {
        let date = item.pub_date().unwrap_or("Unknown");
        let title = item.title().unwrap_or("No title");
        table.add_row(vec![date, title]);
    }

    println!("{}", table);
    Ok(())
}

#[cfg(test)]
mod tests {
    // News tests would require network access
}
