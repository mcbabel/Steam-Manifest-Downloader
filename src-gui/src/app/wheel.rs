use egui::{vec2, Event, MouseWheelUnit, Vec2};
use egui_taffy::WheelSteps;
use std::sync::OnceLock;

const POINTS_PER_TOUCHPAD_UNIT: f32 = 4.0;
const POINTS_PER_TOUCHPAD_LINE: f32 = 40.0;
const COMPOSITORS_WITH_TEN_UNITS_PER_NOTCH: [&str; 4] = ["gnome", "pantheon", "cinnamon", "weston"];

pub fn notch_step(view: f32) -> f32 {
    if view < 1.0 {
        return 40.0;
    }
    f64::from(view.floor()).powf(2.0 / 3.0).floor() as f32
}

pub fn ticks_per_notch(wayland: bool, desktop: &str) -> f32 {
    let desktop = desktop.to_ascii_lowercase();
    if wayland && !COMPOSITORS_WITH_TEN_UNITS_PER_NOTCH.iter().any(|name| desktop.contains(name)) {
        1.5
    } else {
        1.0
    }
}

fn session_ticks_per_notch() -> f32 {
    static TICKS: OnceLock<f32> = OnceLock::new();
    *TICKS.get_or_init(|| {
        let set = |name: &str| std::env::var(name).is_ok_and(|value| !value.is_empty());
        let desktop: Vec<String> = ["XDG_CURRENT_DESKTOP", "XDG_SESSION_DESKTOP", "DESKTOP_SESSION"].iter().filter_map(|name| std::env::var(name).ok()).collect();
        ticks_per_notch(set("WAYLAND_DISPLAY") || set("WAYLAND_SOCKET"), &desktop.join(":"))
    })
}

fn notched(delta: Vec2) -> bool {
    let whole = |value: f32| (value - value.round()).abs() < 0.01;
    whole(delta.x) && whole(delta.y)
}

pub fn movement(delta: Vec2, shift: bool, view: Vec2, ticks: f32) -> (Vec2, Vec2) {
    let turned = |value: Vec2| if shift { value.yx() } else { value };
    if notched(delta) {
        (turned(-delta * ticks * vec2(notch_step(view.x), notch_step(view.y))), Vec2::ZERO)
    } else {
        (Vec2::ZERO, turned(delta * POINTS_PER_TOUCHPAD_LINE))
    }
}

pub fn touchpad_like_webkitgtk(events: &mut [Event]) {
    for event in events {
        if let Event::MouseWheel { unit: MouseWheelUnit::Point, delta, .. } = event {
            *delta *= POINTS_PER_TOUCHPAD_UNIT;
        }
    }
}

pub fn frame(ctx: &egui::Context) {
    let view = ctx.content_rect().size();
    let ticks = session_ticks_per_notch();
    let (mut steps, mut direct) = (Vec2::ZERO, Vec2::ZERO);
    ctx.input(|input| {
        for event in &input.events {
            if let Event::MouseWheel { unit: MouseWheelUnit::Line, delta, modifiers, .. } = event {
                let (step, moved) = movement(*delta, modifiers.shift, view, ticks);
                steps += step;
                direct += moved;
            }
        }
    });
    if direct != Vec2::ZERO {
        ctx.input_mut(|input| input.smooth_scroll_delta += direct);
    }
    ctx.data_mut(|data| {
        if steps == Vec2::ZERO {
            data.remove_temp::<WheelSteps>(WheelSteps::id());
        } else {
            data.insert_temp(WheelSteps::id(), WheelSteps(steps));
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::{Modifiers, MouseWheelSource, TouchPhase};
    use egui_taffy::cubic_bezier;

    #[test]
    fn a_notch_moves_as_far_as_measured_in_webkitgtk() {
        for (view, step) in [(300.0, 44.0), (500.0, 62.0), (600.0, 71.0), (700.0, 78.0), (800.0, 86.0), (900.0, 93.0), (1280.0, 117.0)] {
            assert_eq!(notch_step(view), step, "view of {view}");
        }
        assert_eq!(notch_step(0.0), 40.0);
    }

    #[test]
    fn notches_turn_into_steps_along_their_axis() {
        let view = vec2(1280.0, 800.0);
        assert_eq!(movement(vec2(0.0, -1.0), false, view, 1.0), (vec2(0.0, 86.0), Vec2::ZERO));
        assert_eq!(movement(vec2(0.0, 2.0), false, view, 1.0), (vec2(0.0, -172.0), Vec2::ZERO));
        assert_eq!(movement(vec2(-1.0, 0.0), false, view, 1.0), (vec2(117.0, 0.0), Vec2::ZERO));
        assert_eq!(movement(vec2(0.0, -1.0), false, view, 1.5), (vec2(0.0, 129.0), Vec2::ZERO));
    }

    #[test]
    fn shift_turns_a_notch_sideways_with_the_step_of_its_own_axis() {
        assert_eq!(movement(vec2(0.0, -1.0), true, vec2(1280.0, 800.0), 1.0), (vec2(86.0, 0.0), Vec2::ZERO));
    }

    #[test]
    fn parts_of_a_line_move_at_once_like_a_touchpad() {
        let (steps, direct) = movement(vec2(0.0, -0.25), false, vec2(1280.0, 800.0), 1.0);
        assert_eq!(steps, Vec2::ZERO);
        assert_eq!(direct, vec2(0.0, -10.0));
    }

    #[test]
    fn wayland_desktops_report_a_notch_as_gtk_saw_it() {
        assert_eq!(ticks_per_notch(false, "KDE"), 1.0);
        assert_eq!(ticks_per_notch(true, "KDE"), 1.5);
        assert_eq!(ticks_per_notch(true, "Hyprland"), 1.5);
        assert_eq!(ticks_per_notch(true, ""), 1.5);
        assert_eq!(ticks_per_notch(true, "ubuntu:GNOME"), 1.0);
        assert_eq!(ticks_per_notch(true, "Pantheon"), 1.0);
    }

    #[test]
    fn touchpad_points_are_scaled_and_lines_left_alone() {
        let wheel = |unit, y| Event::MouseWheel { unit, delta: vec2(0.0, y), phase: TouchPhase::Move, source: MouseWheelSource::Unknown, modifiers: Modifiers::NONE };
        let mut events = [wheel(MouseWheelUnit::Point, -3.25), wheel(MouseWheelUnit::Line, -1.0)];
        touchpad_like_webkitgtk(&mut events);
        let moved: Vec<f32> = events.iter().map(|event| match event {
            Event::MouseWheel { delta, .. } => delta.y,
            _ => unreachable!(),
        }).collect();
        assert_eq!(moved, [-13.0, -1.0]);
    }

    #[test]
    fn the_glide_follows_the_curves_of_webkit() {
        let ease = [0.25, 0.1, 0.25, 1.0];
        let ease_out = [0.0, 0.0, 0.58, 1.0];
        assert_eq!(cubic_bezier(ease, 0.0), 0.0);
        assert_eq!(cubic_bezier(ease, 1.0), 1.0);
        assert!((cubic_bezier(ease, 0.5) - 0.8024).abs() < 0.001);
        assert!((cubic_bezier(ease, 0.25) - 0.4085).abs() < 0.001);
        assert!((cubic_bezier(ease_out, 0.5) - 0.6846).abs() < 0.001);
    }
}
