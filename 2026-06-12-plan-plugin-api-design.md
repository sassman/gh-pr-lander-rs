# Plugin API design

Date: 2026-06-12
Status: validated brainstorm, ready for implementation planning
Companion document: `TARGET_ARCHITECTURE.md` (conceptual + author guide)

## Goal

Allow third parties to extend `pr-lander` with new commands, side panels,
modals, fullscreen views, status-bar segments, and effect handlers — without
forking the binary or recompiling it. First-party features (today's host
behaviour and the Claude session) are themselves expressed as plugins, on the
same trait, on the same runtime. Eclipse RCP / OSGi as the design rhyme.

## Constraints set during brainstorm

- Plugins are **sandboxed**: each plugin owns its state and its own message
  type; the host sees only `dyn ErasedPlugin`.
- Cross-plugin contracts go through **API crates** (Rust) backed by
  **WIT packages** (WASM). Domain types like `Pr` live in the API crate.
- Effects use a **Crux-style `Command<Msg>` builder** (`then_send`, `map`,
  `and_then`). No `Box<dyn Any>` in plugin author code.
- Cross-plugin handlers form a **ranked list per Operation**; default
  invocation runs highest-priority. Consumers can enumerate by provider.
- **Static linking** for first-party plugins; **WASM Component Model**
  (`wasm32-wasip2`, `wit-bindgen`, `wasmtime`) for the public dynamic path.
  cdylib explicitly rejected (unstable Rust ABI, no sandbox, codesigning
  pain).
- **No per-frame contribution polling.** Commands and keybindings register
  once at `on_register`; UI manifests as a result of operations
  (`MountEdgePanel`, `ShowModal`, `PushFullscreen`); status segments are
  pushed via operations.
- **Naming:** new crates prefix with `pr-lander-`. Existing `gh-pr-*` crates
  rename in their own migrations later.

## Workspace topology (target)

```
pr-lander                    # binary (was gh-pr-lander)
pr-lander-runtime            # Plugin trait, Command<Msg>, Operation,
                             # Registration, host runtime, WASM loader
pr-lander-foundation-api     # domain types (Pr, Repo, ...) + Operation defs
pr-lander-foundation         # platform plugin: today's host behaviour
pr-lander-claude-api         # (optional) Claude-specific operations
pr-lander-claude             # Claude session plugin

# untouched in this design (rename later)
gh-pr-tui-command-palette
gh-pr-config / gh-pr-config-migrate
gh-client / gh-diff-viewer / gh-actions-log-parser / gh-api-cache
gh-pr-lander-theme
```

## Core trait

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

    fn config_loaded(&mut self, _cfg_bytes: &[u8], _ctx: &HostContext)
        -> Command<Self::Msg> { Command::done() }
}
```

The trait is type-erased at the registry boundary via a blanket
`ErasedPlugin` impl. Plugin authors never write `Box<dyn Any>`.

`Registration` is a builder handed once at startup:

```rust
reg.command("send", "Send to Claude")
   .description("...")
   .category("Claude");
reg.keybinding("Ctrl+J", "send");
reg.keybinding_when_focused("p", "toggle", "session");
reg.view::<SessionView>("session");
reg.config::<ClaudeConfig>();
```

Registered commands have a fully-qualified ID `<plugin-id>.<local-name>`,
e.g. `claude.send`. The host interns these into `CommandRef(u32)` for fast
keymap lookup. `CommandId` enum is removed entirely.

## Operations and `Command<Msg>`

```rust
pub trait Operation: Send + 'static {
    type Output: Send + 'static;
}

pub struct Command<Msg>(CmdNode<Msg>);

enum CmdNode<Msg> {
    Done,
    Send(Msg),
    Op { op: Box<dyn ErasedOp>, k: Continuation<Msg> },
    Batch(Vec<Command<Msg>>),
    Stream { op: Box<dyn ErasedStreamOp>, k: StreamContinuation<Msg> },
}
```

Builder API:

```rust
Command::done();
Command::send(Msg::Tick);
Command::operation(op).map(...).and_then(...).then_send(Msg::Got);
Command::batch(vec![a, b, c]);
Command::stream(op).on_item(Msg::Chunk).on_close(|_| Msg::Done);
```

The runtime owns one `tokio::Runtime`. It walks each plugin-returned
`Command<Msg>` tree, dispatches each `Op` to the highest-priority handler
registered for `op.type_id()`, and re-enters `plugin.update(msg, ctx)` when
continuations resolve. Stream operations are spawned tasks that emit
`on_item` per yielded item.

## Built-in operations

The runtime provides these directly:

| Operation | Purpose |
|---|---|
| `ShowModal { view, props }` | Mount a modal (palette, popup, …) |
| `DismissModal { view }` | Close a modal |
| `PushFullscreen { view }` / `PopFullscreen` | Replace main pane |
| `MountEdgePanel { edge, view, size, style }` | Side/top/bottom drawer |
| `UnmountEdgePanel { edge }` | Hide drawer |
| `RequestFocus(ViewRef)` | Move focus to a mounted view |
| `SetStatusSegment { id, text }` / `ClearStatusSegment { id }` | Bottom-bar chip |
| `OpenUrl(String)` | Hand off to OS browser |
| `GetSecret { key }` | Keychain-backed secret lookup |
| `SubscribeKeys { scope }` (stream) | Raw keystrokes when scope focused |
| `SubscribeConfigChanges { plugin_id }` (stream) | Future hot-reload hook |

Where `Edge ∈ {Top, Right, Bottom, Left}` and
`PanelStyle ∈ {Solid, Overlay { dim_background: bool }}`.

Today's actual usage:

- Right + Solid → Claude session
- Top + Overlay { dim: true } → debug console (Quake-style)

`Bottom` and `Left` cost nothing to leave unused; the layout engine simply
doesn't allocate for them.

## Cross-plugin contracts

API crates ship Operation types + domain types together. Provider plugins
register handlers for those operations; consumer plugins import the API
crate and use them.

```rust
// pr-lander-foundation-api
pub struct Pr { pub number: u64, pub title: String, /* ... */ }
pub struct FetchRepoPrs { pub repo: Repo }
impl Operation for FetchRepoPrs { type Output = Result<Vec<Pr>, FetchError>; }

// pr-lander-foundation, on_register
reg.handle::<FetchRepoPrs>(|op, ctx| async move {
    cache_or_fetch(&ctx.octocrab, op.repo).await
}).priority(50);

// pr-lander-claude, in invoke
Command::operation(FetchRepoPrs { repo: ctx.current_repo() })
    .then_send(Msg::PrsLoaded)
```

**Handler ranking.** Each handler registers with a `priority: u32`. Default
invocation runs the highest-priority handler. Consumers can enumerate via
`ctx.handlers::<FetchRepoPrs>()` and pick a specific provider by id.

**Compile-time discoverability.** A consumer cannot reference an Operation
type without depending on its API crate. No runtime "operation not found"
surprises in the static path. WASM consumers fail to instantiate if the
required WIT package is absent.

## Surfaces

| Surface | Operation | Today's user |
|---|---|---|
| Modal | `ShowModal` | command palette, confirmation, add-repo, keybindings help |
| Fullscreen | `PushFullscreen` | diff viewer, build log |
| Edge panel (Solid) | `MountEdgePanel` Edge::Right | Claude session |
| Edge panel (Overlay+dim) | `MountEdgePanel` Edge::Top | debug console |
| Status segment | `SetStatusSegment` | bottom bar |

**One panel per edge, last-wins.** Mounting into an occupied edge unmounts
the previous occupant; the displaced plugin gets a `PanelUnmounted(view_id)`
message.

**Focus rule.** Topmost mounted surface wins (Modal > Fullscreen >
Overlay edge > Solid edge > main). Plugin can request focus with
`Operation::RequestFocus(ViewRef)`.

**Keybinding scopes.**

```rust
reg.keybinding("Ctrl+J", "send");                          // global
reg.keybinding_when_focused("p", "toggle", "session");     // view-scoped
```

Resolution: focused view's bindings first, then global. No per-plugin scope.

**Raw keys** are a stream subscription, not a special hook:

```rust
Command::stream(SubscribeKeys { scope: ViewRef::local("session") })
    .on_item(Msg::RawKey)
    .on_close(|_| Msg::KeysDetached)
```

## Configuration

One file with namespaced sections, `~/.config/pr-lander/config.toml`:

```toml
[runtime]
multiplexer = "tmux"

[plugins.foundation]
default_repo = "owner/repo"

[plugins.claude]
prompt = "..."
permissions = ["bash"]
```

Plugins declare schema via `reg.config::<MyConfig>()` (must implement
`Deserialize + Default`). Host parses the matching section, calls
`plugin.config_loaded(bytes, ctx)`. Missing section → `Default::default()`.
Bad TOML → plugin disabled with a clear error pointing at the section.

**Secrets** never live in TOML. Plugins ask the host:

```rust
Command::operation(Operation::GetSecret { key: "anthropic_api_key" })
    .then_send(Msg::ApiKeyLoaded)
```

Host backs `GetSecret` with the system keychain (`keyring` crate); falls
back to environment variables for CI/headless. Keys are namespaced as
`pr-lander.<plugin-id>.<key>`.

**Hot reload, settings UI: out of scope for v1.** Both extensible later
without API breakage (config-change stream, generic settings plugin).

## Loading

### Static (first-party)

```rust
// in pr-lander/src/main.rs
let mut runtime = Runtime::new();
runtime.register(pr_lander_foundation::FoundationPlugin::new());
runtime.register(pr_lander_claude::ClaudePlugin::new());
runtime.scan_plugins_dir("~/.config/pr-lander/plugins/");
runtime.run();
```

### Dynamic (WASM Component Model)

Each plugin ships:

```
my_plugin.wasm        # wasm32-wasip2 component
my_plugin.toml        # manifest
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

Loader algorithm:

1. Scan `~/.config/pr-lander/plugins/`, parse all `*.toml`.
2. Build dep graph from `requires`. Topological sort.
3. Load in order. Each `requires` entry must be satisfied by an
   already-loaded plugin's `provides` at compatible semver. If not → log
   warning, mark plugin disabled, continue. No partial loads.
4. Instantiate WASM component, wire host imports, call `on_register`.

### Why not cdylib

- Rust ABI unstable; every boundary type must go through `abi_stable`
  (`RVec<RPr>`) — destroys the design's ergonomics.
- No sandbox; full process privileges.
- Codesigning + Gatekeeper friction on macOS.
- Two compiled `tokio`s = UB; pinning `rustc` per plugin is untenable.

WASM costs ~10–30 % CPU on hot paths (negligible for a TUI driving GitHub
API), gains stable ABI + capability sandbox + cross-platform single
artifact.

## Migration order

| # | Step | Visible change |
|---|---|---|
| 0 | Land `feat/fix-pr-with-claude`. Rename binary crate `gh-pr-lander` → `pr-lander`. | binary name |
| 1 | Add empty `pr-lander-runtime` (trait, `Command<Msg>`, `Operation`, `Registration`) + no-op `PluginMiddleware` in chain. | none |
| 2 | Build Operation executor + handler registry. Smoke-test with trivial in-tree `HelloPlugin`. | one debug command |
| 3 | Carve `pr-lander-foundation-api`: move `domain_models/` and define operations (`FetchRepoPrs`, `MergePr`, …). Existing middleware unchanged. | none |
| 4 | Add empty `pr-lander-foundation` plugin to startup. | none |
| 5 | Migrate commands one at a time (`CommandId` enum + middleware → foundation `invoke` + handlers). Start with leaves (`RepositoryOpenInBrowser`). | none per-command |
| 6 | When `CommandId` is empty, delete it. Switch keymap to `CommandRef(u32)`. | symbolic milestone |
| 7 | Migrate views one at a time (debug console, palette, confirmation, diff viewer, build log) to plugin views mounted via operations. | none per-view |
| 8 | Extract Claude into `pr-lander-claude`. **First non-trivial second plugin.** | validates API |
| 9 | Add WASM Component loader; smoke-test with `HelloPlugin` → `wasm32-wasip2`. | `plugins/` honoured |
| 10 | Stabilise WIT for `pr-lander-foundation-api` v1.0; publish API + WIT package. | external authors can ship |

Each phase compiles, ships, runs. No flag day. Phases 5 + 7 are the long
tail; everything else is small.

## Open questions to resolve during implementation planning

- Naming nit: keep `Foundation` as the platform-plugin name, or pick
  something less generic? (`Core`, `Base`, `Workbench`.)
- Should `pr-lander-runtime` re-export `ratatui` types used in view
  signatures, or pin a specific ratatui version through WIT? (Affects how
  WASM plugins draw — likely they call `Operation::Draw*` primitives
  rather than render `Frame` directly.)
- Concrete shape of `HostContext`: which read-only accessors are baked in
  (`current_repo`, `current_pr`, `theme`) versus which become
  Subscribe-style streams (`SubscribeCurrentPr`)?
- Versioning strategy for the API crates: lockstep with `pr-lander-runtime`
  initially, or independent semver from day one?
- Where do per-plugin logs live? Today's debug console captures
  `log::info!` etc. across the binary. Plugins likely want a host-provided
  `Operation::Log { level, target, message }`.

## Worked example: `pr-lander-claude` after step 8

Sketched in full detail in `TARGET_ARCHITECTURE.md`. Summary: the plugin
holds its own `sessions: HashMap<PrNumber, Session>`, defines its own
`Msg`, asks the host for `FetchPrDetails` (from `pr-lander-foundation-api`)
and built-in operations (`MountEdgePanel`, `RequestFocus`, `SubscribeKeys`).
Host never sees `Msg`, never imports `Session`. The PTY uses
`Command::stream` like any other long-lived effect.
