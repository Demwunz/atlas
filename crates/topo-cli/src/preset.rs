use clap::ValueEnum;

/// Scoring presets that configure index depth and signal selection.
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum Preset {
    /// Shallow index, heuristic-only scoring (fastest)
    Fast,
    /// Deep index (cached), hybrid BM25F + heuristic scoring
    Balanced,
    /// Deep index (fresh rebuild), hybrid + structural signals
    Deep,
    /// Deep index + reranking, all signals including embeddings
    Thorough,
}

impl Preset {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Fast => "fast",
            Self::Balanced => "balanced",
            Self::Deep => "deep",
            Self::Thorough => "thorough",
        }
    }

    /// Whether this preset needs a deep index.
    pub fn needs_deep_index(&self) -> bool {
        matches!(self, Self::Balanced | Self::Deep | Self::Thorough)
    }

    /// Whether this preset should force-rebuild the index.
    pub fn force_rebuild(&self) -> bool {
        matches!(self, Self::Deep | Self::Thorough)
    }

    /// Whether to include structural signals (PageRank, git recency).
    pub fn use_structural_signals(&self) -> bool {
        matches!(self, Self::Deep | Self::Thorough)
    }

    /// Default max bytes budget for this preset.
    pub fn default_max_bytes(&self) -> u64 {
        match self {
            Self::Fast => 50_000,
            Self::Balanced => 100_000,
            Self::Deep => 200_000,
            Self::Thorough => 500_000,
        }
    }

    /// Default minimum score threshold.
    pub fn default_min_score(&self) -> f64 {
        match self {
            Self::Fast => 0.05,
            Self::Balanced => 0.01,
            Self::Deep => 0.005,
            Self::Thorough => 0.001,
        }
    }
}

impl std::fmt::Display for Preset {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preset_as_str() {
        assert_eq!(Preset::Fast.as_str(), "fast");
        assert_eq!(Preset::Balanced.as_str(), "balanced");
        assert_eq!(Preset::Deep.as_str(), "deep");
        assert_eq!(Preset::Thorough.as_str(), "thorough");
    }

    #[test]
    fn preset_needs_deep_index() {
        assert!(!Preset::Fast.needs_deep_index());
        assert!(Preset::Balanced.needs_deep_index());
        assert!(Preset::Deep.needs_deep_index());
        assert!(Preset::Thorough.needs_deep_index());
    }

    #[test]
    fn preset_force_rebuild() {
        assert!(!Preset::Fast.force_rebuild());
        assert!(!Preset::Balanced.force_rebuild());
        assert!(Preset::Deep.force_rebuild());
        assert!(Preset::Thorough.force_rebuild());
    }

    #[test]
    fn preset_structural_signals() {
        assert!(!Preset::Fast.use_structural_signals());
        assert!(!Preset::Balanced.use_structural_signals());
        assert!(Preset::Deep.use_structural_signals());
        assert!(Preset::Thorough.use_structural_signals());
    }

    #[test]
    fn preset_budgets_increase() {
        assert!(Preset::Fast.default_max_bytes() < Preset::Balanced.default_max_bytes());
        assert!(Preset::Balanced.default_max_bytes() < Preset::Deep.default_max_bytes());
        assert!(Preset::Deep.default_max_bytes() < Preset::Thorough.default_max_bytes());
    }
}
