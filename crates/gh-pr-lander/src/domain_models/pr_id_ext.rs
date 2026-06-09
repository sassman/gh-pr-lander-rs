//! PrId conversion helpers for domain models

use gh_pr_fix_with_claude::PrId;

use super::Pr;

impl From<&Pr> for PrId {
    /// Construct PrId from a PR's stable html_url
    fn from(pr: &Pr) -> Self {
        PrId::from_url(&pr.html_url)
    }
}
