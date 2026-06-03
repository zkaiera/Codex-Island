use codex_island_lib::windowing::{
    initial_position_for_layout, layout_for, nearest_edge, snapped_position, Rect, SnapEdge,
    WindowFrame, WindowMode,
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

    assert_eq!(nearest_edge(window, work_area), SnapEdge::Top);
    assert_eq!(snapped_position(window, work_area, SnapEdge::Top), (860, -22));
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
        x: 1780,
        y: 1060,
        width: 44,
        height: 220,
    };

    assert_eq!(nearest_edge(window, work_area), SnapEdge::Right);
    assert_eq!(
        snapped_position(window, work_area, SnapEdge::Right),
        (1898, 860)
    );
}

#[test]
fn initial_top_layout_is_centered_on_the_work_area() {
    let work_area = Rect {
        x: 0,
        y: 0,
        width: 1920,
        height: 1080,
    };
    let layout = layout_for(WindowMode::Island, SnapEdge::Top);

    assert_eq!(
        initial_position_for_layout(work_area, layout, SnapEdge::Top),
        (850, -22)
    );
}
