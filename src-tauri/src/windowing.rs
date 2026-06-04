use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, PhysicalPosition, PhysicalSize, Position, Runtime, Size};

const SNAP_BAND_PX: i32 = 72;

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
    IslandExpanded,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct WindowLayout {
    pub width: i32,
    pub height: i32,
}

impl WindowMode {
    pub fn from_name(name: &str) -> Self {
        match name {
            "island_expanded" => Self::IslandExpanded,
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
    let layout = layout_for(mode, edge);
    let work_area = Rect {
        x: work_area.position.x,
        y: work_area.position.y,
        width: work_area.size.width as i32,
        height: work_area.size.height as i32,
    };
    let next_position = if initial {
        initial_position_for_layout(work_area, layout, edge, frame)
    } else {
        match mode {
            WindowMode::Island => docked_position(frame, work_area, layout, edge),
            WindowMode::IslandExpanded => anchored_position(frame, work_area, layout, edge),
        }
    };

    let _ = window.set_size(Size::Physical(PhysicalSize::new(
        layout.width as u32,
        layout.height as u32,
    )));
    let _ = window.set_position(Position::Physical(PhysicalPosition::new(
        next_position.0,
        next_position.1,
    )));

    Some(())
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
        (WindowMode::IslandExpanded, Some(SnapEdge::Top) | None) => WindowLayout {
            width: 390,
            height: 520,
        },
        (WindowMode::IslandExpanded, Some(SnapEdge::Left | SnapEdge::Right)) => WindowLayout {
            width: 430,
            height: 520,
        },
    }
}

pub fn initial_position_for_layout(
    work_area: Rect,
    next: WindowLayout,
    edge: Option<SnapEdge>,
    current: WindowFrame,
) -> (i32, i32) {
    let centered_x = work_area.x + (work_area.width - next.width) / 2;
    let centered_y = work_area.y + (work_area.height - next.height) / 2;
    let max_x = work_area.x + work_area.width - next.width;
    let max_y = work_area.y + work_area.height - next.height;

    match edge {
        Some(SnapEdge::Top) => (clamp(centered_x, work_area.x, max_x), work_area.y),
        Some(SnapEdge::Left) => (work_area.x, clamp(centered_y, work_area.y, max_y)),
        Some(SnapEdge::Right) => (
            work_area.x + work_area.width - next.width,
            clamp(centered_y, work_area.y, max_y),
        ),
        None => floating_position(current),
    }
}

pub fn docked_position(
    current: WindowFrame,
    work_area: Rect,
    next: WindowLayout,
    edge: Option<SnapEdge>,
) -> (i32, i32) {
    let current_center_x = current.x + current.width / 2;
    let current_center_y = current.y + current.height / 2;
    let max_x = work_area.x + work_area.width - next.width;
    let max_y = work_area.y + work_area.height - next.height;

    match edge {
        Some(SnapEdge::Top) => (
            clamp(current_center_x - next.width / 2, work_area.x, max_x),
            work_area.y,
        ),
        Some(SnapEdge::Left) => (
            work_area.x,
            clamp(current_center_y - next.height / 2, work_area.y, max_y),
        ),
        Some(SnapEdge::Right) => (
            work_area.x + work_area.width - next.width,
            clamp(current_center_y - next.height / 2, work_area.y, max_y),
        ),
        None => floating_position(current),
    }
}

pub fn anchored_position(
    current: WindowFrame,
    work_area: Rect,
    next: WindowLayout,
    edge: Option<SnapEdge>,
) -> (i32, i32) {
    let current_center_x = current.x + current.width / 2;
    let current_center_y = current.y + current.height / 2;
    let max_x = work_area.x + work_area.width - next.width;
    let max_y = work_area.y + work_area.height - next.height;

    match edge {
        Some(SnapEdge::Top) => (
            clamp(current_center_x - next.width / 2, work_area.x, max_x),
            work_area.y,
        ),
        Some(SnapEdge::Left) => (
            work_area.x,
            clamp(current_center_y - next.height / 2, work_area.y, max_y),
        ),
        Some(SnapEdge::Right) => (
            work_area.x + work_area.width - next.width,
            clamp(current_center_y - next.height / 2, work_area.y, max_y),
        ),
        None => floating_position(current),
    }
}

pub fn nearest_edge(window: WindowFrame, work_area: Rect) -> Option<SnapEdge> {
    let distance_to_top = (window.y - work_area.y).abs();
    let distance_to_left = (window.x - work_area.x).abs();
    let right = work_area.x + work_area.width;
    let distance_to_right = (right - (window.x + window.width)).abs();
    let (distance, edge) =
        if distance_to_left <= distance_to_top && distance_to_left <= distance_to_right {
            (distance_to_left, SnapEdge::Left)
        } else if distance_to_right <= distance_to_top {
            (distance_to_right, SnapEdge::Right)
        } else {
            (distance_to_top, SnapEdge::Top)
        };

    (distance <= SNAP_BAND_PX).then_some(edge)
}

pub fn snapped_position(window: WindowFrame, work_area: Rect, edge: SnapEdge) -> (i32, i32) {
    docked_position(
        window,
        work_area,
        WindowLayout {
            width: window.width,
            height: window.height,
        },
        Some(edge),
    )
}

pub fn floating_position(current: WindowFrame) -> (i32, i32) {
    (current.x, current.y)
}

fn clamp(value: i32, min: i32, max: i32) -> i32 {
    value.max(min).min(max)
}
