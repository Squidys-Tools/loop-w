//! Grow/shrink/scale/nudge steps applied to the current frame.

use crate::core::actions::WindowAction;
use crate::core::rect::Rect;

/// Grow/shrink/nudge the current frame. `dpi_scale` scales the 48px step.
pub fn manipulate_frame(work: Rect, action: WindowAction, current: Rect, dpi_scale: f64) -> Rect {
    let _ = work;
    let step = (48.0 * dpi_scale).round() as i32;
    let (mut width, mut height) = (current.width(), current.height());
    let r = Rect::new;
    match action {
        WindowAction::Larger
        | WindowAction::Smaller
        | WindowAction::ScaleUp
        | WindowAction::ScaleDown => {
            // C# `Math.Max(32, dim / 10)` per axis (truncation matches).
            let (step_w, step_h) = ((width / 10).max(32), (height / 10).max(32));
            match action {
                WindowAction::ScaleUp | WindowAction::ScaleDown => {
                    let scale = if action == WindowAction::ScaleUp {
                        1.1
                    } else {
                        0.9
                    };
                    width = (width as f64 * scale).round() as i32;
                    height = (height as f64 * scale).round() as i32;
                }
                _ => {
                    width = if action == WindowAction::Larger {
                        width + step_w
                    } else {
                        width - step_w
                    };
                    height = if action == WindowAction::Larger {
                        height + step_h
                    } else {
                        height - step_h
                    };
                }
            }
            width = width.max(step);
            height = height.max(step);
            let cx = current.left + current.width() / 2;
            let cy = current.top + current.height() / 2;
            r(
                cx - width / 2,
                cy - height / 2,
                cx - width / 2 + width,
                cy - height / 2 + height,
            )
        }
        WindowAction::GrowLeft => r(
            current.left - step,
            current.top,
            current.right,
            current.bottom,
        ),
        WindowAction::GrowRight => r(
            current.left,
            current.top,
            current.right + step,
            current.bottom,
        ),
        WindowAction::GrowTop => r(
            current.left,
            current.top - step,
            current.right,
            current.bottom,
        ),
        WindowAction::GrowBottom => r(
            current.left,
            current.top,
            current.right,
            current.bottom + step,
        ),
        WindowAction::GrowHorizontal => r(
            current.left - step,
            current.top,
            current.right + step,
            current.bottom,
        ),
        WindowAction::GrowVertical => r(
            current.left,
            current.top - step,
            current.right,
            current.bottom + step,
        ),
        WindowAction::ShrinkLeft => r(
            (current.right - 1).min(current.left + step),
            current.top,
            current.right,
            current.bottom,
        ),
        WindowAction::ShrinkRight => r(
            current.left,
            current.top,
            (current.left + 1).max(current.right - step),
            current.bottom,
        ),
        WindowAction::ShrinkTop => r(
            current.left,
            (current.bottom - 1).min(current.top + step),
            current.right,
            current.bottom,
        ),
        WindowAction::ShrinkBottom => r(
            current.left,
            current.top,
            current.right,
            (current.top + 1).max(current.bottom - step),
        ),
        WindowAction::ShrinkHorizontal => {
            let new_width = (current.width() - 2 * step).max(1);
            let cx = current.left + current.width() / 2;
            r(
                cx - new_width / 2,
                current.top,
                cx - new_width / 2 + new_width,
                current.bottom,
            )
        }
        WindowAction::ShrinkVertical => {
            let new_height = (current.height() - 2 * step).max(1);
            let cy = current.top + current.height() / 2;
            r(
                current.left,
                cy - new_height / 2,
                current.right,
                cy - new_height / 2 + new_height,
            )
        }
        WindowAction::MoveLeft => r(
            current.left - step,
            current.top,
            current.right - step,
            current.bottom,
        ),
        WindowAction::MoveRight => r(
            current.left + step,
            current.top,
            current.right + step,
            current.bottom,
        ),
        WindowAction::MoveUp => r(
            current.left,
            current.top - step,
            current.right,
            current.bottom - step,
        ),
        WindowAction::MoveDown => r(
            current.left,
            current.top + step,
            current.right,
            current.bottom + step,
        ),
        _ => current,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn larger_steps_by_tenth_clamped_to_32() {
        // C# Math.Max(32, dim / 10): a 400px window grows by 40px per side
        // pair, centered; small windows use the 32px floor.
        let work = Rect::new(0, 0, 1920, 1080);
        let grown = manipulate_frame(
            work,
            WindowAction::Larger,
            Rect::new(100, 100, 500, 400),
            1.0,
        );
        assert_eq!((grown.width(), grown.height()), (440, 332));
        let small = manipulate_frame(work, WindowAction::Larger, Rect::new(0, 0, 100, 100), 1.0);
        assert_eq!((small.width(), small.height()), (132, 132));
    }
}
