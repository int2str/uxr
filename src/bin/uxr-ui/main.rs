mod gui_device;
mod viewport;

use std::io::IsTerminal;
use std::path::Path;

use anyhow::Result;
use macroquad::prelude::*;

use gui_device::GuiDevice;
use viewport::{calculate_viewport, map_mouse_to_emulator};

use uxr::shared::file_io::read_file_or_stdin;
use uxr::uxn::vm::Vm;

fn window_conf() -> Conf {
    let title = std::env::args()
        .nth(1)
        .map(|path| {
            Path::new(&path)
                .file_name()
                .map(|name| format!("uxr - {}", name.to_string_lossy()))
                .unwrap_or_else(|| "uxr".to_string())
        })
        .unwrap_or_else(|| "uxr".to_string());

    Conf {
        window_title: title,
        window_width: 512,
        window_height: 320,
        window_resizable: true,
        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
async fn main() -> Result<()> {
    let arguments: Vec<String> = std::env::args().collect();
    if arguments.len() <= 1 && std::io::stdin().is_terminal() {
        eprintln!("Usage: uxr-ui <rom-file>");
        return Ok(());
    }

    let rom_data = read_file_or_stdin()?;

    let device = GuiDevice::new();
    let mut vm = Vm::new_with_rom_and_device(&rom_data, device)?;

    // Execute initial reset vector (0x0100)
    vm.run()?;
    show_mouse(false);

    if vm.device.is_terminated() {
        if vm.device.exit_code() != 0 {
            std::process::exit(vm.device.exit_code() as i32);
        }
        return Ok(());
    }

    let mut current_width = vm.device.width();
    let mut current_height = vm.device.height();

    let mut image = Image::gen_image_color(current_width, current_height, BLACK);
    vm.device.render_to_rgba(&mut image.bytes);
    let mut texture = Texture2D::from_image(&image);
    texture.set_filter(FilterMode::Nearest);

    let mut last_mouse_x = u16::MAX;
    let mut last_mouse_y = u16::MAX;

    show_mouse(false);

    loop {
        show_mouse(false);

        // Keyboard / Controller input
        let controller_vector = vm.device.controller_vector();

        if is_key_pressed(KeyCode::Escape) {
            if controller_vector != 0 {
                vm.device.set_controller_key(0x1b);
                vm.eval_vector(controller_vector)?;
            } else {
                break;
            }
        }

        let mut buttons = 0u8;
        if is_key_down(KeyCode::LeftControl) || is_key_down(KeyCode::RightControl) {
            buttons |= 0x01;
        }
        if is_key_down(KeyCode::LeftAlt) || is_key_down(KeyCode::RightAlt) {
            buttons |= 0x02;
        }
        if is_key_down(KeyCode::LeftShift) || is_key_down(KeyCode::RightShift) {
            buttons |= 0x04;
        }
        if is_key_down(KeyCode::Home) {
            buttons |= 0x08;
        }
        if is_key_down(KeyCode::Up) {
            buttons |= 0x10;
        }
        if is_key_down(KeyCode::Down) {
            buttons |= 0x20;
        }
        if is_key_down(KeyCode::Left) {
            buttons |= 0x40;
        }
        if is_key_down(KeyCode::Right) {
            buttons |= 0x80;
        }
        vm.device.set_controller_buttons(buttons);

        while let Some(character) = get_char_pressed() {
            let key_byte = character as u32;
            if key_byte <= 0xff && key_byte != 0x1b && controller_vector != 0 {
                vm.device.set_controller_key(key_byte as u8);
                vm.eval_vector(controller_vector)?;
            }
        }

        let special_keys = [
            (KeyCode::Backspace, 0x08u8),
            (KeyCode::Tab, 0x09u8),
            (KeyCode::Enter, 0x0au8),
        ];
        for (key_code, ascii_value) in special_keys {
            if is_key_pressed(key_code) && controller_vector != 0 {
                vm.device.set_controller_key(ascii_value);
                vm.eval_vector(controller_vector)?;
            }
        }

        let window_width = screen_width();
        let window_height = screen_height();
        let device_width = vm.device.width() as f32;
        let device_height = vm.device.height() as f32;
        let viewport = calculate_viewport(window_width, window_height, device_width, device_height);

        // 1. Mouse coordinates
        let (raw_mouse_x, raw_mouse_y) = mouse_position();
        let (emulator_mouse_x, emulator_mouse_y) = map_mouse_to_emulator(
            raw_mouse_x,
            raw_mouse_y,
            &viewport,
            device_width,
            device_height,
        );

        let mouse_moved = emulator_mouse_x != last_mouse_x || emulator_mouse_y != last_mouse_y;
        if mouse_moved {
            last_mouse_x = emulator_mouse_x;
            last_mouse_y = emulator_mouse_y;
            vm.device
                .set_mouse_position(emulator_mouse_x, emulator_mouse_y);
        }

        // 2. Mouse buttons
        let mut button_changed = false;
        // Left button: 0x01
        if is_mouse_button_pressed(MouseButton::Left) {
            vm.device.set_mouse_button_down(0x01);
            button_changed = true;
        } else if is_mouse_button_released(MouseButton::Left) {
            vm.device.set_mouse_button_up(0x01);
            button_changed = true;
        }
        // Middle button: 0x02
        if is_mouse_button_pressed(MouseButton::Middle) {
            vm.device.set_mouse_button_down(0x02);
            button_changed = true;
        } else if is_mouse_button_released(MouseButton::Middle) {
            vm.device.set_mouse_button_up(0x02);
            button_changed = true;
        }
        // Right button: 0x04
        if is_mouse_button_pressed(MouseButton::Right) {
            vm.device.set_mouse_button_down(0x04);
            button_changed = true;
        } else if is_mouse_button_released(MouseButton::Right) {
            vm.device.set_mouse_button_up(0x04);
            button_changed = true;
        }

        // 3. Mouse wheel
        let (wheel_x, wheel_y) = mouse_wheel();
        let wheel_moved = wheel_x != 0.0 || wheel_y != 0.0;
        if wheel_moved {
            let sx = if wheel_x > 0.0 {
                1
            } else if wheel_x < 0.0 {
                -1
            } else {
                0
            };
            let sy = if wheel_y > 0.0 {
                -1
            } else if wheel_y < 0.0 {
                1
            } else {
                0
            };
            vm.device.set_mouse_scroll(sx, sy);
        }

        // Evaluate mouse vector if mouse moved, button changed, or wheel scrolled
        if mouse_moved || button_changed || wheel_moved {
            let mouse_vector = vm.device.mouse_vector();
            if mouse_vector != 0 {
                vm.eval_vector(mouse_vector)?;
            }
            if wheel_moved {
                vm.device.clear_mouse_scroll();
            }
        }

        // 4. Screen vector (evaluated each frame, ~60 FPS)
        let screen_vector = vm.device.screen_vector();
        if screen_vector != 0 {
            vm.eval_vector(screen_vector)?;
        }

        if vm.device.is_terminated() {
            break;
        }

        // 5. Check if screen size changed
        if vm.device.size_changed()
            || vm.device.width() != current_width
            || vm.device.height() != current_height
        {
            current_width = vm.device.width();
            current_height = vm.device.height();
            image = Image::gen_image_color(current_width, current_height, BLACK);
            texture = Texture2D::from_image(&image);
            texture.set_filter(FilterMode::Nearest);
            request_new_screen_size(current_width as f32, current_height as f32);
            vm.device.clear_size_changed();
        }

        // 6. Blit and draw texture
        if vm.device.needs_redraw() {
            vm.device.render_to_rgba(&mut image.bytes);
            texture.update(&image);
            vm.device.clear_needs_redraw();
        }

        clear_background(BLACK);
        draw_texture_ex(
            &texture,
            viewport.x,
            viewport.y,
            WHITE,
            DrawTextureParams {
                dest_size: Some(vec2(viewport.width, viewport.height)),
                ..Default::default()
            },
        );

        next_frame().await;
    }

    if vm.device.is_terminated() && vm.device.exit_code() != 0 {
        std::process::exit(vm.device.exit_code() as i32);
    }

    Ok(())
}
