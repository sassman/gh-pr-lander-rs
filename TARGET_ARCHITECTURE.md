# Target architecture

Status: design — not yet implemented. Tracked by
`docs/plans/2026-06-12-plugin-api-design.md`. Companion to today's
`ARCHITECTURE.md`, which describes the current state.

This document is the reference + author guide for the plugin model
`pr-lander` is moving towards. Read it once if you intend to write a
plugin or change the runtime. The current `ARCHITECTURE.md` (Redux loop,
middleware, reducers) describes today; this document describes the target.

---

## Why plugins

`pr-lander` started as a TUI for one job — landing GitHub pull requests.
Every new capability (Claude session, Zellij/tmux, GitHub Enterprise, the
debug console, the diff viewer) currently has to touch the central
`Action` enum, the `CommandId` enum, the middleware list, and the views
module. Three friction points:

1. **The host enum grows monotonically.** Every contributor adds variants.
   Conflicts in PRs are mostly enum-merge conflicts.
2. **There is no extension point.** Users cannot add a "list this PR in
   Linear" command without forking and recompiling.
3. **Other backends will be hard.** "GitHub" is in the binary name today;
   Gitea/GitLab/local-git support means adding a backend dimension to
   every enum.

The target architecture inverts this: the host binary is a thin runtime
plus a registry; everything user-visible lives in plugins. The platform
itself ships as one plugin (`pr-lander-foundation`); a Gitea backend
becomes another. Eclipse RCP / OSGi is the design rhyme — *everything is
a plugin, including the platform.*

---

## The mental model

Two directions of plugin ↔ host interaction. Keep them distinct in your
head.

```
                    ┌──────────────┐
                    │    PLUGIN    │
                    └──┬─────────┬─┘
                       │         │
          registration │         │  Command<Msg>
          (push, once  │         │  (push, ongoing —
           at startup) │         │   side-effects + msgs)
                       ▼         ▼
                    ┌──────────────┐
                    │     HOST     │
                    └──────────────┘
```

| Direction | Mechanism | What flows |
|---|---|---|
| Plugin → Host (once) | `Registration` builder at `on_register` | command IDs, keybindings, view types, config schema, effect handlers |
| Plugin → Host (ongoing) | `Command<Msg>` returned from `invoke` / `update` | requests for side-effects (`Operation`s), messages to deliver back |
| Host → Plugin | `invoke(name, ctx)`, `update(msg, ctx)`, `config_loaded(...)` | user actions on the plugin's commands; results of operations; loaded config |

There is **no per-frame pull from the host into the plugin**. Commands and
keybindings are registered once. UI manifests as a *result* of invoking a
command — `MountEdgePanel`, `ShowModal`, `PushFullscreen` are operations
the plugin returns. Status-bar content is pushed via `SetStatusSegment`.
Only mounted views are rendered, and only their owning plugin pays the
render cost.

---

## Core concepts

### `Plugin` trait

```rust
pub trait Plugin: Send + 'static {
    type Msg: Send + 'static;

    fn id(&self) -> PluginId;

    fn on_register(&mut self, reg: &mut Registration<Self::Msg>)
        -> Command<Self::Msg>;

    fn invoke(&mut self, name: &str, ctx: &HostContext)
        -> Command<Self::Msg>;

    fn update(&mut self, msg: Self::Msg, ctx: &HostContext)
        -> Command<Self::Msg>;

    fn config_loaded(&mut self, _bytes: &[u8], _ctx: &HostContext)
        -> Command<Self::Msg> { Command::done() }
}
```

Each plugin owns its state inside `Self`. `Self::Msg` is the plugin's
private message type — host treats it as opaque (erased at the
trait-object boundary). Plugin authors never write `Box<dyn Any>`.

### `Registration`

The runtime hands the plugin a `Registration<Msg>` once at startup:

```rust
fn on_register(&mut self, reg: &mut Registration<Msg>) -> Command<Msg> {
    reg.command("send", "Send to Claude")
       .description("Send the focused PR to a Claude session")
       .category("Claude");

    reg.keybinding("Ctrl+J", "send");
    reg.keybinding_when_focused("p", "toggle", "session");

    reg.view::<SessionView>("session");
    reg.config::<ClaudeConfig>();

    reg.handle::<MyOperation>(handler).priority(50);

    Command::done()    // or initial effects
}
```

The fully-qualified command ID becomes `<plugin-id>.<local-name>`
(e.g. `claude.send`). The runtime interns these into `CommandRef(u32)` for
fast keymap lookup.

### `Command<Msg>` and `Operation`s

Plugins describe work as a *value*, not as a future. The runtime drains
the value:

```rust
pub trait Operation: Send + 'static {
    type Output: Send + 'static;
}

Command::done()
Command::send(Msg::Tick)
Command::operation(op)
    .map(|out| transform(out))
    .and_then(|out| Command::operation(next_op(out)))
    .then_send(Msg::Got)
Command::batch(vec![cmd_a, cmd_b])
Command::stream(op)
    .on_item(Msg::Chunk)
    .on_close(|_| Msg::Done)
```

Inspired by Crux's `Command` builder. The chain stays type-safe end to end:
`Operation::Output` threads through `map` / `and_then` / `then_send` so
your closures take typed values, never `Box<dyn Any>`.

### `HostContext`

Read-only accessors for things every plugin tends to need:

```rust
ctx.current_repo()        -> Option<&Repo>
ctx.current_pr_number()   -> Option<PrNumber>
ctx.current_pr_id()       -> Option<&PrId>
ctx.theme()               -> &Theme
ctx.handlers::<Op>()      -> &[HandlerHandle]
ctx.dispatcher()          -> &Dispatcher<...>     // for crossing the FFI back into invoke
```

Anything that changes during a session — focus changes, repo switches, PR
loads — is delivered as a `Subscribe*` stream rather than read on demand.
Pull from `HostContext` for things that are stable for the duration of an
`invoke` / `update` call; subscribe for things you want to react to.

### Surfaces

The runtime hosts five surfaces. A plugin claims a surface by returning
the matching operation; nothing about a plugin says "I am a panel" up
front.

| Surface | Operation | Notes |
|---|---|---|
| Modal | `ShowModal { view, props }` | centred overlay; topmost takes focus |
| Fullscreen | `PushFullscreen { view }` / `PopFullscreen` | replaces main pane |
| Edge panel | `MountEdgePanel { edge, view, size, style }` | one per edge, last-wins |
| Status segment | `SetStatusSegment { id, text }` / `ClearStatusSegment` | bottom-bar chip |
| Raw keys | `SubscribeKeys { scope }` (stream) | when scope view is focused |

```rust
pub enum Edge { Top, Right, Bottom, Left }

pub enum PanelStyle {
    Solid,                              // claims layout space
    Overlay { dim_background: bool },   // floats over everything
}
```

Today only `Edge::Right` (Solid, Claude session) and `Edge::Top`
(Overlay+dim, debug console) are used. Adding a left sidebar is a feature
the layout engine grows; the operation already accepts it.

**Focus rule:** topmost mounted surface wins
(Modal > Fullscreen > Overlay edge > Solid edge > main). Plugins request
focus via `Operation::RequestFocus(ViewRef)` (e.g. Claude swapping focus
between PR list and session panel).

**Keybinding scopes:** `keybinding(...)` is global,
`keybinding_when_focused(key, cmd, view_id)` is view-scoped. Resolution:
focused view's bindings first, then global. No deeper scope hierarchy.

---

## Cross-plugin contracts: API crates

Operations are the unit of cross-plugin contract. To expose effects to
other plugins, ship a thin sibling crate that contains *only*:

1. `Operation` types (request shapes)
2. Domain types those operations produce / consume
3. Optional capability shims for ergonomic builders

```
pr-lander-foundation         # implementation: registers handlers, owns state
pr-lander-foundation-api     # contract: types + Operation defs (PUBLIC)
```

Other plugins depend on the **API crate**, never the implementation:

```rust
// pr-lander-foundation-api  (Cargo.toml: no octocrab, no host deps)
pub struct Pr { pub number: u64, pub title: String, pub state: PrState }
pub struct Repo { pub org: String, pub repo: String }

pub struct FetchRepoPrs { pub repo: Repo }
impl Operation for FetchRepoPrs {
    type Output = Result<Vec<Pr>, FetchError>;
}
```

Provider side:

```rust
// pr-lander-foundation
fn on_register(&mut self, reg: &mut Registration<Msg>) -> Command<Msg> {
    reg.handle::<FetchRepoPrs>(|op, ctx| async move {
        cache_or_fetch(&ctx.octocrab, op.repo).await
    }).priority(50);
    Command::done()
}
```

Consumer side — type-safe, just import the API crate:

```rust
// pr-lander-claude  (Cargo.toml depends on pr-lander-foundation-api)
use pr_lander_foundation_api::{FetchRepoPrs, Pr};

fn invoke(&mut self, name: &str, ctx: &HostContext) -> Command<Msg> {
    if name == "summarise" {
        return Command::operation(FetchRepoPrs { repo: ctx.current_repo().unwrap().clone() })
            .then_send(Msg::PrsLoaded);
    }
    Command::done()
}
```

**Three properties this gives you:**

- **Compile-time discoverability.** A consumer can't reference an
  Operation without depending on its API crate.
- **Implementation swap.** Anyone can ship an alternative
  `pr-lander-foundation` (e.g. a Gitea backend) that registers handlers
  for the same operations. Consumers don't change.
- **Domain types are plain structs.** `Pr`, `Repo` live in `*-api`, used
  by name everywhere. No marshalling.

**Handler ranking.** Multiple plugins may register for the same
Operation. Default invocation runs the highest-priority handler.
Consumers can enumerate via `ctx.handlers::<Op>()` and pick a specific
provider by id (the OSGi service-ranking pattern).

---

## Loading

### Static (first-party, workspace plugins)

```rust
// pr-lander/src/main.rs
let mut runtime = Runtime::new();
runtime.register(pr_lander_foundation::FoundationPlugin::new());
runtime.register(pr_lander_claude::ClaudePlugin::new());
runtime.scan_plugins_dir("~/.config/pr-lander/plugins/");
runtime.run();
```

Compile-time linked, zero overhead, full Rust types end to end.

### Dynamic (WASM Component Model)

Each third-party plugin ships:

```
my_plugin.wasm       # wasm32-wasip2 component
my_plugin.toml       # manifest
```

Manifest:

```toml
id = "my-plugin"
version = "0.3.0"
artifact = "my_plugin.wasm"

provides = []
requires = [
  { api = "foundation-api", version = "^1.0" },
]

[grants]
network = ["api.example.com"]
```

Loader:

1. Scan `~/.config/pr-lander/plugins/`, parse all `*.toml`.
2. Build dep graph from `requires`. Topological sort.
3. Load in order. Each `requires` entry must be satisfied by an
   already-loaded plugin's `provides` at compatible semver. If not → log
   warning, mark plugin disabled, continue.
4. Instantiate WASM component, wire host imports via WIT, call
   `on_register`.

**Why WASM and not cdylib:** Rust's ABI is unstable; cdylib loading
forces every boundary type through `abi_stable` (`RVec<RPr>`), destroying
the design's ergonomics. cdylib also has no sandbox, no codesigning
story on macOS, and no protection from `tokio` version skew. WASM costs
~10–30 % CPU on hot paths (negligible for a TUI driving GitHub API) and
gains stable ABI + capability sandbox + cross-platform single artifact.

---

## Configuration & secrets

One file with namespaced sections, `~/.config/pr-lander/config.toml`:

```toml
[runtime]
multiplexer = "tmux"

[plugins.foundation]
default_repo = "owner/repo"

[plugins.claude]
prompt = "..."
permissions = ["bash"]

[plugins."open-in-zed"]
binary_path = "/usr/local/bin/zed"
```

Plugins declare schema at registration:

```rust
#[derive(Deserialize, Default)]
pub struct ClaudeConfig {
    pub prompt: String,
    #[serde(default)]
    pub permissions: Vec<String>,
}

reg.config::<ClaudeConfig>();
```

The host parses the matching `[plugins.<id>]` section and calls
`plugin.config_loaded(bytes, ctx)`. Missing section →
`Default::default()`. Bad TOML → plugin disabled with a clear error.

**Secrets** (API keys, tokens) never live in TOML in plain text. Plugins
ask the host:

```rust
Command::operation(Operation::GetSecret { key: "anthropic_api_key" })
    .then_send(Msg::ApiKeyLoaded)
```

Backed by the system keychain (`keyring` crate); falls back to env vars
for CI/headless. Keys are namespaced as
`pr-lander.<plugin-id>.<key>`.

WASM plugins declare their config schema in the manifest as JSON Schema
(`schemars`-generated if also built natively). The host validates user
config against the manifest schema before instantiating the component.

---

## Worked example 1 — `open-in-zed` (stateless plugin)

The simplest possible third-party plugin: contributes one command,
spawns a process when invoked, holds no state.

```rust
// crates/open-in-zed/src/lib.rs
use pr_lander_runtime::{Plugin, PluginId, Registration, HostContext, Command, Operation};
use pr_lander_foundation_api::{Pr, FetchPrDetails};
use serde::Deserialize;

#[derive(Default, Deserialize)]
pub struct ZedConfig {
    #[serde(default = "default_zed_path")]
    binary_path: String,
}
fn default_zed_path() -> String { "zed".into() }

pub struct OpenInZed { cfg: ZedConfig }

pub enum Msg {
    PrLoaded(Result<Pr, pr_lander_foundation_api::FetchError>),
}

impl OpenInZed {
    pub fn new() -> Self { Self { cfg: ZedConfig::default() } }
}

impl Plugin for OpenInZed {
    type Msg = Msg;

    fn id(&self) -> PluginId { PluginId::new("open-in-zed") }

    fn on_register(&mut self, reg: &mut Registration<Msg>) -> Command<Msg> {
        reg.command("open", "Open PR in Zed")
           .description("Clone the PR's branch and open it in Zed")
           .category("PR Actions");
        reg.keybinding_when_focused("z", "open", "pr-list");
        reg.config::<ZedConfig>();
        Command::done()
    }

    fn config_loaded(&mut self, bytes: &[u8], _ctx: &HostContext) -> Command<Msg> {
        if let Ok(cfg) = toml::from_slice(bytes) { self.cfg = cfg; }
        Command::done()
    }

    fn invoke(&mut self, name: &str, ctx: &HostContext) -> Command<Msg> {
        match name {
            "open" => {
                let Some(pr_num) = ctx.current_pr_number() else { return Command::done() };
                let Some(repo) = ctx.current_repo() else { return Command::done() };
                Command::operation(FetchPrDetails { repo: repo.clone(), number: pr_num })
                    .then_send(Msg::PrLoaded)
            }
            _ => Command::done(),
        }
    }

    fn update(&mut self, msg: Msg, _ctx: &HostContext) -> Command<Msg> {
        match msg {
            Msg::PrLoaded(Ok(pr)) => Command::operation(Operation::SpawnProcess {
                cmd: self.cfg.binary_path.clone(),
                args: vec![pr.checkout_path()],
                detach: true,
            }),
            Msg::PrLoaded(Err(e)) => Command::operation(Operation::SetStatusSegment {
                id: "open-in-zed.error",
                text: format!("zed: {e}").into(),
            }),
        }
    }
}
```

Properties of this plugin:

- No long-lived state worth talking about — `cfg` is loaded once and read.
- Pulls the current PR from `HostContext` synchronously; asks the host
  to fetch full PR details via a foundation-api operation.
- All work expressed as `Command<Msg>` — never spawns a thread, never
  touches `tokio`.
- Zero coupling to the host's `Action` enum. Compiles against
  `pr-lander-runtime` + `pr-lander-foundation-api` only.

---

## Worked example 2 — `pr-lander-claude` (stateful plugin with PTY)

The motivating non-trivial case: spawns Claude as a PTY, mounts a side
panel, streams its output, captures raw keys when focused.

```rust
// crates/pr-lander-claude/src/lib.rs
use std::collections::HashMap;
use bytes::Bytes;
use pr_lander_runtime::{
    Plugin, PluginId, Registration, HostContext, Command,
    Operation, ViewRef, Edge, PanelStyle, Constraint, KeyEvent,
};
use pr_lander_foundation_api::{Pr, FetchPrDetails, FetchError};

pub struct ClaudePlugin {
    sessions: HashMap<u64, Session>,
    focused: Option<u64>,
}

struct Session { /* pty handle, scrollback, etc. */ }

pub enum Msg {
    PrLoaded(Result<Pr, FetchError>),
    PanelMounted,
    PanelUnmounted,
    PtyChunk { pr: u64, bytes: Bytes },
    PtyClosed { pr: u64 },
    RawKey(KeyEvent),
}

impl Plugin for ClaudePlugin {
    type Msg = Msg;

    fn id(&self) -> PluginId { PluginId::new("claude") }

    fn on_register(&mut self, reg: &mut Registration<Msg>) -> Command<Msg> {
        reg.command("toggle", "Toggle Claude session");
        reg.command("send", "Send selection to Claude");
        reg.view::<SessionView>("session");
        reg.keybinding("Ctrl+J", "toggle");
        reg.config::<ClaudeConfig>();
        Command::done()
    }

    fn invoke(&mut self, name: &str, ctx: &HostContext) -> Command<Msg> {
        match name {
            "toggle" => {
                let Some(pr) = ctx.current_pr_number() else { return Command::done() };
                if self.sessions.contains_key(&pr) {
                    Command::operation(Operation::UnmountEdgePanel { edge: Edge::Right })
                } else {
                    Command::operation(FetchPrDetails {
                        repo: ctx.current_repo().unwrap().clone(),
                        number: pr,
                    }).then_send(Msg::PrLoaded)
                }
            }
            _ => Command::done(),
        }
    }

    fn update(&mut self, msg: Msg, _ctx: &HostContext) -> Command<Msg> {
        match msg {
            Msg::PrLoaded(Ok(pr)) => {
                self.sessions.insert(pr.number, Session::spawn(&pr));
                self.focused = Some(pr.number);
                Command::batch(vec![
                    Command::operation(Operation::MountEdgePanel {
                        edge: Edge::Right,
                        view: ViewRef::local("session"),
                        size: Constraint::Percentage(40),
                        style: PanelStyle::Solid,
                    }).then_send(|_| Msg::PanelMounted),
                    Command::operation(Operation::RequestFocus(ViewRef::local("session"))),
                ])
            }
            Msg::PanelMounted => {
                Command::stream(Operation::SubscribeKeys {
                    scope: ViewRef::local("session"),
                })
                .on_item(Msg::RawKey)
                .on_close(|_| Msg::PanelUnmounted)
            }
            Msg::RawKey(k) => {
                if let Some(pr) = self.focused {
                    if let Some(s) = self.sessions.get_mut(&pr) { s.write_key(k); }
                }
                Command::done()
            }
            Msg::PtyChunk { pr, bytes } => {
                if let Some(s) = self.sessions.get_mut(&pr) { s.append(bytes); }
                Command::done()
            }
            Msg::PtyClosed { pr } => {
                self.sessions.remove(&pr);
                Command::operation(Operation::UnmountEdgePanel { edge: Edge::Right })
            }
            Msg::PanelUnmounted => {
                self.focused = None;
                Command::done()
            }
            Msg::PrLoaded(Err(e)) => Command::operation(Operation::SetStatusSegment {
                id: "claude.error",
                text: format!("claude: {e}").into(),
            }),
        }
    }
}
```

Properties:

- Owns its state (`sessions`, `focused`) in `Self`. Never mutates
  `AppState`.
- `Msg` is the plugin's private vocabulary; host never sees it.
- Asks `pr-lander-foundation-api` for PR details (cross-plugin
  contract) and the runtime for built-in operations (mount/unmount panel,
  request focus, subscribe keys).
- The PTY is just `Command::stream(Operation::SubscribeKeys)` — same
  pattern as any other long-lived effect. No bespoke threading.

---

## Anti-patterns / non-goals

- **Plugins do not call into `AppState`.** All state access goes through
  `HostContext` accessors and `Subscribe*` streams. If you want to read
  something not exposed, propose adding it to `HostContext`.
- **Plugins do not spawn their own runtime.** The runtime owns
  `tokio`. Side-effects flow through `Command::operation` /
  `Command::stream`. If your plugin needs an effect the runtime doesn't
  expose, add an Operation to your plugin's API crate.
- **Plugins do not emit log lines as side effects of `update`.** Use
  `Operation::Log { level, target, message }`. Today's debug console will
  be backed by this operation post-migration.
- **No closures cross the boundary as actions.** Commands carry data;
  continuations carry typed `Msg` mappers; raw `Box<dyn Fn>` does not
  appear in any public API.
- **No reading `ratatui::Frame` from WASM.** WASM plugins draw via
  `Operation::Draw*` primitives that the host translates to ratatui calls.
  Static plugins may render `Frame` directly via the view trait — this
  is the one asymmetry between the two paths and is intentional (the
  ratatui surface is too large for WIT).
- **No hot-reloading config in v1.** Plugins receive `config_loaded` once
  at startup. A future `Operation::SubscribeConfigChanges` stream will
  add hot-reload without API breakage.

---

## Future extensions

The design is intentionally narrow at v1. These extend cleanly later:

- **Settings UI plugin.** A built-in plugin that introspects every
  registered config schema and renders a settings modal. Lives in its own
  crate, ships with `pr-lander`.
- **Telemetry / analytics plugin.** Subscribes to host streams
  (`SubscribeCurrentPr`, command-invocation events) and writes locally.
- **Alternative backends.** A `pr-lander-gitea` plugin that registers
  handlers for `FetchRepoPrs`, `MergePr`, etc. with priority equal to
  foundation's; the user picks the active backend in config.
- **Workspace-multiplexer plugin.** Today's tmux/zellij integration
  becomes its own plugin offering `Operation::OpenInMultiplexer`.
- **Notifications.** `Operation::Notify { level, body }` backed by
  `notify-rust`.
- **Network grants enforcement.** WASM plugins' `[grants].network` list
  is enforced by wasmtime's outbound HTTP capability; static plugins are
  trusted (they run in-process).

---

## Migration map (current → target)

| Today (`gh-pr-lander`) | Target |
|---|---|
| `Action` enum | mostly gone; only the runtime's internal coordination actions remain |
| `CommandId` enum | gone; replaced by `<plugin-id>.<local-name>` strings interned to `CommandRef` |
| `middleware/*` | host runtime's effect executor + foundation plugin's `invoke`/`update` |
| `reducers/*` | each plugin's `update` |
| `views/*` | each plugin's view types, mounted via operations |
| `state/*` | each plugin owns its own state in `Self` |
| `domain_models/*` | `pr-lander-foundation-api` |
| `actions/event.rs` | host-emitted streams (`SubscribeCurrentPr`, `SubscribeRepoList`, …) |

The migration is incremental — see
`docs/plans/2026-06-12-plugin-api-design.md` for the phase-by-phase order.
Each phase compiles, ships, and runs.
