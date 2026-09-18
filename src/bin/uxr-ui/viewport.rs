#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Viewport {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

/// Computes a centered, aspect-ratio-preserving viewport within the given window dimensions.
pub fn calculate_viewport(
    window_width: f32,
    window_height: f32,
    target_width: f32,
    target_height: f32,
) -> Viewport {
    if window_width <= 0.0 || window_height <= 0.0 || target_width <= 0.0 || target_height <= 0.0 {
        return Viewport {
            x: 0.0,
            y: 0.0,
            width: window_width.max(0.0),
            height: window_height.max(0.0),
        };
    }

    let scale = (window_width / target_width).min(window_height / target_height);
    let width = (target_width * scale).floor();
    let height = (target_height * scale).floor();
    let x = ((window_width - width) * 0.5).floor();
    let y = ((window_height - height) * 0.5).floor();

    Viewport {
        x,
        y,
        width,
        height,
    }
}

/// Maps window-space mouse coordinates to emulator screen coordinates,
/// accounting for viewport offset, scaling, and clamping out-of-bounds clicks in letterboxes.
pub fn map_mouse_to_emulator(
    raw_x: f32,
    raw_y: f32,
    viewport: &Viewport,
    device_width: f32,
    device_height: f32,
) -> (u16, u16) {
    if viewport.width <= 0.0
        || viewport.height <= 0.0
        || device_width <= 0.0
        || device_height <= 0.0
    {
        return (0, 0);
    }

    let relative_x = raw_x - viewport.x;
    let relative_y = raw_y - viewport.y;

    let emulator_x =
        (relative_x * (device_width / viewport.width)).clamp(0.0, device_width - 1.0) as u16;
    let emulator_y =
        (relative_y * (device_height / viewport.height)).clamp(0.0, device_height - 1.0) as u16;

    (emulator_x, emulator_y)
}
