# View Model Analysis

## Overview
Analysis of all views in the application to identify where MVVM pattern should be applied for better separation of concerns.

## Current State

### ✅ Views with Proper MVVM Implementation

#### 1. `pull_requests.rs`
- **View Model**: `PrTableViewModel` (in `view_models/pr_table.rs`)
- **Status**: ✅ Complete
- **Quality**: Excellent separation - view only renders pre-computed data
- **Location**: Lines 10-99

**What the View Model Does**:
- Pre-computes all row data (PR number, title, author, comments, status)
- Calculates colors for each row based on state
- Formats status text and colors
- Prepares header with title and status

**What the View Does**:
- Simple iteration over `vm.rows`
- Direct rendering of pre-computed text and colors
- No business logic, no data transformation

---

#### 2. `build_log.rs` (Log Panel)
- **View Model**: `LogPanelViewModel` (in `view_models/log_panel.rs`)
- **Status**: ✅ Complete
- **Quality**: Excellent separation - pure presentation
- **Location**: Lines 1-150

**What the View Model Does**:
- Builds tree structure from raw job logs
- Computes scroll offset and visible range
- Determines row styles (Normal, Error, Success, Selected)
- Formats PR header with colors
- Pre-formats all log text with proper indentation

**What the View Does**:
- Iterates over `view_model.rows[start..end]`
- Applies pre-determined styles
- No complex calculations or business logic

---

## 🚫 Views That DON'T Need View Models (Too Simple)

### 3. `status_bar.rs`
- **Complexity**: Very Low
- **Lines of Code**: 27
- **Logic**: Simple icon + color mapping based on TaskStatusType enum
- **Verdict**: ❌ No view model needed
- **Reason**: Single-line text with basic conditional styling. Adding a view model would be over-engineering.

**Current Approach (Acceptable)**:
```rust
let (icon, color) = match status.status_type {
    TaskStatusType::Running => ("⏳", theme.status_warning),
    TaskStatusType::Success => ("✓", theme.status_success),
    // ...
};
```

---

## ⚠️ Views That NEED View Models (Complex Logic in View)

### 4. `splash_screen.rs`
- **Current Issues**: ❌ Business logic mixed with rendering
- **Complexity**: Medium
- **Lines of Code**: 158
- **Priority**: Medium

**Problems Identified**:

**Lines 65-90: Stage Message Determination**
```rust
let (stage_message, progress, is_error) = match &app.store.state().infrastructure.bootstrap_state {
    BootstrapState::NotStarted => ("Initializing application...", 0, false),
    BootstrapState::LoadingRepositories => ("Loading repositories...", 25, false),
    BootstrapState::RestoringSession => ("Restoring session...", 50, false),
    BootstrapState::LoadingFirstRepo => {
        // BUSINESS LOGIC: Finding selected repo and formatting message
        if let Some(repo) = app.store.state().repos.recent_repos.get(...) {
            (&format!("Loading {}...", repo.repo)[..], 75, false)
        } else {
            ("Loading repository...", 75, false)
        }
    }
    BootstrapState::Error(err) => (&format!("Error: {}", err)[..], 0, true),
};
```

**Lines 133-137: Progress Bar Rendering Logic**
```rust
let bar_width = chunks[5].width.saturating_sub(10) as usize;
let filled = (bar_width * progress) / 100;
let empty = bar_width.saturating_sub(filled);
let progress_bar = format!("{}{}  {}%", "▰".repeat(filled), "▱".repeat(empty), progress);
```

**Lines 110: Spinner Frame Calculation**
```rust
let spinner = SPINNER_FRAMES[app.store.state().ui.spinner_frame % SPINNER_FRAMES.len()];
```

**Recommended View Model**:
```rust
pub struct SplashScreenViewModel {
    pub title: String,
    pub stage_message: String,
    pub progress_percent: usize,
    pub is_error: bool,
    pub spinner_char: String,
    pub progress_bar: String,
    pub title_color: Color,
    pub message_color: Color,
}
```

**Benefits**:
- Centralizes all bootstrap state interpretation
- Pre-formats progress bar string
- Pre-selects spinner frame
- View becomes pure rendering

---

### 5. `command_palette.rs`
- **Current Issues**: ❌ Complex layout calculations in view
- **Complexity**: High
- **Lines of Code**: 278
- **Priority**: High

**Problems Identified**:

**Lines 109-115: Scroll Offset Calculation**
```rust
let scroll_offset = if selected < visible_height / 2 {
    0
} else if selected >= total_items.saturating_sub(visible_height / 2) {
    total_items.saturating_sub(visible_height)
} else {
    selected.saturating_sub(visible_height / 2)
};
```

**Lines 159-169: Title Truncation Logic**
```rust
let category_text = format!("[{}]", cmd.category);
let fixed_width = 2 + 13 + category_text.len() + 3;
let max_title_width = available_width.saturating_sub(fixed_width);

let title_text = if cmd.title.len() > max_title_width && max_title_width > 3 {
    format!("{}...", &cmd.title[..max_title_width.saturating_sub(3)])
} else {
    cmd.title.clone()
};
```

**Lines 192-197: Padding Calculation**
```rust
let used_width = 2 + 13 + title_text.len() + category_text.len();
let padding = if available_width > used_width {
    available_width.saturating_sub(used_width)
} else {
    1
};
```

**Recommended View Model**:
```rust
pub struct CommandPaletteViewModel {
    pub input_text: String,
    pub total_commands: usize,
    pub visible_rows: Vec<CommandRow>,
    pub selected_command: Option<SelectedCommand>,
    pub scroll_offset: usize,
}

pub struct CommandRow {
    pub is_selected: bool,
    pub indicator: String,        // "> " or "  "
    pub shortcut_hint: String,    // Pre-formatted to 13 chars
    pub title: String,            // Pre-truncated if needed
    pub category: String,         // Pre-formatted with brackets
    pub padding: String,          // Pre-computed spaces
    pub fg_color: Color,
    pub bg_color: Color,
}

pub struct SelectedCommand {
    pub description: String,
    pub context: Option<String>,
}
```

**Benefits**:
- All scroll logic happens in view model computation
- Truncation logic centralized
- Padding pre-computed
- View becomes simple iteration

---

### 6. `debug_console.rs`
- **Current Issues**: ❌ Data transformation mixed with rendering
- **Complexity**: Medium-High
- **Lines of Code**: 98
- **Priority**: High

**Problems Identified**:

**Lines 33-41: Scroll Offset Calculation**
```rust
let scroll_offset = if console_state.auto_scroll {
    // Auto-scroll: show most recent logs
    total_logs.saturating_sub(visible_height)
} else {
    // Manual scroll: use scroll_offset
    console_state.scroll_offset.min(total_logs.saturating_sub(visible_height))
};
```

**Lines 44-74: Log Item Transformation**
```rust
let log_items: Vec<ListItem> = logs
    .iter()
    .skip(scroll_offset)
    .take(visible_height)
    .map(|entry| {
        // Log level color mapping
        let level_color = match entry.level {
            Level::Error => theme.status_error,
            Level::Warn => theme.status_warning,
            // ...
        };

        // Timestamp formatting
        let timestamp = entry.timestamp.format("%H:%M:%S%.3f");
        let level_str = format!("{:5}", entry.level.to_string().to_uppercase());

        // Target truncation logic
        let target_short = if entry.target.len() > 20 {
            format!("{}...", &entry.target[..17])
        } else {
            format!("{:20}", entry.target)
        };

        // Message formatting
        let text = format!("{} {} {} {}", timestamp, level_str, target_short, entry.message);
        ListItem::new(text).style(Style::default().fg(level_color))
    })
    .collect();
```

**Recommended View Model**:
```rust
pub struct DebugConsoleViewModel {
    pub title: String,              // "Debug Console (50/100) [AUTO]"
    pub footer: String,             // Keyboard shortcuts
    pub visible_logs: Vec<LogLine>,
    pub scroll_offset: usize,
    pub visible_height: usize,
}

pub struct LogLine {
    pub text: String,      // Pre-formatted: "12:34:56.789 ERROR my_target... message"
    pub color: Color,      // Pre-determined based on level
}
```

**Benefits**:
- All timestamp formatting done once in view model
- Target truncation centralized
- Color mapping in view model
- View just iterates and displays pre-formatted lines

---

### 7. `repositories.rs`
- **Current Issues**: ❌ State checks and formatting in view
- **Complexity**: Medium
- **Lines of Code**: 277
- **Priority**: Medium

**Problems Identified**:

**Lines 14-43: Repository Tab Title Generation**
```rust
let tab_titles: Vec<Line> = app
    .store
    .state()
    .repos
    .recent_repos
    .iter()
    .enumerate()
    .map(|(i, repo)| {
        // STATE CHECK: Is this repo loading?
        let is_loading = app
            .store
            .state()
            .repos
            .repo_data
            .get(&i)
            .map(|data| matches!(data.loading_state, LoadingState::Loading))
            .unwrap_or(false);

        // FORMATTING LOGIC: Number formatting
        let number = if i < 9 {
            format!("{} ", i + 1)
        } else {
            String::new()
        };

        // CONDITIONAL LOGIC: Loading indicator
        let prefix = if is_loading { "⏳ " } else { "" };

        Line::from(format!("{}{}{}/{}", prefix, number, repo.org, repo.repo))
    })
    .collect();
```

**Lines 121-240: Form Field Rendering (Repetitive)**
- Repetitive conditional styling for org/repo/branch fields
- Same pattern repeated 3 times with slight variations
- Not terrible, but could be cleaner with view model

**Recommended View Model**:
```rust
pub struct RepositoryTabsViewModel {
    pub title: String,              // "Projects [Tab/1-9: switch, /: cycle]..."
    pub tabs: Vec<TabItem>,
    pub selected_index: usize,
}

pub struct TabItem {
    pub display_text: String,       // "⏳ 1 org/repo" (pre-formatted)
    pub is_loading: bool,
}

pub struct AddRepoFormViewModel {
    pub instructions: String,
    pub fields: Vec<FormField>,
    pub footer: String,
}

pub struct FormField {
    pub label: String,
    pub value: String,
    pub is_focused: bool,
    pub indicator: String,          // "> " or "  "
    pub label_color: Color,
    pub value_color: Color,
    pub value_bg_color: Color,
}
```

**Benefits**:
- Loading state checks happen in view model
- Tab title formatting centralized
- Form field styling becomes data-driven
- Reduces repetitive code in view

---

### 8. `help.rs`
- **Current Issues**: ⚠️ Minor calculation logic, but mostly acceptable
- **Complexity**: Low-Medium
- **Lines of Code**: 158
- **Priority**: Low

**Analysis**:
This view is mostly fine. It has some scroll calculation logic (lines 87-102), but it's relatively simple. The content is static (shortcuts) and already comes from a separate function (`get_shortcuts()`).

**Verdict**: ✅ Acceptable as-is (low priority for refactoring)

**If Refactoring (Optional)**:
```rust
pub struct ShortcutsPanelViewModel {
    pub title: String,              // "Keyboard Shortcuts [5/45]"
    pub categories: Vec<ShortcutCategory>,
    pub footer: String,
    pub scroll_offset: usize,
    pub max_scroll: usize,
}

pub struct ShortcutCategory {
    pub name: String,
    pub shortcuts: Vec<ShortcutItem>,
    pub name_color: Color,
}

pub struct ShortcutItem {
    pub key_display: String,        // Pre-formatted to 18 chars
    pub description: String,
    pub key_color: Color,
    pub desc_color: Color,
}
```

---

## Summary Table

| View | Has View Model? | Needs View Model? | Priority | Complexity | Issues |
|------|-----------------|-------------------|----------|------------|---------|
| `pull_requests.rs` | ✅ Yes | N/A | - | High | None - excellent example |
| `build_log.rs` | ✅ Yes | N/A | - | High | None - excellent example |
| `status_bar.rs` | ❌ No | ❌ No | - | Very Low | None - too simple |
| `help.rs` | ❌ No | 🟡 Optional | Low | Low-Medium | Minor scroll calc |
| `splash_screen.rs` | ❌ No | ✅ Yes | Medium | Medium | Business logic in view |
| `command_palette.rs` | ❌ No | ✅ Yes | **High** | High | Complex layout calc |
| `debug_console.rs` | ❌ No | ✅ Yes | **High** | Medium-High | Data transformation |
| `repositories.rs` | ❌ No | ✅ Yes | Medium | Medium | State checks, repetition |

---

## Recommended Implementation Order

1. **High Priority**: `command_palette.rs` - Most complex, highest benefit
2. **High Priority**: `debug_console.rs` - Heavy data transformation
3. **Medium Priority**: `repositories.rs` - Repository tabs have clear view model structure
4. **Medium Priority**: `splash_screen.rs` - Clean bootstrap state interpretation
5. **Low Priority**: `help.rs` - Optional, mostly for consistency

---

## MVVM Pattern Benefits (Observed)

Based on the successful implementations in `pull_requests.rs` and `build_log.rs`:

### ✅ Benefits Achieved:
1. **View Simplicity**: Views become < 100 lines, mostly iteration
2. **Testability**: View models can be unit tested without UI
3. **Reusability**: Same view model could drive different UI frameworks
4. **Performance**: View models computed once in reducer, not on every frame
5. **Separation**: Business logic lives in reducers, presentation logic in view models, rendering in views

### 📏 Good View Model Characteristics:
- Contains **only** presentation-ready data (strings, colors, booleans)
- Pre-computes all formatting, truncation, padding
- Pre-determines all colors based on state
- Pre-calculates scroll offsets and visible ranges
- Has no references to `app` or `state` (views pass them in)
- Is `Clone` + `Debug` for easy debugging

### 🎯 Good View Characteristics (After MVVM):
- **No** business logic (no match on domain enums)
- **No** data transformation (no string formatting, no calculations)
- **No** conditional logic beyond styling
- Simple iteration over `view_model.rows` or similar
- Direct mapping of view model fields to widgets
- Typically < 100 lines of code
