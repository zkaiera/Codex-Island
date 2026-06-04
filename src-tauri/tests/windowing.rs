use codex_island_lib::windowing::{
    anchored_position, floating_position, initial_position_for_layout, layout_for, nearest_edge,
    snapped_position, Rect, SnapEdge, WindowFrame, WindowMode,
};

#[test]
fn snaps_to_the_nearest_top_edge() {
    let work_area = Rect {
        x: 0,
        y: 0,
        width: 1920,
        height: 1080,
    };
    let window = WindowFrame {
        x: 860,
        y: 12,
        width: 220,
        height: 44,
    };

    assert_eq!(nearest_edge(window, work_area), Some(SnapEdge::Top));
    assert_eq!(snapped_position(window, work_area, SnapEdge::Top), (860, 0));
}

#[test]
fn snaps_to_the_nearest_right_edge_and_clamps_y() {
    let work_area = Rect {
        x: 0,
        y: 0,
        width: 1920,
        height: 1080,
    };
    let window = WindowFrame {
        x: 1810,
        y: 1060,
        width: 44,
        height: 220,
    };

    assert_eq!(nearest_edge(window, work_area), Some(SnapEdge::Right));
    assert_eq!(
        snapped_position(window, work_area, SnapEdge::Right),
        (1876, 860)
    );
}

#[test]
fn does_not_snap_when_window_is_outside_the_snap_band() {
    let work_area = Rect {
        x: 0,
        y: 0,
        width: 1920,
        height: 1080,
    };
    let window = WindowFrame {
        x: 820,
        y: 90,
        width: 220,
        height: 44,
    };

    assert_eq!(nearest_edge(window, work_area), None);
}

#[test]
fn side_collapsed_layout_stays_fully_inside_screen() {
    let work_area = Rect {
        x: 0,
        y: 0,
        width: 1920,
        height: 1080,
    };
    let current = WindowFrame {
        x: 80,
        y: 430,
        width: 44,
        height: 220,
    };
    let layout = layout_for(WindowMode::Island, Some(SnapEdge::Left));

    assert_eq!(
        snapped_position(current, work_area, SnapEdge::Left),
        (0, 430)
    );
    assert_eq!(
        snapped_position(current, work_area, SnapEdge::Right),
        (1876, 430)
    );
    assert_eq!(layout.width, 44);
}

#[test]
fn initial_top_layout_is_centered_on_the_work_area() {
    let work_area = Rect {
        x: 0,
        y: 0,
        width: 1920,
        height: 1080,
    };
    let layout = layout_for(WindowMode::Island, Some(SnapEdge::Top));

    assert_eq!(
        initial_position_for_layout(
            work_area,
            layout,
            Some(SnapEdge::Top),
            WindowFrame {
                x: 0,
                y: 0,
                width: layout.width,
                height: layout.height,
            },
        ),
        (850, 0)
    );
}

#[test]
fn expanded_side_layout_anchors_to_screen_edge_without_gap() {
    let work_area = Rect {
        x: 0,
        y: 0,
        width: 1920,
        height: 1080,
    };
    let current = WindowFrame {
        x: 1898,
        y: 430,
        width: 44,
        height: 220,
    };
    let layout = layout_for(WindowMode::IslandExpanded, Some(SnapEdge::Right));

    assert_eq!(
        anchored_position(current, work_area, layout, Some(SnapEdge::Right)),
        (1490, 280)
    );
    assert_eq!(
        anchored_position(current, work_area, layout, Some(SnapEdge::Left)),
        (0, 280)
    );
}

#[test]
fn floating_layout_keeps_the_current_window_position() {
    let current = WindowFrame {
        x: 512,
        y: 278,
        width: 220,
        height: 44,
    };

    assert_eq!(layout_for(WindowMode::Island, None).width, 220);
    assert_eq!(layout_for(WindowMode::IslandExpanded, None).width, 390);
    assert_eq!(floating_position(current), (current.x, current.y));
}
