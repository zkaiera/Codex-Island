# Codex Island 交互打磨 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 修复悬浮闪烁、补齐浮动与边缘吸附状态，并把拖动、吸附、展开/收起过渡调顺。

**Architecture:** 前端负责 hover 区域、展开收起节奏和状态展示；Tauri Rust 负责吸附带判定、窗口尺寸与位置；CSS 负责横向、纵向和浮动三种视觉形态。实现时保持窗口几何和 UI 状态分离，避免拖拽过程里的布局抖动。

**Tech Stack:** React 19, TypeScript, Vite, Vitest, Testing Library, Tauri 2, Rust, `tauri-apps/api`, 原生 CSS。

---

### 文件结构

- `src/components/Island.tsx`：岛本体、hover 边界、拖动启动、吸附结果驱动的边框类名。
- `src/App.tsx`：前端窗口锚点状态、`set_window_mode` 参数翻译、窗口模式收敛延迟。
- `src/styles.css`：`top`、`left`、`right`、`floating` 四种视觉布局和过渡动画。
- `src/__tests__/Island.test.tsx`：hover 区域不闪烁、左右侧布局类名、展开/收起节奏回归。
- `src/__tests__/App.test.tsx`：`floating` 状态与 Tauri 命令参数的端到端翻译。
- `src-tauri/src/windowing.rs`：吸附带检测、浮动布局定位、左右侧窗口尺寸。
- `src-tauri/src/lib.rs`：把 `None` 吸附结果传到布局引擎，不再默认回退到顶部。
- `src-tauri/tests/windowing.rs`：边缘吸附、未吸附、侧边尺寸与位置回归。

### Task 1: 稳定 hover 区域，消除岛本体与面板之间的闪烁

**Files:**
- Modify: `src/components/Island.tsx`
- Modify: `src/App.tsx`
- Modify: `src/__tests__/Island.test.tsx`
- Modify: `src/__tests__/App.test.tsx`

- [ ] **Step 1: 先写会失败的 hover 回归测试**

```ts
it("鼠标从岛本体移到展开面板时不会触发收起", async () => {
  vi.useFakeTimers();
  const onExpandedChange = vi.fn();
  render(
    <Island
      sessions={[makeSession("one", "2026-06-03T09:00:00.000Z")]}
      onHide={vi.fn()}
      onExpandedChange={onExpandedChange}
    />,
  );

  const wrapper = screen.getByLabelText("Codex Island").parentElement!;
  fireEvent.pointerEnter(wrapper);
  await vi.advanceTimersByTimeAsync(100);

  const panel = screen.getByText("one-project").closest(".island-panel")!;
  fireEvent.pointerLeave(wrapper, { relatedTarget: panel });
  await vi.advanceTimersByTimeAsync(250);

  expect(screen.getByText("one-project")).toBeInTheDocument();
  expect(onExpandedChange).toHaveBeenCalledWith(true);
});
```

再补一条离开整个区域后才收起的测试，`relatedTarget` 设为 `document.body`，断言 `onExpandedChange(false)` 只在延迟结束后出现。

- [ ] **Step 2: 在 `Island.tsx` 里把 hover 判定改成“整个岛区域”**

把 hover 入口统一到一个根容器上，用 `relatedTarget` 判断指针是否仍在岛内：

```ts
function isInsideIsland(nextTarget: EventTarget | null) {
  return nextTarget instanceof Node && rootRef.current?.contains(nextTarget) === true;
}

function handlePointerLeave(event: PointerEvent<HTMLDivElement>) {
  if (isInsideIsland(event.relatedTarget)) {
    return;
  }

  queueCollapse();
}
```

同时把计时器收敛成命名常量，建议直接改成：

```ts
const EXPAND_DELAY_MS = 80;
const COLLAPSE_DELAY_MS = 180;
```

拖动开始时清掉两个 hover 计时器，拖动结束后再允许重新进入 hover 收起流程。

- [ ] **Step 3: 在 `App.tsx` 里把窗口收拢延迟和 hover 动画对齐**

把窗口模式收敛延迟单独提常量，避免收起动画结束前就先把窗口切回去：

```ts
const WINDOW_MODE_SETTLE_MS = 240;
```

`handleExpandedChange(false)` 里只做一次延迟收敛；如果 hover 在这个窗口期内重新进入，必须取消旧定时器并保持展开。

- [ ] **Step 4: 跑前端测试并确认 hover 逻辑通过**

运行：

```bash
pnpm exec vitest run src/__tests__/Island.test.tsx src/__tests__/App.test.tsx
```

预期：面板内悬停不会闪，离开整个岛区域后才收起。

- [ ] **Step 5: 提交这一段变更**

```bash
git add src/components/Island.tsx src/App.tsx src/__tests__/Island.test.tsx src/__tests__/App.test.tsx
git commit -m "fix: stabilize island hover region"
```

### Task 2: 引入 floating 状态，改成“命中吸附带才贴边”

**Files:**
- Modify: `src-tauri/src/windowing.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src-tauri/tests/windowing.rs`
- Modify: `src/App.tsx`
- Modify: `src/__tests__/App.test.tsx`

- [ ] **Step 1: 先写会失败的吸附带与 floating 测试**

```rust
#[test]
fn returns_none_when_window_is_outside_snap_band() {
    let work_area = Rect {
        x: 0,
        y: 0,
        width: 1920,
        height: 1080,
    };
    let window = WindowFrame {
        x: 760,
        y: 420,
        width: 220,
        height: 44,
    };

    assert_eq!(nearest_edge(window, work_area, 72), None);
}

#[test]
fn floating_layout_keeps_current_position_when_not_snapped() {
    let work_area = Rect {
        x: 0,
        y: 0,
        width: 1920,
        height: 1080,
    };
    let current = WindowFrame {
        x: 760,
        y: 420,
        width: 220,
        height: 44,
    };

    assert_eq!(floating_position(current, work_area), (760, 420));
}
```

再补一条前端测试：`snap_window` 返回 `null` 时，`App` 要把锚点更新为 `"floating"`，并且 `set_window_mode` 里传给后端的 `edge` 必须是 `null`。

- [ ] **Step 2: 在 Rust 里把吸附判断改成“吸附带”而不是“最近边”**

把 `nearest_edge` 改成返回 `Option<SnapEdge>`，并加上 72px 的吸附带常量：

```rust
const SNAP_BAND_PX: i32 = 72;

pub fn nearest_edge(window: WindowFrame, work_area: Rect, band: i32) -> Option<SnapEdge> {
    let distance_to_top = (window.y - work_area.y).abs();
    let distance_to_left = (window.x - work_area.x).abs();
    let distance_to_right = (work_area.x + work_area.width - (window.x + window.width)).abs();

    let mut candidate = None;
    let mut best = band + 1;

    if distance_to_top <= band && distance_to_top < best {
        best = distance_to_top;
        candidate = Some(SnapEdge::Top);
    }
    if distance_to_left <= band && distance_to_left < best {
        best = distance_to_left;
        candidate = Some(SnapEdge::Left);
    }
    if distance_to_right <= band && distance_to_right < best {
        best = distance_to_right;
        candidate = Some(SnapEdge::Right);
    }

    candidate
}
```

再把 `apply_window_layout` 改成接收 `Option<SnapEdge>`，`None` 时走浮动定位，不再默认回退到顶部贴边。

```rust
pub fn apply_window_layout<R: Runtime>(
    app: &AppHandle<R>,
    mode: WindowMode,
    edge: Option<SnapEdge>,
    initial: bool,
) -> Option<()>
```

`layout_for` 也改成接收 `Option<SnapEdge>`，这样 collapsed 和 expanded 都可以在 `None` 时返回浮动尺寸；`edge == None` 时再额外走浮动位置函数，不做顶边锚定。

```rust
pub fn layout_for(mode: WindowMode, edge: Option<SnapEdge>) -> WindowLayout
```

- [ ] **Step 3: 在前端把 `"floating"` 和 `null` 串起来**

`App.tsx` 里的锚点类型改成：

```ts
type SnapEdge = "top" | "left" | "right" | "floating";
```

发给 Tauri 时翻译成：

```ts
const edge = snapEdge === "floating" ? null : snapEdge;
void invoke("set_window_mode", { mode, edge, initial });
```

`snap_window` 解析到 `null` 时，把本地状态切到 `"floating"`，但不要改成别的边缘。

- [ ] **Step 4: 跑 Rust 和前端测试**

运行：

```bash
cd src-tauri && cargo test windowing
pnpm exec vitest run src/__tests__/App.test.tsx
```

预期：屏幕中间不再强制贴边，只有命中边缘吸附带才会变成 `top`、`left` 或 `right`。

- [ ] **Step 5: 提交这一段变更**

```bash
git add src-tauri/src/windowing.rs src-tauri/src/lib.rs src-tauri/tests/windowing.rs src/App.tsx src/__tests__/App.test.tsx
git commit -m "fix: add floating snap state"
```

### Task 3: 把浮动、顶部、左右侧的视觉布局和过渡统一起来

**Files:**
- Modify: `src/styles.css`
- Modify: `src/components/Island.tsx`
- Modify: `src/App.tsx`
- Modify: `src/__tests__/Island.test.tsx`

- [ ] **Step 1: 先写会失败的类名和方向测试**

```ts
it("浮动状态会挂上 floating 类名", () => {
  render(<Island sessions={[]} onHide={vi.fn()} snapEdge="floating" />);
  expect(screen.getByLabelText("Codex Island").parentElement).toHaveClass(
    "island-wrapper--edge-floating",
  );
});

it("左右侧状态仍然保持对应的边缘类名", () => {
  const { rerender } = render(<Island sessions={[]} onHide={vi.fn()} snapEdge="left" />);
  expect(screen.getByLabelText("Codex Island").parentElement).toHaveClass(
    "island-wrapper--edge-left",
  );

  rerender(<Island sessions={[]} onHide={vi.fn()} snapEdge="right" />);
  expect(screen.getByLabelText("Codex Island").parentElement).toHaveClass(
    "island-wrapper--edge-right",
  );
});
```

再补一条展开态测试，确认浮动状态下仍能正常展开，且不会因为类名切换把内容移出视图。

- [ ] **Step 2: 在 `styles.css` 里把三种布局写清楚**

新增或调整这三组规则：

```css
.app-shell--edge-floating {
  align-items: center;
  justify-content: center;
}

.island-wrapper--edge-floating {
  flex-direction: column;
  align-items: center;
  --panel-enter-x: 0;
  --panel-enter-y: -14px;
  --panel-origin: top center;
}

.island-wrapper--edge-left,
.island-wrapper--edge-right {
  flex-direction: row;
  align-items: center;
  gap: 10px;
}

.island-wrapper--edge-left .island-panel {
  clip-path: inset(0 0 0 100% round 22px);
  transform: translate3d(14px, 0, 0) scale(0.96);
  transform-origin: left center;
}

.island-wrapper--edge-right .island-panel {
  clip-path: inset(0 100% 0 0 round 22px);
  transform: translate3d(-14px, 0, 0) scale(0.96);
  transform-origin: right center;
}
```

同时把 `session-pill` 的纵向排列、`island` 的最小尺寸、`border-radius` 和 `transition` 统一收紧，避免横向/纵向切换时尺寸跳动。

- [ ] **Step 3: 在 `Island.tsx` 里让面板方向跟着边缘走**

`Island` 只负责根据 `snapEdge` 输出对应类名，不自己推断样式。把边缘相关的 class 集中成：

```ts
`island-wrapper--edge-${snapEdge}`
```

并确保 `floating`、`top`、`left`、`right` 都能走同一套渲染路径，只在 CSS 里分叉。

- [ ] **Step 4: 跑前端构建和组件测试**

运行：

```bash
pnpm exec vitest run src/__tests__/Island.test.tsx
pnpm build
```

预期：类名切换正常，样式编译通过，左右侧展开不再像“顶部贴边”。

- [ ] **Step 5: 提交这一段变更**

```bash
git add src/styles.css src/components/Island.tsx src/App.tsx src/__tests__/Island.test.tsx
git commit -m "fix: polish island layouts and transitions"
```

### Task 4: 做一次全量回归，补齐最后的行为检查

**Files:**
- Modify: 如有必要，仅修复前面测试暴露出来的少量文件。

- [ ] **Step 1: 跑前端全量测试**

运行：

```bash
pnpm test
```

预期：所有 React/Vitest 用例通过，尤其是 hover、floating、吸附边缘翻译相关的用例。

- [ ] **Step 2: 跑前端构建**

运行：

```bash
pnpm build
```

预期：TypeScript 和 Vite 构建都通过，没有类型漂移。

- [ ] **Step 3: 跑 Rust 全量测试**

运行：

```bash
cd src-tauri && cargo test
```

预期：`windowing`、`hook`、`store` 相关测试都保持通过，特别是 `snap_window` 在未命中吸附带时返回 `None`。

- [ ] **Step 4: 做一次手工烟雾测试**

运行：

```bash
pnpm dev -- --host 127.0.0.1 --port 1420
```

打开 `http://127.0.0.1:1420/?demo=1`，手工检查这四个场景：

1. 鼠标移到展开面板上，面板保持展开。
2. 拖到屏幕中间，窗口保持浮动，不被硬贴到边缘。
3. 拖到左/右边缘，窗口切成纵向侧边布局。
4. 左右侧展开方向朝内，拖动、吸附、展开/收起没有明显闪烁。

- [ ] **Step 5: 如果烟雾测试暴露问题，只修最小范围文件并重跑对应测试**

优先只改 `src/components/Island.tsx`、`src/App.tsx`、`src/styles.css`、`src-tauri/src/windowing.rs` 这四个文件里最小的一段，然后重跑第 1 到第 4 步的验证命令。
