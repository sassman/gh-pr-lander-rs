//! Diff Viewer Reducer
//!
//! Handles state transitions for the diff viewer panel.
//! Translates DiffViewerAction to gh_diff_viewer::DiffAction and forwards to the inner state.

use crate::actions::DiffViewerAction;
use crate::state::DiffViewerState;
use gh_diff_viewer::DiffAction;

/// Reducer for diff viewer actions.
///
/// Translates gh-pr-lander's DiffViewerAction to gh-diff-viewer's DiffAction
/// and forwards to the inner state for processing.
pub fn reduce_diff_viewer(
    mut state: DiffViewerState,
    action: &DiffViewerAction,
) -> DiffViewerState {
    match action {
        // === Loading (handled at this level) ===
        DiffViewerAction::Open => {
            // Open is handled by middleware to fetch the diff
            state
        }

        DiffViewerAction::LoadStart => {
            state.set_loading();
            state
        }

        DiffViewerAction::Loaded {
            diff,
            pr_number,
            pr_title,
        } => {
            state.load(diff.clone(), *pr_number, pr_title.clone());
            state
        }

        DiffViewerAction::LoadError(error) => {
            state.set_error(error.clone());
            state
        }

        // === Navigation (forward to inner state) ===
        DiffViewerAction::NavigateDown => {
            forward_action(&mut state, DiffAction::CursorDown);
            state
        }

        DiffViewerAction::NavigateUp => {
            forward_action(&mut state, DiffAction::CursorUp);
            state
        }

        DiffViewerAction::NavigateLeft => {
            // In diff viewer context, left goes to file tree
            forward_action(&mut state, DiffAction::FocusFileTree);
            state
        }

        DiffViewerAction::NavigateRight => {
            // In diff viewer context, right goes to diff content
            forward_action(&mut state, DiffAction::FocusDiffContent);
            state
        }

        DiffViewerAction::NavigateToTop => {
            forward_action(&mut state, DiffAction::CursorFirst);
            state
        }

        DiffViewerAction::NavigateToBottom => {
            forward_action(&mut state, DiffAction::CursorLast);
            state
        }

        // === Scrolling ===
        DiffViewerAction::PageDown => {
            forward_action(&mut state, DiffAction::ScrollPageDown);
            state
        }

        DiffViewerAction::PageUp => {
            forward_action(&mut state, DiffAction::ScrollPageUp);
            state
        }

        // === Tree Operations ===
        DiffViewerAction::Toggle => {
            forward_action(&mut state, DiffAction::ToggleTreeNode);
            state
        }

        DiffViewerAction::ExpandAll | DiffViewerAction::CollapseAll => {
            // Not directly supported by inner state yet
            state
        }

        // === Focus Management ===
        DiffViewerAction::SwitchPane => {
            forward_action(&mut state, DiffAction::ToggleFocus);
            state
        }

        // === Visual Mode ===
        DiffViewerAction::EnterVisualMode => {
            forward_action(&mut state, DiffAction::EnterVisualMode);
            state
        }

        DiffViewerAction::ExitVisualMode => {
            forward_action(&mut state, DiffAction::ExitVisualMode);
            state
        }

        // === Comments ===
        DiffViewerAction::AddComment => {
            forward_action(&mut state, DiffAction::StartComment);
            state
        }

        DiffViewerAction::CancelComment => {
            forward_action(&mut state, DiffAction::CancelComment);
            state
        }

        DiffViewerAction::CommitComment => {
            forward_action(&mut state, DiffAction::CommitComment);
            state
        }

        DiffViewerAction::CommentChar(c) => {
            forward_action(&mut state, DiffAction::CommentInsertChar(*c));
            state
        }

        DiffViewerAction::CommentBackspace => {
            forward_action(&mut state, DiffAction::CommentBackspace);
            state
        }

        // === Review ===
        DiffViewerAction::ShowReviewPopup => {
            forward_action(&mut state, DiffAction::ShowReviewPopup);
            state
        }

        DiffViewerAction::HideReviewPopup => {
            forward_action(&mut state, DiffAction::HideReviewPopup);
            state
        }

        DiffViewerAction::ReviewOptionNext => {
            forward_action(&mut state, DiffAction::ReviewOptionNext);
            state
        }

        DiffViewerAction::ReviewOptionPrev => {
            forward_action(&mut state, DiffAction::ReviewOptionPrev);
            state
        }

        DiffViewerAction::SubmitReview => {
            forward_action(&mut state, DiffAction::SubmitReview);
            state
        }

        // === Events from DiffViewerState ===
        DiffViewerAction::Event(_event) => {
            // Events are handled by middleware, not by the reducer
            state
        }

        // === Viewport ===
        DiffViewerAction::SetViewport { width, height } => {
            forward_action(
                &mut state,
                DiffAction::SetViewport {
                    width: *width,
                    height: *height,
                },
            );
            state
        }
    }
}

/// Forward a DiffAction to the inner state if it exists.
fn forward_action(state: &mut DiffViewerState, action: DiffAction) {
    if let Some(ref mut inner) = state.inner {
        // Process the action and ignore events for now
        // Events will be handled by middleware when we implement that
        let _events = inner.handle_action(action);
    }
}
