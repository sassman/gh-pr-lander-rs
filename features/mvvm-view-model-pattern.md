# MVVM (Model-View-ViewModel) Pattern for TUI Views

## Problem Statement

Our current view layer has several architectural issues that make it hard to maintain and test:

### 1. Views Know Too Much About Model Structure

**Current state** (`views/build_log.rs:152-259`):
```rust
fn build_tree_row(panel: &LogPanel, path: &[usize], theme: &Theme) -> Line {
    match path.len() {
        2 => {
            // View navigates complex nested structure
            let workflow = &panel.workflows[path[0]];
            let job = &workflow.jobs[path[1]];

            // View calls business logic
            let expanded = panel.is_expanded(path);

            // View computes display formatting
            let key = format!("{}:{}", workflow.name, job.name);
            let duration_info = if let Some(metadata) = panel.job_metadata.get(&key) {
                if let Some(duration) = metadata.duration {
                    let secs = duration.as_secs();
                    if secs >= 60 {
                        format!(", {}m {}s", secs / 60, secs % 60)
                    } else {
                        format!(", {}s", secs)
                    }
                }
                // ...
            };
        }
    }
}
```

**Problems:**
- ❌ View directly accesses nested model structure (`panel.workflows[path[0]].jobs[path[1]]`)
- ❌ View calls business logic methods (`panel.is_expanded()`)
- ❌ View computes display formatting (duration, error counts, icons)
- ❌ View makes presentation decisions (which icon to show, how to format)
- ❌ Hard to test formatting logic without rendering infrastructure
- ❌ Changes to model structure require view updates

### 2. Model Construction in View Layer

**Current state** (`views/build_log.rs:421-520`):
```rust
pub fn create_log_panel_from_jobs(
    jobs: Vec<(JobMetadata, JobLog)>,
    pr_context: PrContext,
) -> LogPanel {
    // This builds the MODEL, not the view!
    // Should be in log.rs, not views/build_log.rs
}
```

**Problems:**
- ❌ Model construction logic in view module
- ❌ Violates separation of concerns
- ❌ View module imports model dependencies

### 3. Mixed Responsibilities

Views currently do THREE things:
1. **Data transformation** (formatting durations, computing icons)
2. **Business decisions** (is this an error? show icon?)
3. **Rendering** (create widgets, apply styles)

Should only do #3!

---

## Proposed Solution: MVVM Pattern

Introduce a **View Model** layer between Redux State and Views:

```
┌─────────────────────────────────────────────────────────────┐
│                    Redux Layer                               │
│                                                              │
│  Action → Reducer → State → Effects → New State            │
│                                                              │
│  State contains:                                            │
│  - LogPanel (tree structure, navigation state)             │
│  - PrList (repos, PRs, filters)                            │
│  - UI state (selected indices, scroll positions)           │
└─────────────────────────────────────────────────────────────┘
                           │
                           ↓ Transform
┌─────────────────────────────────────────────────────────────┐
│                  View Model Layer (NEW)                      │
│                                                              │
│  Transform State → Display-Ready Data                       │
│                                                              │
│  - Flatten complex structures                               │
│  - Pre-compute display text                                 │
│  - Pre-select icons, colors, styles                         │
│  - Format numbers, durations, dates                         │
│  - Apply theme colors                                       │
│                                                              │
│  View Models:                                               │
│  - LogPanelViewModel (flattened rows, ready to render)     │
│  - PrTableViewModel (formatted PR data with styles)        │
│  - StatusBarViewModel (status text, icon, color)           │
└─────────────────────────────────────────────────────────────┘
                           │
                           ↓ Render
┌─────────────────────────────────────────────────────────────┐
│                      View Layer                              │
│                                                              │
│  Pure Presentation - Just Render View Model                 │
│                                                              │
│  - Iterate pre-computed data                                │
│  - Create ratatui widgets                                   │
│  - Apply pre-determined styles                              │
│  - NO business logic                                        │
│  - NO data transformation                                   │
│  - NO model navigation                                      │
└─────────────────────────────────────────────────────────────┘
```

---

## Architecture Details

### Layer Responsibilities

#### Model Layer (`log.rs`, `state.rs`, etc.)
- **What**: Business entities, state management, navigation logic
- **Contains**:
  - `LogPanel`, `Pr`, `Repo` structs
  - Navigation methods (`navigate_up()`, `find_next_error()`)
  - State queries (`is_expanded()`, `flatten_visible_nodes()`)
- **Does NOT**: Formatting, display decisions, theme application

#### View Model Layer (`view_models/` - NEW)
- **What**: Transform State → Display-Ready Data
- **Contains**:
  - Display-specific structs (`LogPanelViewModel`, `TreeRowViewModel`)
  - Formatting functions (`format_duration()`, `select_icon()`)
  - Pre-computation logic (flatten lists, compute styles)
- **Does NOT**: Rendering, widget creation, state mutation

#### View Layer (`views/`)
- **What**: Pure presentation - render view models
- **Contains**:
  - Ratatui widget creation
  - Layout logic
  - Event-free rendering code
- **Does NOT**: Business logic, data transformation, model access

---

## Example: Build Log Refactoring

### Current Flow (Problematic)

```
State (LogPanel)
    └→ View renders directly from state
          ├─ Navigates nested structure
          ├─ Calls business methods
          ├─ Computes formatting
          └─ Creates widgets
```

### New Flow (MVVM)

```
State (LogPanel)
    └→ View Model transforms state
          ├─ Flattens tree to list of rows
          ├─ Pre-computes all display text
          ├─ Pre-selects icons, colors
          └─ Creates TreeRowViewModel[]
                └→ View renders view model
                      ├─ Iterates simple list
                      ├─ No business decisions
                      └─ Pure widget creation
```

### View Model Design

**File: `view_models/log_panel.rs`** (NEW)

```rust
/// Display-ready view model for the log panel
#[derive(Debug, Clone)]
pub struct LogPanelViewModel {
    /// PR header information (already formatted)
    pub pr_header: PrHeaderViewModel,

    /// Flattened list of visible tree rows, ready to render
    pub rows: Vec<TreeRowViewModel>,

    /// Scroll state
    pub scroll_offset: usize,
    pub viewport_height: usize,
}

#[derive(Debug, Clone)]
pub struct PrHeaderViewModel {
    pub number_text: String,     // "#123"
    pub title: String,            // "Fix: broken tests"
    pub author_text: String,      // "by sassman"
    pub number_color: Color,      // theme.status_info
    pub title_color: Color,       // theme.text_primary
    pub author_color: Color,      // theme.text_muted
}

#[derive(Debug, Clone)]
pub struct TreeRowViewModel {
    /// Complete display text (already formatted with indent, icon, status)
    pub text: String,

    /// Indentation level (for manual indent if needed)
    pub indent_level: usize,

    /// Whether this row is under cursor
    pub is_cursor: bool,

    /// Pre-determined style
    pub style: RowStyle,

    /// Additional metadata for interactions (not displayed)
    pub path: Vec<usize>,  // For handling click events
    pub node_type: NodeType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RowStyle {
    Normal,
    Error,      // Red text for errors
    Warning,    // Yellow for warnings
    Success,    // Green for success
    Selected,   // Highlighted background
    Muted,      // Gray for disabled/skipped
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeType {
    Workflow,
    Job,
    Step,
    LogLine,
}

impl LogPanelViewModel {
    /// Transform LogPanel state into display-ready view model
    pub fn from_log_panel(
        panel: &LogPanel,
        theme: &Theme,
    ) -> Self {
        let pr_header = PrHeaderViewModel {
            number_text: format!("#{}", panel.pr_context.number),
            title: panel.pr_context.title.clone(),
            author_text: format!("by {}", panel.pr_context.author),
            number_color: theme.status_info,
            title_color: theme.text_primary,
            author_color: theme.text_muted,
        };

        let visible_paths = panel.flatten_visible_nodes();
        let mut rows = Vec::new();

        for (display_idx, path) in visible_paths.iter().enumerate() {
            // Skip rows outside viewport (optimization)
            if display_idx < panel.scroll_offset {
                continue;
            }
            if display_idx >= panel.scroll_offset + panel.viewport_height {
                break;
            }

            let row = Self::build_row_view_model(panel, path, theme);
            rows.push(row);
        }

        Self {
            pr_header,
            rows,
            scroll_offset: panel.scroll_offset,
            viewport_height: panel.viewport_height,
        }
    }

    fn build_row_view_model(
        panel: &LogPanel,
        path: &[usize],
        theme: &Theme,
    ) -> TreeRowViewModel {
        let indent_level = path.len().saturating_sub(1);
        let indent = "  ".repeat(indent_level);

        match path.len() {
            1 => {
                // Workflow node
                let workflow = &panel.workflows[path[0]];
                let expanded = panel.is_expanded(path);

                let icon = if workflow.jobs.is_empty() {
                    " "
                } else if expanded {
                    "▼"
                } else {
                    "▶"
                };

                let status_icon = if workflow.has_failures { "✗" } else { "✓" };

                let error_info = if workflow.total_errors > 0 {
                    format!(" ({} errors)", workflow.total_errors)
                } else {
                    String::new()
                };

                let text = format!(
                    "{}{} {} {}{}",
                    indent, icon, status_icon, workflow.name, error_info
                );

                TreeRowViewModel {
                    text,
                    indent_level,
                    is_cursor: path == &panel.cursor_path,
                    style: if workflow.has_failures {
                        RowStyle::Error
                    } else {
                        RowStyle::Success
                    },
                    path: path.to_vec(),
                    node_type: NodeType::Workflow,
                }
            }

            2 => {
                // Job node
                let workflow = &panel.workflows[path[0]];
                let job = &workflow.jobs[path[1]];
                let expanded = panel.is_expanded(path);

                let icon = if job.steps.is_empty() {
                    " "
                } else if expanded {
                    "▼"
                } else {
                    "▶"
                };

                let status_icon = if job.error_count > 0 { "✗" } else { "✓" };

                let error_info = if job.error_count > 0 {
                    format!(" ({} errors)", job.error_count)
                } else {
                    String::new()
                };

                // Format duration HERE (view model responsibility)
                let duration_info = Self::format_job_duration(
                    &panel.job_metadata,
                    workflow,
                    job,
                );

                let text = format!(
                    "{}├─ {} {} {}{}{}",
                    indent, icon, status_icon, job.name, error_info, duration_info
                );

                TreeRowViewModel {
                    text,
                    indent_level,
                    is_cursor: path == &panel.cursor_path,
                    style: if job.error_count > 0 {
                        RowStyle::Error
                    } else {
                        RowStyle::Success
                    },
                    path: path.to_vec(),
                    node_type: NodeType::Job,
                }
            }

            // ... similar for Step and LogLine ...

            _ => TreeRowViewModel {
                text: String::new(),
                indent_level: 0,
                is_cursor: false,
                style: RowStyle::Normal,
                path: path.to_vec(),
                node_type: NodeType::LogLine,
            },
        }
    }

    /// Format job duration for display
    /// This is view model responsibility - preparing display strings
    fn format_job_duration(
        metadata: &std::collections::HashMap<String, JobMetadata>,
        workflow: &gh_actions_log_parser::WorkflowNode,
        job: &gh_actions_log_parser::JobNode,
    ) -> String {
        let key = format!("{}:{}", workflow.name, job.name);

        if let Some(meta) = metadata.get(&key) {
            if let Some(duration) = meta.duration {
                let secs = duration.as_secs();
                return if secs >= 60 {
                    format!(", {}m {}s", secs / 60, secs % 60)
                } else {
                    format!(", {}s", secs)
                };
            }
        }

        String::new()
    }
}
```

### Simplified View

**File: `views/build_log.rs`** (REFACTORED)

```rust
use crate::view_models::log_panel::LogPanelViewModel;
use ratatui::{prelude::*, widgets::*};

/// Render log panel from view model
/// View is now PURELY presentational - no business logic!
pub fn render_log_panel_card(
    f: &mut Frame,
    view_model: &LogPanelViewModel,  // ← View model, not model!
    theme: &crate::theme::Theme,
    available_area: Rect,
) -> usize {
    f.render_widget(Clear, available_area);

    let background = Block::default().style(Style::default().bg(theme.bg_panel));
    f.render_widget(background, available_area);

    let card_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // PR header
            Constraint::Min(0),    // Log content
        ])
        .split(available_area);

    // Render PR header (simple - data is pre-formatted)
    render_pr_header(f, &view_model.pr_header, theme, card_chunks[0]);

    // Render log tree (simple - just iterate rows)
    render_log_tree(f, view_model, theme, card_chunks[1])
}

fn render_pr_header(
    f: &mut Frame,
    header: &PrHeaderViewModel,
    theme: &Theme,
    area: Rect,
) {
    let text = vec![
        Line::from(vec![
            Span::styled(
                header.number_text.clone(),
                Style::default()
                    .fg(header.number_color)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" "),
            Span::styled(
                header.title.clone(),
                Style::default()
                    .fg(header.title_color)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(Span::styled(
            header.author_text.clone(),
            Style::default().fg(header.author_color),
        )),
    ];

    let paragraph = Paragraph::new(text).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(
                Style::default()
                    .fg(theme.accent_primary)
                    .add_modifier(Modifier::BOLD),
            )
            .style(Style::default().bg(theme.bg_panel)),
    );

    f.render_widget(paragraph, area);
}

fn render_log_tree(
    f: &mut Frame,
    view_model: &LogPanelViewModel,
    theme: &Theme,
    area: Rect,
) -> usize {
    let visible_height = area.height.saturating_sub(2) as usize;

    // Simple iteration - no complex logic!
    let mut rows = Vec::new();
    for row_vm in &view_model.rows {
        let style = match row_vm.style {
            RowStyle::Normal => Style::default()
                .fg(theme.text_primary)
                .bg(theme.bg_panel),
            RowStyle::Error => Style::default()
                .fg(theme.status_error)
                .bg(theme.bg_panel),
            RowStyle::Success => Style::default()
                .fg(theme.text_primary)
                .bg(theme.bg_panel),
            RowStyle::Selected => Style::default()
                .fg(theme.text_primary)
                .bg(theme.selected_bg),
            RowStyle::Warning => Style::default()
                .fg(theme.status_warning)
                .bg(theme.bg_panel),
            RowStyle::Muted => Style::default()
                .fg(theme.text_muted)
                .bg(theme.bg_panel),
        };

        // Text is pre-formatted, just create row
        rows.push(Row::new(vec![Cell::from(row_vm.text.clone())]).style(style));
    }

    let table = Table::new(rows, vec![Constraint::Percentage(100)])
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Build Logs | j/k: navigate, Enter: toggle, n: next error, x: close ")
                .border_style(Style::default().fg(theme.accent_primary))
                .style(Style::default().bg(theme.bg_panel)),
        )
        .style(Style::default().bg(theme.bg_panel));

    f.render_widget(table, area);
    visible_height
}
```

**Notice how simple the view became:**
- ✅ No model navigation (`panel.workflows[path[0]]...`)
- ✅ No business logic calls (`panel.is_expanded()`)
- ✅ No formatting decisions (duration, icons)
- ✅ Just iterates pre-computed rows
- ✅ Applies pre-determined styles
- ✅ Creates widgets from ready-to-display data

---

## Integration with Redux

### Where to Create View Model?

**Option A: Lazy Creation in Render Loop** (Recommended for Start)

```rust
// main.rs
fn ui(f: &mut Frame, app: &App) {
    // ... layout ...

    if let Some(ref panel) = app.store.state().log_panel.panel {
        // Create view model on-the-fly
        let view_model = LogPanelViewModel::from_log_panel(
            panel,
            &app.store.state().theme,
        );

        let viewport_height = views::build_log::render_log_panel_card(
            f,
            &view_model,
            &app.store.state().theme,
            chunks[1],
        );

        app.store.dispatch(Action::UpdateLogPanelViewport(viewport_height));
    }
}
```

**Pros:**
- ✅ Simple to implement
- ✅ No state changes needed
- ✅ View model always in sync with state
- ✅ No caching complexity

**Cons:**
- ❌ Recomputes on every frame (but fast - just iteration)
- ❌ No performance benefit from caching

**Option B: Cached in State** (Future Optimization)

```rust
// state.rs
pub struct LogPanelState {
    pub panel: Option<LogPanel>,
    pub view_model: Option<LogPanelViewModel>,  // Cached
}

// Recompute only when panel changes
impl LogPanelState {
    pub fn get_or_create_view_model(&mut self, theme: &Theme) -> Option<&LogPanelViewModel> {
        if let Some(ref panel) = self.panel {
            // Only recompute if missing or stale
            if self.view_model.is_none() {
                self.view_model = Some(LogPanelViewModel::from_log_panel(panel, theme));
            }
            self.view_model.as_ref()
        } else {
            None
        }
    }
}

// Invalidate in reducer when panel changes
fn log_panel_reducer(mut state: LogPanelState, action: &Action) -> (LogPanelState, Vec<Effect>) {
    match action {
        Action::BuildLogsLoaded(jobs, pr_context) => {
            state.panel = Some(create_log_panel_from_jobs(jobs, pr_context));
            state.view_model = None;  // Invalidate - will recompute on next render
        }
        Action::ScrollLogPanelDown => {
            // Scroll doesn't invalidate view model (just offset)
            if let Some(ref mut panel) = state.panel {
                panel.scroll_offset += 1;
            }
            // view_model stays cached
        }
        // ...
    }
}
```

**Pros:**
- ✅ Better performance (compute once, render many times)
- ✅ Only recomputes when data changes

**Cons:**
- ❌ More complex state management
- ❌ Need invalidation logic in reducers
- ❌ Risk of stale view model if invalidation missed

**Recommendation**: Start with **Option A**, optimize to **Option B** if profiling shows performance issues.

---

## Benefits

### 1. Separation of Concerns

**Before:**
```rust
// View does everything
fn build_tree_row(panel: &LogPanel, ...) {
    let job = &panel.workflows[path[0]].jobs[path[1]];  // Navigate
    let expanded = panel.is_expanded(path);              // Business logic
    let duration = format_duration(...);                 // Format
    Line::from(format!("{}├─ {} {}", ...))              // Render
}
```

**After:**
```rust
// View Model: transforms data
impl LogPanelViewModel {
    fn build_row_view_model(panel: &LogPanel, ...) -> TreeRowViewModel {
        // All transformation happens HERE
        TreeRowViewModel { text: "...", style: Error, ... }
    }
}

// View: just renders
fn render_log_tree(view_model: &LogPanelViewModel, ...) {
    for row in &view_model.rows {
        rows.push(Row::new(vec![Cell::from(row.text)]));  // Just display
    }
}
```

### 2. Testability

**Before** (hard to test):
```rust
// Can't test formatting without Frame, rendering infrastructure
#[test]
fn test_duration_format() {
    // Need to create Frame, Area, render, inspect widgets...
    // Very difficult!
}
```

**After** (easy to test):
```rust
#[test]
fn test_duration_formatting() {
    let panel = create_test_panel();
    let theme = Theme::default();

    let vm = LogPanelViewModel::from_log_panel(&panel, &theme);

    // Direct assertion on display text
    assert_eq!(vm.rows[1].text.contains("1m 30s"), true);
    assert_eq!(vm.rows[1].style, RowStyle::Success);
}

#[test]
fn test_error_row_styling() {
    let panel = create_panel_with_errors();
    let vm = LogPanelViewModel::from_log_panel(&panel, &Theme::default());

    assert_eq!(vm.rows[0].style, RowStyle::Error);
    assert!(vm.rows[0].text.contains("✗"));
}
```

### 3. View Simplification

**Lines of code reduction:**
- `build_tree_row()`: 250 lines → 30 lines
- No nested match statements
- No business logic calls
- No formatting computations

### 4. Performance Optimization (Future)

With cached view models:
```
┌─────────────────────────────────────────┐
│  User scrolls down                      │
│  → Only scroll_offset changes           │
│  → View model stays cached              │
│  → No recomputation needed              │
│  → Just render cached rows              │
└─────────────────────────────────────────┘
```

### 5. Easier Maintenance

Changes to display format only touch view model:
- Want to change duration format? → Edit `format_job_duration()`
- Want different icons? → Edit icon selection in view model
- Want different colors? → Edit style determination in view model
- **View rendering code unchanged**

---

## Other Views to Refactor

### High Priority (Similar Complexity)

**1. PR Table View** (`views/pull_requests.rs`)

**Current issues:**
```rust
// Directly formats PR data in view
fn render_pr_table(...) {
    for (index, pr) in prs.iter().enumerate() {
        let status_icon = if pr.build_status == BuildStatus::Success {
            "✓"
        } else {
            "✗"
        };

        let mergeable_icon = match pr.mergeable_status {
            MergeableStatus::Mergeable => "✓",
            MergeableStatus::Conflicting => "✗",
            MergeableStatus::Unknown => "?",
        };

        // ... complex formatting logic ...
    }
}
```

**Proposed:**
```rust
pub struct PrTableViewModel {
    pub rows: Vec<PrRowViewModel>,
}

pub struct PrRowViewModel {
    pub number: String,           // "#123"
    pub title: String,            // Truncated to fit
    pub author: String,           // "@sassman"
    pub status_icon: &'static str,
    pub mergeable_icon: &'static str,
    pub approval_icon: &'static str,
    pub style: RowStyle,
    pub is_selected: bool,
}
```

**2. Command Palette View** (`views/command_palette.rs`)

**Current issues:**
```rust
// Complex filtering and display logic in view
fn render_command_palette(...) {
    let filtered = app.command_palette.results.iter()
        .filter(|(item, score)| *score > 0)
        .take(10)
        .map(|(item, score)| {
            // Format display text
            let category = format!("[{}]", item.category);
            let title = item.title.clone();
            let hint = item.shortcut_hint.as_ref().map(...);
            // ...
        });
}
```

**Proposed:**
```rust
pub struct CommandPaletteViewModel {
    pub input: String,
    pub results: Vec<CommandResultViewModel>,
    pub selected_index: usize,
}

pub struct CommandResultViewModel {
    pub display_text: String,    // Pre-formatted with category, title
    pub hint_text: Option<String>,
    pub score: u16,
    pub is_selected: bool,
}
```

### Medium Priority

**3. Repository Tabs** (`views/repositories.rs`)
- Pre-format repo names with status indicators
- Pre-compute tab styles

**4. Status Bar** (`views/status_bar.rs`)
- Pre-format status messages
- Pre-select icons and colors

**5. Debug Console** (`views/debug_console.rs`)
- Pre-format log entries with timestamps
- Pre-color code log levels

### Low Priority (Already Simple)

- Splash Screen (minimal logic)
- Help Panel (static content)
- Popups (simple forms)

---

## Implementation Plan

### Phase 1: Proof of Concept (Week 1)
1. ✅ Create design doc (this document)
2. Create `view_models/` directory structure
3. Implement `LogPanelViewModel` for build log
4. Refactor `views/build_log.rs` to use view model
5. Write unit tests for view model
6. Verify no regressions

**Success criteria:**
- Build log renders identically
- Tests verify formatting logic
- Code is simpler and more readable

### Phase 2: Core Views (Week 2)
1. Implement `PrTableViewModel`
2. Refactor `views/pull_requests.rs`
3. Implement `CommandPaletteViewModel`
4. Refactor `views/command_palette.rs`
5. Update tests

### Phase 3: Remaining Views (Week 3)
1. Implement view models for remaining views
2. Refactor all views to use view models
3. Move `create_log_panel_from_jobs()` to `log.rs`
4. Comprehensive testing

### Phase 4: Optimization (Optional - Week 4)
1. Add view model caching to state
2. Implement invalidation logic in reducers
3. Performance benchmarking
4. Optimize hot paths

---

## Trade-offs

### Pros
✅ **Better separation of concerns** - Clear boundaries between layers
✅ **Improved testability** - Test formatting without rendering
✅ **Simpler views** - Pure presentation, no business logic
✅ **Easier maintenance** - Changes to display don't touch rendering
✅ **Performance potential** - Can cache view models
✅ **Type safety** - View model enforces display structure

### Cons
❌ **More code** - Additional view model layer
❌ **Memory overhead** - View models duplicate some data
❌ **Learning curve** - New pattern for contributors
❌ **Initial complexity** - Requires refactoring existing views

### Mitigation
- Start with most complex views (build_log, pr_table)
- Document pattern with examples
- Write comprehensive tests
- Keep view models simple (avoid over-engineering)

---

## Alternative Approaches Considered

### 1. Keep Current Approach
**Pros:** No changes needed
**Cons:** Views remain complex, hard to test, tightly coupled to model
**Verdict:** ❌ Technical debt will grow

### 2. Pure Functions in Views
```rust
fn format_duration(duration: Duration) -> String { ... }
fn select_icon(expanded: bool) -> &'static str { ... }
```
**Pros:** Simpler than full view model
**Cons:** Views still navigate model structure, call business logic
**Verdict:** ⚠️ Partial improvement, doesn't solve core issues

### 3. Smart Components (React-style)
Container components that own logic, presentational components that render.
**Pros:** Familiar pattern from web development
**Cons:** Doesn't fit well with Rust ownership model, Redux already handles state
**Verdict:** ❌ Over-engineered for TUI

### 4. Presenter Pattern (MVP)
Similar to MVVM but Presenter owns logic instead of view model being data.
**Pros:** Clear separation
**Cons:** More complex than MVVM, Presenter needs mutable access to view
**Verdict:** ⚠️ MVVM is better fit for Rust + Redux

**Selected:** MVVM with view models - best fit for our architecture

---

## Success Metrics

After full implementation:

1. **Code Quality**
   - ✅ Views have zero business logic calls
   - ✅ Views navigate no model structures
   - ✅ All formatting logic has unit tests

2. **Maintainability**
   - ✅ Can change display format without touching views
   - ✅ Can change model structure without breaking views
   - ✅ New views follow clear pattern

3. **Testability**
   - ✅ 100% test coverage for view models
   - ✅ Can test formatting without rendering infrastructure
   - ✅ Faster test execution (no rendering overhead)

4. **Performance** (Optional with caching)
   - ✅ View model creation < 1ms
   - ✅ Cached view models reduce recomputation
   - ✅ Smooth scrolling with large log files

---

## References

### MVVM Pattern
- [Martin Fowler - Presentation Model](https://martinfowler.com/eaaDev/PresentationModel.html)
- [MVVM Pattern Explanation](https://en.wikipedia.org/wiki/Model%E2%80%93view%E2%80%93viewmodel)

### Redux Architecture
- [Redux Style Guide](https://redux.js.org/style-guide/)
- Our implementation: `architecture/ARCHITECTURE_ANALYSIS.md`

### Similar Patterns in Rust
- [Yew Framework - Components](https://yew.rs/docs/concepts/components)
- [Dioxus - Props](https://dioxuslabs.com/learn/0.5/guide/props)

---

## Open Questions

1. **View model lifecycle**: Create per-frame or cache in state?
   - **Decision**: Start with per-frame, optimize later if needed

2. **Theme handling**: Pass theme to view model or embed colors?
   - **Decision**: Pass theme, embed colors in view model

3. **Partial updates**: Update single row or rebuild entire view model?
   - **Decision**: Rebuild entire view model (simple, fast enough)

4. **Error handling**: How to handle view model creation failures?
   - **Decision**: View models should be infallible (panic on bugs, not errors)

---

## Next Steps

1. Review this design doc with team
2. Get feedback on approach
3. Start Phase 1 implementation
4. Iterate based on learnings

**Goal**: Cleaner, more maintainable, testable view layer that scales with app complexity.
