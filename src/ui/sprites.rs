use ratatui::style::Color;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnimState {
    Idle,
    Typing,
    Reading,
    Walking,
}

/// Get sprite frame (3 lines) for a given animation state and frame index.
pub fn sprite_frame(state: AnimState, frame: usize) -> [&'static str; 3] {
    match state {
        AnimState::Idle => IDLE_FRAMES[frame % IDLE_FRAMES.len()],
        AnimState::Typing => TYPING_FRAMES[frame % TYPING_FRAMES.len()],
        AnimState::Reading => READING_FRAMES[frame % READING_FRAMES.len()],
        AnimState::Walking => WALKING_FRAMES[frame % WALKING_FRAMES.len()],
    }
}

const IDLE_FRAMES: &[[&str; 3]] = &[
    [
        " \u{25c9} ",
        "\u{2554}\u{2551}\u{2557}",
        "\u{255a}\u{2569}\u{255d}",
    ],
    [" \u{25c9} ", "\u{2554}\u{2551}\u{2557}", " \u{2551} "],
];

const TYPING_FRAMES: &[[&str; 3]] = &[
    [
        " \u{25c9} ",
        "\u{2554}\u{2551}\u{2557}",
        "\u{255a}\u{2569}\u{255d}",
    ],
    [
        " \u{25c9} ",
        "\u{2554}\u{2551}~",
        "\u{255a}\u{2569}\u{255d}",
    ],
    [
        " \u{25c9} ",
        "~\u{2551}\u{2557}",
        "\u{255a}\u{2569}\u{255d}",
    ],
];

const READING_FRAMES: &[[&str; 3]] = &[
    [
        " \u{25c9} ",
        "\u{2554}\u{2551}\u{2590}",
        "\u{255a}\u{2569}\u{255d}",
    ],
    [
        " \u{25c9} ",
        "\u{2554}\u{2551}\u{2590}",
        "\u{255a}\u{2569}\u{255d}",
    ],
];

const WALKING_FRAMES: &[[&str; 3]] = &[
    [
        " \u{25c9} ",
        "\u{2554}\u{2551}\u{2557}",
        "\u{255d} \u{255a}",
    ],
    [
        " \u{25c9} ",
        "\u{2554}\u{2551}\u{2557}",
        "\u{255a} \u{255d}",
    ],
];

pub const DESK: [&str; 2] = [
    "\u{2554}\u{2550}\u{2550}\u{2550}\u{2557}",
    "\u{255a}\u{2550}\u{2550}\u{2550}\u{255d}",
];

pub const AGENT_COLORS: &[Color] = &[
    Color::Cyan,
    Color::Magenta,
    Color::Yellow,
    Color::Green,
    Color::Blue,
    Color::Red,
];

pub fn agent_color(id: u32) -> Color {
    AGENT_COLORS[(id as usize).saturating_sub(1) % AGENT_COLORS.len()]
}

pub fn sub_agent_color(_parent_id: u32) -> Color {
    Color::DarkGray
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sprite_frames_have_3_lines() {
        for state in [
            AnimState::Idle,
            AnimState::Typing,
            AnimState::Reading,
            AnimState::Walking,
        ] {
            let frame = sprite_frame(state, 0);
            assert_eq!(frame.len(), 3);
        }
    }

    #[test]
    fn sprite_frames_cycle() {
        let f0 = sprite_frame(AnimState::Typing, 0);
        let f3 = sprite_frame(AnimState::Typing, 3);
        assert_eq!(f0, f3);
    }

    #[test]
    fn agent_colors_cycle() {
        assert_eq!(agent_color(1), Color::Cyan);
        assert_eq!(agent_color(7), Color::Cyan);
    }

    #[test]
    fn desk_has_correct_dimensions() {
        assert_eq!(DESK.len(), 2);
        assert_eq!(DESK[0].chars().count(), 5);
    }
}
