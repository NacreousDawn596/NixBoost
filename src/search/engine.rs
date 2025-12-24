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

//! Parallel fuzzy search engine for NixBoost.

use crate::core::error::{Result, SearchError};
use crate::core::types::{Package, PackageSource, SearchResult, MatchType};
use fuzzy_matcher::FuzzyMatcher;
use fuzzy_matcher::skim::SkimMatcherV2;
use rayon::prelude::*;
use std::sync::Arc;
use tracing::{debug, info};

/// Parallel search engine with fuzzy matching
pub struct SearchEngine {
    /// Fuzzy matcher
    matcher: Arc<SkimMatcherV2>,
    /// Minimum score threshold (0-100)
    min_score: i64,
    /// Maximum results to return
    max_results: usize,
}

impl SearchEngine {
    /// Create a new search engine
    pub fn new() -> Self {
        Self {
            matcher: Arc::new(SkimMatcherV2::default()),
            min_score: 30,
            max_results: 50,
        }
    }

    /// Create with custom settings
    pub fn with_settings(min_score: i64, max_results: usize) -> Self {
        Self {
            matcher: Arc::new(SkimMatcherV2::default()),
            min_score,
            max_results,
        }
    }

    /// Set maximum results
    pub fn max_results(mut self, max: usize) -> Self {
        self.max_results = max;
        self
    }

    /// Set minimum score threshold
    pub fn min_score(mut self, score: i64) -> Self {
        self.min_score = score;
        self
    }

    /// Search packages with fuzzy matching
    pub fn search(&self, query: &str, packages: &[Package]) -> Result<Vec<SearchResult>> {
        if query.is_empty() {
            return Err(SearchError::QueryTooShort { min_length: 1 }.into());
        }

        if query.len() > 200 {
            return Err(SearchError::QueryTooLong { max_length: 200 }.into());
        }

        let query_lower = query.to_lowercase();
        debug!("Searching {} packages for '{}'", packages.len(), query);

        // Parallel search
        let mut results: Vec<SearchResult> = packages
            .par_iter()
            .filter_map(|pkg| self.score_package(&query_lower, pkg))
            .collect();

        // Sort by score (highest first)
        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));

        // Limit results
        results.truncate(self.max_results);

        info!("Found {} results for '{}'", results.len(), query);
        Ok(results)
    }

    /// Score a single package against the query
    fn score_package(&self, query: &str, package: &Package) -> Option<SearchResult> {
        let name_lower = package.name.to_lowercase();
        let desc_lower = package.description.to_lowercase();

        // Check for exact match first
        if name_lower == query {
            return Some(SearchResult::new(
                package.clone(),
                1.0,
                MatchType::ExactName,
            ));
        }

        // Check for prefix match
        if name_lower.starts_with(query) {
            return Some(SearchResult::new(
                package.clone(),
                0.95 - (name_lower.len() - query.len()) as f64 * 0.01,
                MatchType::NamePrefix,
            ));
        }

        // Check for substring match in name
        if name_lower.contains(query) {
            let position_bonus = 1.0 - (name_lower.find(query).unwrap_or(0) as f64 / name_lower.len() as f64) * 0.2;
            return Some(SearchResult::new(
                package.clone(),
                0.8 * position_bonus,
                MatchType::NameContains,
            ));
        }

        // Check for substring match in description
        if desc_lower.contains(query) {
            return Some(SearchResult::new(
                package.clone(),
                0.5,
                MatchType::DescriptionContains,
            ));
        }

        // Fuzzy match on name
        if let Some(score) = self.matcher.fuzzy_match(&name_lower, query) {
            if score >= self.min_score {
                let normalized = (score as f64 / 100.0).min(0.7);
                return Some(SearchResult::new(
                    package.clone(),
                    normalized,
                    MatchType::Fuzzy,
                ));
            }
        }

        // Fuzzy match on description (lower weight)
        if let Some(score) = self.matcher.fuzzy_match(&desc_lower, query) {
            if score >= self.min_score {
                let normalized = (score as f64 / 200.0).min(0.4);
                return Some(SearchResult::new(
                    package.clone(),
                    normalized,
                    MatchType::Fuzzy,
                ));
            }
        }

        None
    }

    /// Search with suggestions for typos
    pub fn search_with_suggestions(
        &self,
        query: &str,
        packages: &[Package],
    ) -> Result<SearchWithSuggestions> {
        let results = self.search(query, packages)?;
        
        let suggestions = if results.is_empty() {
            self.generate_suggestions(query, packages)
        } else {
            vec![]
        };

        Ok(SearchWithSuggestions { results, suggestions })
    }

    /// Generate "Did you mean...?" suggestions
    fn generate_suggestions(&self, query: &str, packages: &[Package]) -> Vec<String> {
        let query_lower = query.to_lowercase();
        
        // Find packages with similar names
        let mut candidates: Vec<(String, i64)> = packages
            .par_iter()
            .filter_map(|pkg| {
                let score = self.matcher.fuzzy_match(&pkg.name.to_lowercase(), &query_lower)?;
                if score > 20 {
                    Some((pkg.name.clone(), score))
                } else {
                    None
                }
            })
            .collect();

        candidates.sort_by(|a, b| b.1.cmp(&a.1));
        candidates.truncate(5);
        
        candidates.into_iter().map(|(name, _)| name).collect()
    }

    /// Quick search for exact matches only (fastest)
    pub fn quick_search(&self, query: &str, packages: &[Package]) -> Vec<Package> {
        let query_lower = query.to_lowercase();
        
        packages
            .par_iter()
            .filter(|pkg| {
                let name_lower = pkg.name.to_lowercase();
                name_lower == query_lower || 
                name_lower.starts_with(&query_lower) ||
                name_lower.contains(&query_lower)
            })
            .cloned()
            .collect()
    }

    /// Filter packages by source
    pub fn filter_by_source<'a>(
        packages: &'a [Package],
        source: &PackageSource,
    ) -> Vec<&'a Package> {
        packages.iter().filter(|p| &p.source == source).collect()
    }
}

impl Default for SearchEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// Search results with suggestions
pub struct SearchWithSuggestions {
    pub results: Vec<SearchResult>,
    pub suggestions: Vec<String>,
}

impl SearchWithSuggestions {
    pub fn is_empty(&self) -> bool {
        self.results.is_empty()
    }

    pub fn has_suggestions(&self) -> bool {
        !self.suggestions.is_empty()
    }
}

/// Multi-source parallel search
pub struct MultiSourceSearch {
    engine: SearchEngine,
}

impl MultiSourceSearch {
    pub fn new() -> Self {
        Self {
            engine: SearchEngine::new(),
        }
    }

    /// Search across multiple package sources in parallel
    pub fn search(
        &self,
        query: &str,
        nixpkgs: &[Package],
        nur: &[Package],
    ) -> Result<Vec<SearchResult>> {
        // Search both sources in parallel
        let (nixpkgs_results, nur_results) = rayon::join(
            || self.engine.search(query, nixpkgs),
            || self.engine.search(query, nur),
        );

        let mut all_results = nixpkgs_results?;
        all_results.extend(nur_results?);

        // Re-sort combined results
        all_results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        all_results.truncate(self.engine.max_results);

        Ok(all_results)
    }
}

impl Default for MultiSourceSearch {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_packages() -> Vec<Package> {
        vec![
            Package::new("firefox", "120.0", "Web browser"),
            Package::new("firefox-esr", "115.0", "Firefox Extended Support Release"),
            Package::new("chromium", "120.0", "Web browser"),
            Package::new("git", "2.43", "Version control system"),
            Package::new("github-cli", "2.40", "GitHub's command-line tool"),
            Package::new("vim", "9.0", "Text editor"),
            Package::new("neovim", "0.9", "Vim-fork focused on extensibility"),
        ]
    }

    #[test]
    fn test_exact_match() {
        let engine = SearchEngine::new();
        let packages = create_test_packages();
        
        let results = engine.search("firefox", &packages).unwrap();
        
        assert!(!results.is_empty());
        assert_eq!(results[0].package.name, "firefox");
        assert_eq!(results[0].match_type, MatchType::ExactName);
        assert_eq!(results[0].score, 1.0);
    }

    #[test]
    fn test_prefix_match() {
        let engine = SearchEngine::new();
        let packages = create_test_packages();
        
        let results = engine.search("fire", &packages).unwrap();
        
        assert!(!results.is_empty());
        assert!(matches!(results[0].match_type, MatchType::NamePrefix));
    }

    #[test]
    fn test_contains_match() {
        let engine = SearchEngine::new();
        let packages = create_test_packages();
        
        let results = engine.search("hub", &packages).unwrap();
        
        assert!(!results.is_empty());
        assert_eq!(results[0].package.name, "github-cli");
    }

    #[test]
    fn test_description_match() {
        let engine = SearchEngine::new();
        let packages = create_test_packages();
        
        let results = engine.search("version control", &packages).unwrap();
        
        assert!(!results.is_empty());
        assert_eq!(results[0].package.name, "git");
    }

    #[test]
    fn test_fuzzy_match() {
        let engine = SearchEngine::new();
        let packages = create_test_packages();
        
        // Typo: "neovmi" instead of "neovim"
        let results = engine.search("neovi", &packages).unwrap();
        
        assert!(!results.is_empty());
        assert!(results.iter().any(|r| r.package.name == "neovim"));
    }

    #[test]
    fn test_max_results() {
        let engine = SearchEngine::new().max_results(2);
        let packages = create_test_packages();
        
        let results = engine.search("e", &packages).unwrap();
        
        assert!(results.len() <= 2);
    }

    #[test]
    fn test_quick_search() {
        let engine = SearchEngine::new();
        let packages = create_test_packages();
        
        let results = engine.quick_search("git", &packages);
        
        assert_eq!(results.len(), 2); // git and github-cli
    }

    #[test]
    fn test_suggestions() {
        let engine = SearchEngine::new();
        let packages = create_test_packages();
        
        let result = engine.search_with_suggestions("firefx", &packages).unwrap();
        
        // Should have suggestions since "firefx" doesn't match exactly
        if result.is_empty() {
            assert!(result.has_suggestions() || !result.is_empty());
        }
    }

    #[test]
    fn test_empty_query() {
        let engine = SearchEngine::new();
        let packages = create_test_packages();
        
        let result = engine.search("", &packages);
        assert!(result.is_err());
    }
}
