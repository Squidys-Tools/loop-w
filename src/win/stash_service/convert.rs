//! WINDOWPLACEMENT <-> StashPlacement conversion for persisted records.

use windows::Win32::UI::WindowsAndMessaging::*;

use crate::core::rect::Rect;
use crate::core::stash::{StashPlacement, StashPoint, StashRect};
use crate::win::native;

pub(super) fn placement_to_stash(placement: &WINDOWPLACEMENT) -> StashPlacement {
    StashPlacement {
        length: placement.length as i32,
        flags: placement.flags.0,
        show_command: placement.showCmd,
        min_position: StashPoint {
            x: placement.ptMinPosition.x,
            y: placement.ptMinPosition.y,
        },
        max_position: StashPoint {
            x: placement.ptMaxPosition.x,
            y: placement.ptMaxPosition.y,
        },
        normal_position: StashRect::from(native::rect_from_native(placement.rcNormalPosition)),
    }
}

pub(super) fn placement_from_stash(placement: &StashPlacement) -> WINDOWPLACEMENT {
    let normal: Rect = placement.normal_position.into();
    WINDOWPLACEMENT {
        // Preserve the stored Length when present (C# ToNative semantics).
        length: if placement.length != 0 {
            placement.length as u32
        } else {
            core::mem::size_of::<WINDOWPLACEMENT>() as u32
        },
        flags: WINDOWPLACEMENT_FLAGS(placement.flags),
        showCmd: placement.show_command,
        ptMinPosition: windows::Win32::Foundation::POINT {
            x: placement.min_position.x,
            y: placement.min_position.y,
        },
        ptMaxPosition: windows::Win32::Foundation::POINT {
            x: placement.max_position.x,
            y: placement.max_position.y,
        },
        rcNormalPosition: native::rect_to_native(normal),
    }
}
