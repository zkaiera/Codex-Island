use codex_island_lib::windowing::{
    floating_position, frame_for_layout, initial_position_for_layout, layout_for, nearest_edge,
    panel_frame_for_anchor, panel_height_for_session_count, point_is_inside_frame,
    primary_mouse_release_is_pending, snapped_position, Rect, SnapEdge, WindowFrame, WindowMode,
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
fn legacy_expanded_mode_keeps_the_main_window_collapsed() {
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
            WindowMode::from_name("island_expanded"),
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
fn right_collapsed_frame_updates_position_and_size_together() {
    let work_area = Rect {
        x: 0,
        y: 0,
        width: 2560,
        height: 1380,
    };
    let current = WindowFrame {
        x: 2340,
        y: 556,
        width: 220,
        height: 44,
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
            y: 468,
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
    assert_eq!(floating_position(current), (current.x, current.y));
}

#[test]
fn top_panel_opens_below_the_stable_status_island() {
    let work_area = Rect {
        x: 0,
        y: 0,
        width: 2560,
        height: 1380,
    };
    let island = WindowFrame {
        x: 1170,
        y: 0,
        width: 220,
        height: 44,
    };

    assert_eq!(
        panel_frame_for_anchor(island, work_area, Some(SnapEdge::Top), 1),
        WindowFrame {
            x: 1085,
            y: 54,
            width: 390,
            height: 148,
        }
    );
}

#[test]
fn panel_height_tracks_visible_session_count() {
    assert_eq!(panel_height_for_session_count(1), 148);
    assert_eq!(panel_height_for_session_count(2), 228);
    assert_eq!(panel_height_for_session_count(3), 308);
    assert_eq!(panel_height_for_session_count(20), 1668);
}

#[test]
fn side_panels_open_next_to_the_vertical_status_island() {
    let work_area = Rect {
        x: 0,
        y: 0,
        width: 2560,
        height: 1380,
    };
    let left_island = WindowFrame {
        x: 0,
        y: 580,
        width: 44,
        height: 220,
    };
    let right_island = WindowFrame {
        x: 2516,
        y: 580,
        width: 44,
        height: 220,
    };

    assert_eq!(
        panel_frame_for_anchor(left_island, work_area, Some(SnapEdge::Left), 1),
        WindowFrame {
            x: 54,
            y: 616,
            width: 390,
            height: 148,
        }
    );
    assert_eq!(
        panel_frame_for_anchor(right_island, work_area, Some(SnapEdge::Right), 1),
        WindowFrame {
            x: 2116,
            y: 616,
            width: 390,
            height: 148,
        }
    );
}

#[test]
fn side_panel_height_is_limited_by_work_area() {
    let work_area = Rect {
        x: 0,
        y: 0,
        width: 2560,
        height: 1380,
    };
    let right_island = WindowFrame {
        x: 2516,
        y: 580,
        width: 44,
        height: 220,
    };

    assert_eq!(
        panel_frame_for_anchor(right_island, work_area, Some(SnapEdge::Right), 200),
        WindowFrame {
            x: 2116,
            y: 0,
            width: 390,
            height: 1380,
        }
    );
}

#[test]
fn floating_panel_uses_available_space_around_the_status_island() {
    let work_area = Rect {
        x: 0,
        y: 0,
        width: 800,
        height: 600,
    };
    let island = WindowFrame {
        x: 690,
        y: 540,
        width: 220,
        height: 44,
    };

    assert_eq!(
        panel_frame_for_anchor(island, work_area, None, 1),
        WindowFrame {
            x: 410,
            y: 382,
            width: 390,
            height: 148,
        }
    );
}

#[test]
fn floating_panel_height_is_limited_to_the_larger_vertical_slot() {
    let work_area = Rect {
        x: 0,
        y: 0,
        width: 800,
        height: 600,
    };
    let island = WindowFrame {
        x: 290,
        y: 260,
        width: 220,
        height: 44,
    };

    assert_eq!(
        panel_frame_for_anchor(island, work_area, None, 20),
        WindowFrame {
            x: 205,
            y: 314,
            width: 390,
            height: 286,
        }
    );
}

#[test]
fn pointer_hit_testing_treats_window_edges_as_inside() {
    let frame = WindowFrame {
        x: 100,
        y: 200,
        width: 44,
        height: 220,
    };

    assert!(point_is_inside_frame((100, 200), frame));
    assert!(point_is_inside_frame((143, 419), frame));
    assert!(!point_is_inside_frame((144, 419), frame));
    assert!(!point_is_inside_frame((143, 420), frame));
}

#[test]
fn drag_release_wait_finishes_when_command_starts_after_mouse_is_already_up() {
    assert!(!primary_mouse_release_is_pending(false));
}

#[test]
fn drag_release_wait_keeps_waiting_while_mouse_is_still_down() {
    assert!(primary_mouse_release_is_pending(true));
}
