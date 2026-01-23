//! PrId conversion helpers for domain models

use gh_pr_fix_with_claude::PrId;

use super::{Pr, Repository};

impl From<&Pr> for PrId {
    /// Construct PrId from a PR's stable html_url
    fn from(pr: &Pr) -> Self {
        PrId::from_url(&pr.html_url)
    }
}

/// Construct a PrId from a Repository + PR number (for lookups without a full Pr)
pub fn pr_id_from_repo(repo: &Repository, pr_number: usize) -> PrId {
    PrId::from_parts(repo.effective_host(), &repo.org, &repo.repo, pr_number)
}
