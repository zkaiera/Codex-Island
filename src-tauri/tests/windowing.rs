use codex_island_lib::windowing::{
    nearest_edge, snapped_position, Rect, SnapEdge, WindowFrame,
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
        width: 180,
        height: 48,
    };

    assert_eq!(nearest_edge(window, work_area), SnapEdge::Top);
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
        x: 1780,
        y: 1060,
        width: 180,
        height: 48,
    };

    assert_eq!(nearest_edge(window, work_area), SnapEdge::Right);
    assert_eq!(
        snapped_position(window, work_area, SnapEdge::Right),
        (1740, 1032)
    );
}
