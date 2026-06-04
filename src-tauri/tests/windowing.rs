use codex_island_lib::windowing::{
    anchored_position, centered_floating_position, floating_position, frame_for_layout,
    initial_position_for_layout, layout_for, nearest_edge, primary_mouse_release_is_pending,
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
fn snaps_to_the_right_edge_when_window_overshoots_the_snap_band() {
    let work_area = Rect {
        x: 0,
        y: 0,
        width: 2048,
        height: 1104,
    };
    let window = WindowFrame {
        x: 1960,
        y: 462,
        width: 176,
        height: 35,
    };

    assert_eq!(nearest_edge(window, work_area), Some(SnapEdge::Right));
}

#[test]
fn snaps_side_capsule_to_the_right_when_dragged_near_the_visible_edge() {
    let work_area = Rect {
        x: 0,
        y: 0,
        width: 2560,
        height: 1380,
    };
    let window = WindowFrame {
        x: 2341,
        y: 578,
        width: 44,
        height: 220,
    };

    assert_eq!(nearest_edge(window, work_area), Some(SnapEdge::Right));
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
fn side_edges_take_priority_over_top_near_corners() {
    let work_area = Rect {
        x: 0,
        y: 0,
        width: 1920,
        height: 1080,
    };
    let window = WindowFrame {
        x: 30,
        y: 10,
        width: 220,
        height: 44,
    };

    assert_eq!(nearest_edge(window, work_area), Some(SnapEdge::Left));
}

#[test]
fn left_half_stays_floating_without_entering_an_edge_band() {
    let work_area = Rect {
        x: 0,
        y: 0,
        width: 1920,
        height: 1080,
    };
    let window = WindowFrame {
        x: 600,
        y: 200,
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
        initial_position_for_layout(work_area, layout, SnapEdge::Top,),
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
        anchored_position(current, work_area, layout, SnapEdge::Right),
        (1490, 280)
    );
    assert_eq!(
        anchored_position(current, work_area, layout, SnapEdge::Left),
        (0, 280)
    );
}

#[test]
fn right_expand_frame_updates_position_and_size_together() {
    let work_area = Rect {
        x: 0,
        y: 0,
        width: 2560,
        height: 1380,
    };
    let current = WindowFrame {
        x: 2516,
        y: 556,
        width: 44,
        height: 220,
    };

    assert_eq!(
        frame_for_layout(
            current,
            work_area,
            WindowMode::IslandExpanded,
            Some(SnapEdge::Right),
            false,
        ),
        WindowFrame {
            x: 2130,
            y: 406,
            width: 430,
            height: 520,
        }
    );
}

#[test]
fn right_collapse_frame_updates_position_and_size_together() {
    let work_area = Rect {
        x: 0,
        y: 0,
        width: 2560,
        height: 1380,
    };
    let current = WindowFrame {
        x: 2130,
        y: 406,
        width: 430,
        height: 520,
    };

    assert_eq!(
        frame_for_layout(
            current,
            work_area,
            WindowMode::Island,
            Some(SnapEdge::Right),
            false,
        ),
        WindowFrame {
            x: 2516,
            y: 556,
            width: 44,
            height: 220,
        }
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

#[test]
fn floating_expand_frame_keeps_the_status_island_center_stable() {
    let work_area = Rect {
        x: 0,
        y: 0,
        width: 2560,
        height: 1380,
    };
    let current = WindowFrame {
        x: 1000,
        y: 400,
        width: 220,
        height: 44,
    };

    let next = frame_for_layout(current, work_area, WindowMode::IslandExpanded, None, false);

    assert_eq!(
        centered_floating_position(current, layout_for(WindowMode::IslandExpanded, None),),
        (915, 162),
    );
    assert_eq!(
        (next.x + next.width / 2, next.y + next.height / 2),
        (
            current.x + current.width / 2,
            current.y + current.height / 2
        ),
    );
}

#[test]
fn floating_collapse_frame_keeps_the_status_island_center_stable() {
    let work_area = Rect {
        x: 0,
        y: 0,
        width: 2560,
        height: 1380,
    };
    let current = WindowFrame {
        x: 915,
        y: 162,
        width: 390,
        height: 520,
    };

    let next = frame_for_layout(current, work_area, WindowMode::Island, None, false);

    assert_eq!(
        (next.x + next.width / 2, next.y + next.height / 2),
        (
            current.x + current.width / 2,
            current.y + current.height / 2
        ),
    );
}

#[test]
fn drag_release_wait_finishes_when_command_starts_after_mouse_is_already_up() {
    assert!(!primary_mouse_release_is_pending(false));
}

#[test]
fn drag_release_wait_keeps_waiting_while_mouse_is_still_down() {
    assert!(primary_mouse_release_is_pending(true));
}
