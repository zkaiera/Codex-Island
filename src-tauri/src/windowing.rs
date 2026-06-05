use serde::{Deserialize, Serialize};
use std::time::Duration;
use tauri::{
    AppHandle, Manager, PhysicalPosition, PhysicalSize, Position, Runtime, Size, WebviewWindow,
};

const SIDE_SNAP_BAND_PX: i32 = 192;
const TOP_SNAP_BAND_PX: i32 = 72;
pub const PANEL_WIDTH_PX: i32 = 390;
pub const PANEL_HEIGHT_PX: i32 = 520;
const PANEL_GAP_PX: i32 = 10;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum SnapEdge {
    Top,
    Left,
    Right,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum WindowMode {
    Island,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct WindowLayout {
    pub width: i32,
    pub height: i32,
}

impl WindowMode {
    pub fn from_name(name: &str) -> Self {
        match name {
            "island_expanded" => Self::Island,
            _ => Self::Island,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct WindowFrame {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

pub fn snap_main_window<R: Runtime>(app: &AppHandle<R>) -> Option<SnapEdge> {
    let window = app.get_webview_window("main")?;
    let position = window.outer_position().ok()?;
    let size = window.outer_size().ok()?;
    let monitor = window
        .current_monitor()
        .ok()
        .flatten()
        .or_else(|| app.primary_monitor().ok().flatten())?;
    let work_area = monitor.work_area();

    let edge = nearest_edge(
        WindowFrame {
            x: position.x,
            y: position.y,
            width: size.width as i32,
            height: size.height as i32,
        },
        Rect {
            x: work_area.position.x,
            y: work_area.position.y,
            width: work_area.size.width as i32,
            height: work_area.size.height as i32,
        },
    );
    let frame = WindowFrame {
        x: position.x,
        y: position.y,
        width: size.width as i32,
        height: size.height as i32,
    };
    let work_area = Rect {
        x: work_area.position.x,
        y: work_area.position.y,
        width: work_area.size.width as i32,
        height: work_area.size.height as i32,
    };
    if let Some(edge) = edge {
        let next_position = snapped_position(frame, work_area, edge);
        let _ = window.set_position(Position::Physical(PhysicalPosition::new(
            next_position.0,
            next_position.1,
        )));
        Some(edge)
    } else {
        None
    }
}

pub fn apply_window_layout<R: Runtime>(
    app: &AppHandle<R>,
    mode: WindowMode,
    edge: Option<SnapEdge>,
    initial: bool,
) -> Option<()> {
    let window = app.get_webview_window("main")?;
    let position = window.outer_position().ok()?;
    let size = window.outer_size().ok()?;
    let monitor = window
        .current_monitor()
        .ok()
        .flatten()
        .or_else(|| app.primary_monitor().ok().flatten())?;
    let work_area = monitor.work_area();
    let frame = WindowFrame {
        x: position.x,
        y: position.y,
        width: size.width as i32,
        height: size.height as i32,
    };
    let work_area = Rect {
        x: work_area.position.x,
        y: work_area.position.y,
        width: work_area.size.width as i32,
        height: work_area.size.height as i32,
    };
    let next_frame = frame_for_layout(frame, work_area, mode, edge, initial);
    apply_window_frame(&window, next_frame);

    Some(())
}

pub fn frame_for_layout(
    current: WindowFrame,
    work_area: Rect,
    mode: WindowMode,
    edge: Option<SnapEdge>,
    initial: bool,
) -> WindowFrame {
    let layout = layout_for(mode, edge);
    let next_position = match edge {
        None => centered_floating_position(current, layout),
        Some(edge) if initial => initial_position_for_layout(work_area, layout, edge),
        Some(edge) => match mode {
            WindowMode::Island => docked_position(current, work_area, layout, edge),
        },
    };

    WindowFrame {
        x: next_position.0,
        y: next_position.1,
        width: layout.width,
        height: layout.height,
    }
}

pub fn apply_window_frame<R: Runtime>(window: &WebviewWindow<R>, frame: WindowFrame) {
    if apply_window_frame_atomic(window, frame) {
        return;
    }

    let _ = window.set_size(Size::Physical(PhysicalSize::new(
        frame.width as u32,
        frame.height as u32,
    )));
    let _ = window.set_position(Position::Physical(PhysicalPosition::new(frame.x, frame.y)));
}

#[cfg(windows)]
fn apply_window_frame_atomic<R: Runtime>(window: &WebviewWindow<R>, frame: WindowFrame) -> bool {
    use windows_sys::Win32::UI::WindowsAndMessaging::{SetWindowPos, SWP_NOACTIVATE, SWP_NOZORDER};

    let Ok(hwnd) = window.hwnd() else {
        return false;
    };

    unsafe {
        SetWindowPos(
            hwnd.0 as _,
            std::ptr::null_mut(),
            frame.x,
            frame.y,
            frame.width,
            frame.height,
            SWP_NOACTIVATE | SWP_NOZORDER,
        ) != 0
    }
}

#[cfg(not(windows))]
fn apply_window_frame_atomic<R: Runtime>(_window: &WebviewWindow<R>, _frame: WindowFrame) -> bool {
    false
}

pub fn layout_for(mode: WindowMode, edge: Option<SnapEdge>) -> WindowLayout {
    match (mode, edge) {
        (WindowMode::Island, Some(SnapEdge::Top) | None) => WindowLayout {
            width: 220,
            height: 44,
        },
        (WindowMode::Island, Some(SnapEdge::Left | SnapEdge::Right)) => WindowLayout {
            width: 44,
            height: 220,
        },
    }
}

pub fn initial_position_for_layout(
    work_area: Rect,
    next: WindowLayout,
    edge: SnapEdge,
) -> (i32, i32) {
    let centered_x = work_area.x + (work_area.width - next.width) / 2;
    let centered_y = work_area.y + (work_area.height - next.height) / 2;
    let max_x = work_area.x + work_area.width - next.width;
    let max_y = work_area.y + work_area.height - next.height;

    match edge {
        SnapEdge::Top => (clamp(centered_x, work_area.x, max_x), work_area.y),
        SnapEdge::Left => (work_area.x, clamp(centered_y, work_area.y, max_y)),
        SnapEdge::Right => (
            work_area.x + work_area.width - next.width,
            clamp(centered_y, work_area.y, max_y),
        ),
    }
}

pub fn docked_position(
    current: WindowFrame,
    work_area: Rect,
    next: WindowLayout,
    edge: SnapEdge,
) -> (i32, i32) {
    let current_center_x = current.x + current.width / 2;
    let current_center_y = current.y + current.height / 2;
    let max_x = work_area.x + work_area.width - next.width;
    let max_y = work_area.y + work_area.height - next.height;

    match edge {
        SnapEdge::Top => (
            clamp(current_center_x - next.width / 2, work_area.x, max_x),
            work_area.y,
        ),
        SnapEdge::Left => (
            work_area.x,
            clamp(current_center_y - next.height / 2, work_area.y, max_y),
        ),
        SnapEdge::Right => (
            work_area.x + work_area.width - next.width,
            clamp(current_center_y - next.height / 2, work_area.y, max_y),
        ),
    }
}

pub fn anchored_position(
    current: WindowFrame,
    work_area: Rect,
    next: WindowLayout,
    edge: SnapEdge,
) -> (i32, i32) {
    let current_center_x = current.x + current.width / 2;
    let current_center_y = current.y + current.height / 2;
    let max_x = work_area.x + work_area.width - next.width;
    let max_y = work_area.y + work_area.height - next.height;

    match edge {
        SnapEdge::Top => (
            clamp(current_center_x - next.width / 2, work_area.x, max_x),
            work_area.y,
        ),
        SnapEdge::Left => (
            work_area.x,
            clamp(current_center_y - next.height / 2, work_area.y, max_y),
        ),
        SnapEdge::Right => (
            work_area.x + work_area.width - next.width,
            clamp(current_center_y - next.height / 2, work_area.y, max_y),
        ),
    }
}

pub fn nearest_edge(window: WindowFrame, work_area: Rect) -> Option<SnapEdge> {
    let right = work_area.x + work_area.width;
    let window_right = window.x + window.width;

    if window.x <= work_area.x + SIDE_SNAP_BAND_PX {
        return Some(SnapEdge::Left);
    }

    if window_right >= right - SIDE_SNAP_BAND_PX {
        return Some(SnapEdge::Right);
    }

    if window.y <= work_area.y + TOP_SNAP_BAND_PX {
        return Some(SnapEdge::Top);
    }

    None
}

pub fn snapped_position(window: WindowFrame, work_area: Rect, edge: SnapEdge) -> (i32, i32) {
    docked_position(
        window,
        work_area,
        WindowLayout {
            width: window.width,
            height: window.height,
        },
        edge,
    )
}

pub fn floating_position(current: WindowFrame) -> (i32, i32) {
    (current.x, current.y)
}

pub fn panel_frame_for_anchor(
    island: WindowFrame,
    work_area: Rect,
    edge: Option<SnapEdge>,
) -> WindowFrame {
    let layout = WindowLayout {
        width: PANEL_WIDTH_PX,
        height: PANEL_HEIGHT_PX,
    };
    let island_center_x = island.x + island.width / 2;
    let island_center_y = island.y + island.height / 2;
    let max_x = work_area.x + work_area.width - layout.width;
    let max_y = work_area.y + work_area.height - layout.height;

    let (x, y) = match edge {
        Some(SnapEdge::Left) => (
            clamp(island.x + island.width + PANEL_GAP_PX, work_area.x, max_x),
            clamp(island_center_y - layout.height / 2, work_area.y, max_y),
        ),
        Some(SnapEdge::Right) => (
            clamp(island.x - PANEL_GAP_PX - layout.width, work_area.x, max_x),
            clamp(island_center_y - layout.height / 2, work_area.y, max_y),
        ),
        Some(SnapEdge::Top) | None => (
            clamp(island_center_x - layout.width / 2, work_area.x, max_x),
            clamp(island.y + island.height + PANEL_GAP_PX, work_area.y, max_y),
        ),
    };

    WindowFrame {
        x,
        y,
        width: layout.width,
        height: layout.height,
    }
}

pub fn point_is_inside_frame(point: (i32, i32), frame: WindowFrame) -> bool {
    point.0 >= frame.x
        && point.0 < frame.x + frame.width
        && point.1 >= frame.y
        && point.1 < frame.y + frame.height
}

pub fn cursor_is_inside_any_window<R: Runtime>(windows: &[WebviewWindow<R>]) -> bool {
    let Some(point) = cursor_position() else {
        return true;
    };

    windows.iter().any(|window| {
        let Ok(position) = window.outer_position() else {
            return false;
        };
        let Ok(size) = window.outer_size() else {
            return false;
        };

        point_is_inside_frame(
            point,
            WindowFrame {
                x: position.x,
                y: position.y,
                width: size.width as i32,
                height: size.height as i32,
            },
        )
    })
}

pub fn centered_floating_position(current: WindowFrame, next: WindowLayout) -> (i32, i32) {
    let current_center_x = current.x + current.width / 2;
    let current_center_y = current.y + current.height / 2;

    (
        current_center_x - next.width / 2,
        current_center_y - next.height / 2,
    )
}

pub async fn wait_for_primary_mouse_release() {
    let _ = tauri::async_runtime::spawn_blocking(wait_for_primary_mouse_release_blocking).await;
}

pub fn primary_mouse_release_is_pending(is_left_button_down: bool) -> bool {
    is_left_button_down
}

#[cfg(target_os = "windows")]
fn wait_for_primary_mouse_release_blocking() {
    use std::time::Instant;
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_LBUTTON};

    let deadline = Instant::now() + Duration::from_secs(10);

    while Instant::now() < deadline {
        let is_left_button_down = unsafe { GetAsyncKeyState(VK_LBUTTON as i32) } < 0;
        if !primary_mouse_release_is_pending(is_left_button_down) {
            break;
        }

        std::thread::sleep(Duration::from_millis(16));
    }

    std::thread::sleep(Duration::from_millis(120));
}

#[cfg(not(target_os = "windows"))]
fn wait_for_primary_mouse_release_blocking() {
    std::thread::sleep(Duration::from_millis(120));
}

#[cfg(target_os = "windows")]
fn cursor_position() -> Option<(i32, i32)> {
    use windows_sys::Win32::Foundation::POINT;
    use windows_sys::Win32::UI::WindowsAndMessaging::GetCursorPos;

    let mut point = POINT { x: 0, y: 0 };
    if unsafe { GetCursorPos(&mut point) } == 0 {
        return None;
    }

    Some((point.x, point.y))
}

#[cfg(not(target_os = "windows"))]
fn cursor_position() -> Option<(i32, i32)> {
    None
}

fn clamp(value: i32, min: i32, max: i32) -> i32 {
    value.max(min).min(max)
}
