# Codex Island Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a Tauri-based Windows desktop floating island that reads Codex hook state files and shows multi-session status with hover details.

**Architecture:** A Rust-first Tauri app owns the state model, file watching, stale detection, and hook-install UX. A separate Rust CLI helper writes per-session JSON state files atomically from Codex hook events. The React frontend stays thin: it renders the collapsed island, expanded list, and hide/unhide interactions without owning business rules.

**Tech Stack:** Tauri 2, Rust, React, TypeScript, Vite, Vitest, Testing Library, `notify` for file watching, `serde`/`serde_json` for state, `chrono` for timestamps, `tempfile` for Rust tests.

---

### Task 1: Bootstrap the Tauri workspace and app shell

**Files:**
- Create: `package.json`
- Create: `pnpm-workspace.yaml`
- Create: `vite.config.ts`
- Create: `index.html`
- Create: `src/main.tsx`
- Create: `src/App.tsx`
- Create: `src/styles.css`
- Create: `src-tauri/Cargo.toml`
- Create: `src-tauri/tauri.conf.json`
- Create: `src-tauri/src/main.rs`
- Create: `src-tauri/src/lib.rs`

- [ ] **Step 1: Scaffold the workspace and lock the runtime shape**

Use a React + TypeScript + Tauri app shell with these scripts:

```json
{
  "scripts": {
    "dev": "vite",
    "build": "vite build",
    "test": "vitest run",
    "tauri": "tauri"
  }
}
```

- [ ] **Step 2: Wire the Tauri window defaults**

Make the window transparent, always-on-top, undecorated, and centered near the top edge. The frontend should mount without any state logic yet.

- [ ] **Step 3: Verify the shell builds**

Run:

```bash
pnpm install
pnpm build
cd src-tauri && cargo test
```

Expected: frontend build succeeds and Rust tests are runnable even before feature code exists.

---

### Task 2: Define the shared Codex Island domain model and file paths

**Files:**
- Create: `src-tauri/src/domain.rs`
- Create: `src-tauri/src/paths.rs`
- Create: `src-tauri/src/time.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Write the failing Rust tests for state mapping and ordering**

```rust
#[test]
fn session_created_at_is_preserved() {
    let first = SessionRecord::new("abc".into(), "/work/a".into(), Source::Wsl, "Ubuntu".into())
        .with_created_at(Utc.with_ymd_and_hms(2026, 6, 3, 10, 0, 0).unwrap());
    let second = first.clone().with_event(HookEvent::PreToolUse);
    assert_eq!(first.created_at, second.created_at);
}

#[test]
fn hidden_session_reappears_only_after_newer_event() {
    let hidden_at = Utc.with_ymd_and_hms(2026, 6, 3, 10, 0, 0).unwrap();
    let record = SessionRecord::new("abc".into(), "/work/a".into(), Source::Windows, "".into())
        .with_updated_at(hidden_at + Duration::seconds(30));
    assert!(record.is_newer_than(hidden_at));
}

#[test]
fn status_priority_does_not_change_order() {
    let older = SessionRecord::new("older".into(), "/work/older".into(), Source::Wsl, "Ubuntu".into())
        .with_created_at(Utc.with_ymd_and_hms(2026, 6, 3, 9, 0, 0).unwrap())
        .with_ui_state(UiState::Running);
    let newer = SessionRecord::new("newer".into(), "/work/newer".into(), Source::Wsl, "Ubuntu".into())
        .with_created_at(Utc.with_ymd_and_hms(2026, 6, 3, 11, 0, 0).unwrap())
        .with_ui_state(UiState::Waiting);
    let mut sessions = vec![newer, older];
    sessions.sort_by_key(|s| s.created_at);
    assert_eq!(sessions[0].session_id, "older");
}
```

- [ ] **Step 2: Implement the domain types**

Define:

```rust
pub enum Source { Wsl, Windows }
pub enum UiState { Running, Completed, Waiting, Error, Stale }
pub enum HookEvent { SessionStart, UserPromptSubmit, PermissionRequest, PreToolUse, PostToolUse, Stop }

pub struct SessionRecord {
    pub session_id: String,
    pub turn_id: Option<String>,
    pub cwd: String,
    pub title: String,
    pub source: Source,
    pub distro: Option<String>,
    pub last_event: HookEvent,
    pub last_tool: Option<String>,
    pub ui_state: UiState,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
```

Add a helper that maps `cwd` to `title` by taking the last path segment.
Add `SessionRecord::new`, `with_created_at`, `with_updated_at`, `with_event`, and `with_ui_state` helpers so the tests above can stay small.

- [ ] **Step 3: Run the domain tests**

Run:

```bash
cd src-tauri
cargo test domain
```

Expected: the state mapping and ordering tests pass.

---

### Task 3: Implement the hook writer CLI with atomic file writes

**Files:**
- Create: `src-tauri/src/bin/codex-island-hook.rs`
- Create: `src-tauri/src/hook.rs`
- Create: `src-tauri/tests/hook_cli.rs`
- Modify: `src-tauri/Cargo.toml`

- [ ] **Step 1: Write the failing hook tests**

```rust
#[test]
fn writes_one_session_file_per_session_id() {
    let payload = format!(
        r#"{{
            "session_id": "abc123",
            "turn_id": "turn-1",
            "cwd": "/work/a",
            "hook_event_name": "PreToolUse",
            "tool_name": "Bash"
        }}"#
    );
    let result = parse_and_build_record(&payload).unwrap();
    assert_eq!(result.session_id, "abc123");
    assert_eq!(result.ui_state, UiState::Running);
}

#[test]
fn preserves_created_at_after_first_write() {
    let dir = tempfile::tempdir().unwrap();
    let first = write_record(dir.path(), SessionRecord::new("abc123".into(), "/work/a".into(), Source::Wsl, "Ubuntu".into()));
    let second = write_record(dir.path(), first.clone().with_event(HookEvent::PostToolUse));
    assert_eq!(first.created_at, second.created_at);
}
```

- [ ] **Step 2: Implement the hook parser and writer**

The CLI should:

1. Read the full hook event JSON from stdin.
2. Extract `session_id`, `turn_id`, `cwd`, `hook_event_name`, `tool_name`, `distro`, and `permission_mode` when available.
3. Infer `source` from `WSL_DISTRO_NAME`.
4. Create the session record with `created_at` on first write.
5. Update only mutable fields on later writes.
6. Write `sessions/<session_id>.json.tmp` and rename it into place.

Use this shape for the atomic write helper:

```rust
fn atomic_write(path: &Path, body: &str) -> std::io::Result<()> {
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, body)?;
    std::fs::rename(tmp, path)?;
    Ok(())
}
```

- [ ] **Step 3: Run hook tests**

Run:

```bash
cd src-tauri
cargo test hook_cli
```

Expected: the CLI parses hook JSON and writes stable state files.

---

### Task 4: Build the Rust state store, watcher, and stale engine

**Files:**
- Create: `src-tauri/src/store.rs`
- Create: `src-tauri/src/watcher.rs`
- Create: `src-tauri/src/state.rs`
- Modify: `src-tauri/src/main.rs`
- Create: `src-tauri/tests/store_watcher.rs`

- [ ] **Step 1: Write the failing store tests**

```rust
#[test]
fn sessions_are_sorted_by_created_at() {
    let older = SessionRecord::new("older".into(), "/work/older".into(), Source::Wsl, "Ubuntu".into())
        .with_created_at(Utc.with_ymd_and_hms(2026, 6, 3, 9, 0, 0).unwrap());
    let newer = SessionRecord::new("newer".into(), "/work/newer".into(), Source::Wsl, "Ubuntu".into())
        .with_created_at(Utc.with_ymd_and_hms(2026, 6, 3, 11, 0, 0).unwrap());
    let list = sort_sessions(vec![newer, older]);
    assert_eq!(list[0].session_id, older.session_id);
}

#[test]
fn stale_after_ten_minutes_without_update() {
    let stale = mark_stale(
        SessionRecord::new("abc".into(), "/work/a".into(), Source::Wsl, "Ubuntu".into())
            .with_updated_at(Utc::now() - Duration::minutes(11)),
    );
    assert_eq!(stale.ui_state, UiState::Stale);
}

#[test]
fn hidden_session_reappears_when_newer_event_arrives() {
    let hidden_at = Utc.with_ymd_and_hms(2026, 6, 3, 10, 0, 0).unwrap();
    let hidden = HiddenSession::new("abc123".into(), hidden_at);
    let updated_after_hidden = SessionRecord::new("abc123".into(), "/work/a".into(), Source::Wsl, "Ubuntu".into())
        .with_updated_at(hidden_at + Duration::seconds(30));
    assert!(should_show_again(&hidden, &updated_after_hidden));
}
```

- [ ] **Step 2: Implement the store and watcher**

The store should own:

- the current list of parsed sessions,
- a hidden-session map keyed by `session_id`,
- the stale threshold,
- a function that recomputes visible sessions from disk snapshots.

The watcher should:

1. Watch the `sessions/` directory.
2. Debounce bursts of file changes.
3. Reload only the changed file when possible.
4. Recompute visible state on each change.
5. Emit a Tauri event to the frontend with the current list.

Use a separate tick loop for stale detection so long-running sessions keep aging even without file writes.

- [ ] **Step 3: Run store and watcher tests**

Run:

```bash
cd src-tauri
cargo test store_watcher
```

Expected: stable ordering, stale marking, and reappearance behavior all pass.

---

### Task 5: Build the hover island UI and hide interaction

**Files:**
- Create: `src/components/Island.tsx`
- Create: `src/components/SessionPill.tsx`
- Create: `src/components/SessionList.tsx`
- Modify: `src/App.tsx`
- Modify: `src/styles.css`
- Create: `src/__tests__/Island.test.tsx`

- [ ] **Step 1: Write the failing React tests**

```tsx
const makeSession = (id: string, createdAt: string) => ({
  sessionId: id,
  title: `${id}-project`,
  status: "running" as const,
  source: "wsl" as const,
  updatedAt: createdAt,
  createdAt,
});

it("renders all sessions in created_at order", () => {
  const older = makeSession("older", "2026-06-03T09:00:00.000Z");
  const newer = makeSession("newer", "2026-06-03T11:00:00.000Z");
  render(<Island sessions={[older, newer]} />);
  expect(screen.getByText("older-project")).toBeInTheDocument();
  expect(screen.getByText("newer-project")).toBeInTheDocument();
});

it("shows +N when the collapsed strip exceeds max width", () => {
  const manySessions = Array.from({ length: 8 }, (_, index) =>
    makeSession(`session-${index}`, `2026-06-03T0${index}:00:00.000Z`)
  );
  render(<Island sessions={manySessions} />);
  expect(screen.getByText(/\+\d+/)).toBeInTheDocument();
});

it("hides a session locally when close is clicked", async () => {
  const oneSession = makeSession("one", "2026-06-03T09:00:00.000Z");
  render(<Island sessions={[oneSession]} />);
  await user.click(screen.getByRole("button", { name: /hide/i }));
  expect(screen.queryByText("one-project")).not.toBeInTheDocument();
});
```

- [ ] **Step 2: Implement the collapsed and expanded views**

The collapsed strip should:

- stay centered at the top,
- render one pill per visible session,
- keep pill order stable by `created_at`,
- animate only color and breathing, not position,
- collapse to `+N` only when the strip would overflow the screen.

The expanded panel should:

- appear on hover,
- show project title, status, source, last update, and hide button,
- keep the same order as the collapsed strip,
- never show prompt text.

- [ ] **Step 3: Wire the hide interaction to Rust**

When the user clicks hide, call a Tauri command that records the session as hidden in the Rust store. The frontend should remove it immediately from the local view, then wait for the backend state refresh to confirm the current visible list.

- [ ] **Step 4: Run the component tests**

Run:

```bash
pnpm test
```

Expected: collapsed rendering, hover expansion, `+N` truncation, and hide behavior all pass.

---

### Task 6: Add hook installation guidance and setup UX

**Files:**
- Create: `src-tauri/src/install.rs`
- Modify: `src-tauri/src/main.rs`
- Create: `src/components/SetupPanel.tsx`
- Modify: `src/App.tsx`

- [ ] **Step 1: Write the failing install tests**

```rust
#[test]
fn generates_separate_windows_and_wsl_snippets() {
    let snippets = build_install_snippets("/opt/codex-island", "C:\\Users\\zk\\AppData\\Local\\CodexIsland");
    assert!(snippets.windows.contains("py -3"));
    assert!(snippets.wsl.contains("/mnt/c/Users/zk/AppData/Local/CodexIsland"));
}
```

- [ ] **Step 2: Implement the setup helper**

The app should generate two explicit hook snippets:

1. A Windows snippet that points to the installed hook helper.
2. A WSL snippet that points to the same logical helper through the Windows-mounted path.

The setup UI should explain that the user must still trust Codex hooks before activation. It must not overwrite existing configuration without confirmation.

- [ ] **Step 3: Render setup state in the app**

If the app cannot find any session files, show a minimal setup panel with copyable snippets and a status hint, then return to the island view once sessions appear.

---

### Task 7: Verify end-to-end behavior and documentation

**Files:**
- Modify: `README.md`
- Create: `docs/superpowers/plans/2026-06-03-codex-island-verification.md` if manual notes are needed

- [ ] **Step 1: Run the full build and test set**

Run:

```bash
cd src-tauri
cargo test
cd ..
pnpm test
pnpm build
```

Expected: all unit and component tests pass, and the frontend builds.

- [ ] **Step 2: Validate the manual scenarios from the spec**

Check these cases in the running app:

1. WSL session appears as a blue pill.
2. Stop event turns it green without moving it.
3. Permission request turns it yellow.
4. Ten minutes with no update turns it gray.
5. Windows-native session appears beside it.
6. Hover shows the expanded detail panel.
7. Hide removes a pill from the UI.
8. A newer hook event brings back a hidden session.
9. App restart keeps the same created-at order.

- [ ] **Step 3: Update README with first-run instructions**

Document:

- how to launch the app,
- where the state files live,
- how the two hook snippets are installed,
- how to trust the hooks,
- how to interpret the five UI states.
