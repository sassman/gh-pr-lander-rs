//! Actions for Repository management.
//!
//! Includes both repository operations and the add repository form.

use crate::domain_models::Repository;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RepositoryAction {
    // === Repository operations ===
    /// Open the current repository in the browser
    OpenRepositoryInBrowser,

    /// Adds a new repository to the list
    AddRepository(Repository),

    /// Load all repository related data (e.g., pull requests etc.)
    LoadRepositoryData(Repository),

    // === Add Repository Form actions ===
    /// Move to next field (Tab)
    FormNextField,
    /// Move to previous field (Shift+Tab)
    FormPrevField,

    /// Character typed into current field
    FormChar(char),
    /// Backspace pressed in current field
    FormBackspace,
    /// Clear entire current field
    FormClearField,

    /// Confirm and add the repository (Enter)
    FormConfirm,
    /// Close the form without adding (Esc)
    FormClose,
}
