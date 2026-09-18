use std::fs::{File, OpenOptions};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use uxr::uxn::device::VmDevice;
use uxr::uxn::memory::Memory;

const DEFAULT_SCREEN_WIDTH: u16 = 512;
const DEFAULT_SCREEN_HEIGHT: u16 = 320;
const PADDING: usize = 16;
const HALF_PADDING: usize = 8;

const BLEND_LUT: [[[u8; 4]; 2]; 16] = [
    [[0, 0, 1, 2], [0, 0, 4, 8]],
    [[0, 1, 2, 3], [0, 4, 8, 12]],
    [[0, 2, 3, 1], [0, 8, 12, 4]],
    [[0, 3, 1, 2], [0, 12, 4, 8]],
    [[1, 0, 1, 2], [4, 0, 4, 8]],
    [[1, 1, 2, 3], [4, 4, 8, 12]],
    [[1, 2, 3, 1], [4, 8, 12, 4]],
    [[1, 3, 1, 2], [4, 12, 4, 8]],
    [[2, 0, 1, 2], [8, 0, 4, 8]],
    [[2, 1, 2, 3], [8, 4, 8, 12]],
    [[2, 2, 3, 1], [8, 8, 12, 4]],
    [[2, 3, 1, 2], [8, 12, 4, 8]],
    [[3, 0, 1, 2], [12, 0, 4, 8]],
    [[3, 1, 2, 3], [12, 4, 8, 12]],
    [[3, 2, 3, 1], [12, 8, 12, 4]],
    [[3, 3, 1, 2], [12, 12, 4, 8]],
];

#[repr(C)]
#[derive(Default, Clone, Copy)]
struct Tm {
    tm_sec: std::ffi::c_int,
    tm_min: std::ffi::c_int,
    tm_hour: std::ffi::c_int,
    tm_mday: std::ffi::c_int,
    tm_mon: std::ffi::c_int,
    tm_year: std::ffi::c_int,
    tm_wday: std::ffi::c_int,
    tm_yday: std::ffi::c_int,
    tm_isdst: std::ffi::c_int,
    tm_gmtoff: std::ffi::c_long,
    tm_zone: *const std::ffi::c_char,
}

unsafe extern "C" {
    fn localtime_r(timep: *const i64, result: *mut Tm) -> *mut Tm;
}

enum FileState {
    Idle,
    FileRead { file: File },
    FileWrite { file: File },
    DirRead { data: Vec<u8>, offset: usize },
}

struct UxnFile {
    filepath: String,
    state: FileState,
}

impl UxnFile {
    fn new() -> Self {
        Self {
            filepath: String::new(),
            state: FileState::Idle,
        }
    }

    fn reset(&mut self) {
        self.state = FileState::Idle;
    }
}

pub struct GuiDevice {
    pub(crate) dev: [u8; 256],
    pub(crate) screen_layers: Vec<u8>,
    pub(crate) screen_palette: [[u8; 4]; 16],
    pub(crate) width: u16,
    pub(crate) height: u16,
    pub(crate) x: i16,
    pub(crate) y: i16,
    pub(crate) addr: u16,
    pub(crate) auto_x: bool,
    pub(crate) auto_y: bool,
    pub(crate) auto_addr: u8,
    pub(crate) auto_len: u8,
    pub(crate) dx: u8,
    pub(crate) dy: u8,
    pub(crate) terminated: bool,
    pub(crate) exit_code: u8,
    pub(crate) needs_redraw: bool,
    pub(crate) size_changed: bool,
    files: [UxnFile; 2],
}

impl GuiDevice {
    pub fn new() -> Self {
        let mut dev = [0u8; 256];

        // Default palette: #fff, #000, #7db, #f62
        dev[0x08] = 0xf0;
        dev[0x09] = 0x7f;
        dev[0x0a] = 0xf0;
        dev[0x0b] = 0xd6;
        dev[0x0c] = 0xf0;
        dev[0x0d] = 0xb2;

        // Default screen dimensions
        dev[0x22] = (DEFAULT_SCREEN_WIDTH >> 8) as u8;
        dev[0x23] = DEFAULT_SCREEN_WIDTH as u8;
        dev[0x24] = (DEFAULT_SCREEN_HEIGHT >> 8) as u8;
        dev[0x25] = DEFAULT_SCREEN_HEIGHT as u8;

        let stride = DEFAULT_SCREEN_WIDTH as usize + PADDING;
        let total_height = DEFAULT_SCREEN_HEIGHT as usize + PADDING;
        let screen_layers = vec![0u8; stride * total_height];

        let mut device = Self {
            dev,
            screen_layers,
            screen_palette: [[0u8; 4]; 16],
            width: DEFAULT_SCREEN_WIDTH,
            height: DEFAULT_SCREEN_HEIGHT,
            x: 0,
            y: 0,
            addr: 0,
            auto_x: false,
            auto_y: false,
            auto_addr: 0,
            auto_len: 0,
            dx: 0,
            dy: 0,
            terminated: false,
            exit_code: 0,
            needs_redraw: true,
            size_changed: false,
            files: [UxnFile::new(), UxnFile::new()],
        };

        device.update_palette();
        device
    }

    pub fn width(&self) -> u16 {
        self.width
    }

    pub fn height(&self) -> u16 {
        self.height
    }

    pub fn screen_vector(&self) -> u16 {
        ((self.dev[0x20] as u16) << 8) | (self.dev[0x21] as u16)
    }

    pub fn mouse_vector(&self) -> u16 {
        ((self.dev[0x90] as u16) << 8) | (self.dev[0x91] as u16)
    }

    pub fn controller_vector(&self) -> u16 {
        ((self.dev[0x80] as u16) << 8) | (self.dev[0x81] as u16)
    }

    pub fn set_controller_key(&mut self, key: u8) {
        self.dev[0x83] = key;
    }

    pub fn set_controller_buttons(&mut self, buttons: u8) {
        self.dev[0x82] = buttons;
    }

    pub fn is_terminated(&self) -> bool {
        self.terminated
    }

    pub fn exit_code(&self) -> u8 {
        self.exit_code
    }

    pub fn needs_redraw(&self) -> bool {
        self.needs_redraw
    }

    pub fn clear_needs_redraw(&mut self) {
        self.needs_redraw = false;
    }

    pub fn size_changed(&self) -> bool {
        self.size_changed
    }

    pub fn clear_size_changed(&mut self) {
        self.size_changed = false;
    }

    pub fn set_mouse_position(&mut self, x: u16, y: u16) {
        self.dev[0x92] = (x >> 8) as u8;
        self.dev[0x93] = x as u8;
        self.dev[0x94] = (y >> 8) as u8;
        self.dev[0x95] = y as u8;
    }

    pub fn set_mouse_button_down(&mut self, mask: u8) {
        self.dev[0x96] |= mask;
    }

    pub fn set_mouse_button_up(&mut self, mask: u8) {
        self.dev[0x96] &= !mask;
    }

    pub fn set_mouse_scroll(&mut self, x: i16, y: i16) {
        self.dev[0x9a] = (x >> 8) as u8;
        self.dev[0x9b] = x as u8;
        self.dev[0x9c] = (y >> 8) as u8;
        self.dev[0x9d] = y as u8;
    }

    pub fn clear_mouse_scroll(&mut self) {
        self.dev[0x9a] = 0;
        self.dev[0x9b] = 0;
        self.dev[0x9c] = 0;
        self.dev[0x9d] = 0;
    }

    pub fn render_to_rgba(&self, dst: &mut [u8]) {
        let stride = self.width as usize + PADDING;
        let mut src_idx = HALF_PADDING * stride + HALF_PADDING;
        let mut dst_idx = 0;

        for _y in 0..self.height {
            let row_slice = &self.screen_layers[src_idx..src_idx + self.width as usize];
            for &layer_byte in row_slice {
                let color = self.screen_palette[layer_byte as usize];
                dst[dst_idx..dst_idx + 4].copy_from_slice(&color);
                dst_idx += 4;
            }
            src_idx += stride;
        }
    }
}

fn read_ram_string(memory: &Memory, address: u16) -> String {
    let mut bytes = Vec::new();
    let mut cur = address;
    for _ in 0..4096 {
        let b = memory.read_u8_at(cur);
        if b == 0 {
            break;
        }
        bytes.push(b);
        cur = cur.wrapping_add(1);
    }
    String::from_utf8_lossy(&bytes).to_string()
}

fn write_ram_slice(memory: &mut Memory, address: u16, data: &[u8]) {
    for (i, &b) in data.iter().enumerate() {
        let target = address.wrapping_add(i as u16);
        memory.write_u8_at(target, b);
    }
}

impl GuiDevice {
    fn update_palette(&mut self) {
        let mut colors = [[0u8; 4]; 4];
        for (i, color) in colors.iter_mut().enumerate() {
            let shift = if i % 2 == 0 { 4 } else { 0 };
            let byte_offset = i / 2;
            let r_nibble = (self.dev[0x08 + byte_offset] >> shift) & 0x0f;
            let g_nibble = (self.dev[0x0a + byte_offset] >> shift) & 0x0f;
            let b_nibble = (self.dev[0x0c + byte_offset] >> shift) & 0x0f;
            *color = [
                (r_nibble << 4) | r_nibble,
                (g_nibble << 4) | g_nibble,
                (b_nibble << 4) | b_nibble,
                255,
            ];
        }

        for i in 0..16 {
            self.screen_palette[i] = if i < 4 { colors[i] } else { colors[i >> 2] };
        }
        self.needs_redraw = true;
    }

    fn resize_screen(&mut self, width: u16, height: u16) {
        let width = width.max(8);
        let height = height.max(8);

        if width != self.width || height != self.height {
            self.width = width;
            self.height = height;
            self.dev[0x22] = (width >> 8) as u8;
            self.dev[0x23] = width as u8;
            self.dev[0x24] = (height >> 8) as u8;
            self.dev[0x25] = height as u8;

            let stride = width as usize + PADDING;
            let total_height = height as usize + PADDING;
            self.screen_layers = vec![0u8; stride * total_height];
            self.needs_redraw = true;
            self.size_changed = true;
        }
    }

    fn sync_coords_to_dev(&mut self) {
        self.dev[0x28] = (self.x >> 8) as u8;
        self.dev[0x29] = self.x as u8;
        self.dev[0x2a] = (self.y >> 8) as u8;
        self.dev[0x2b] = self.y as u8;
        self.dev[0x2c] = (self.addr >> 8) as u8;
        self.dev[0x2d] = self.addr as u8;
    }

    fn deo_pixel(&mut self) {
        let ctrl = self.dev[0x2e];
        let hi = (ctrl & 0x40) != 0;
        let mask = if hi { 0x03 } else { 0x0c };
        let color = if hi { (ctrl & 0x03) << 2 } else { ctrl & 0x03 };
        let x = self.x;
        let y = self.y;

        if std::env::var_os("UXR_DEBUG_SCREEN").is_some() {
            eprintln!(
                "[SCREEN pixel] ctrl={:#04x} pos=({}, {}) auto_x={} auto_y={}",
                ctrl, self.x, self.y, self.auto_x, self.auto_y
            );
        }

        if x >= 0 && (x as u16) < self.width && y >= 0 && (y as u16) < self.height {
            let stride = self.width as usize + PADDING;
            let x_u = x as usize;
            let y_u = y as usize;

            if (ctrl & 0x80) != 0 {
                // Fill mode
                let x1 = if (ctrl & 0x10) != 0 {
                    HALF_PADDING
                } else {
                    x_u + HALF_PADDING
                };
                let x2 = if (ctrl & 0x10) != 0 {
                    x_u
                } else {
                    self.width as usize
                };
                let y1 = if (ctrl & 0x20) != 0 {
                    HALF_PADDING
                } else {
                    y_u + HALF_PADDING
                };
                let y2 = if (ctrl & 0x20) != 0 {
                    y_u
                } else {
                    self.height as usize
                };
                let hor = (x2 + HALF_PADDING).saturating_sub(x1);
                let ver = (y2 + HALF_PADDING).saturating_sub(y1);

                for row in 0..ver {
                    let row_start = (y1 + row) * stride + x1;
                    for px in 0..hor {
                        let idx = row_start + px;
                        self.screen_layers[idx] = (self.screen_layers[idx] & mask) | color;
                    }
                }
            } else {
                // Single pixel mode
                let dst = (y_u + HALF_PADDING) * stride + (x_u + HALF_PADDING);
                self.screen_layers[dst] = (self.screen_layers[dst] & mask) | color;

                if self.auto_x {
                    self.x = self.x.wrapping_add(if (ctrl & 0x10) != 0 { -1 } else { 1 });
                }
                if self.auto_y {
                    self.y = self.y.wrapping_add(if (ctrl & 0x20) != 0 { -1 } else { 1 });
                }
                self.sync_coords_to_dev();
            }
            self.needs_redraw = true;
        }
    }

    fn deo_sprite(&mut self, memory: &Memory) {
        let ctrl = self.dev[0x2f];
        let flip_x = (ctrl & 0x10) != 0;
        let flip_y = (ctrl & 0x20) != 0;
        let dx: i32 = if flip_x {
            -(self.dy as i32)
        } else {
            self.dy as i32
        };
        let dy: i32 = if flip_y {
            -(self.dx as i32)
        } else {
            self.dx as i32
        };
        let row_start: i32 = if flip_x { 0 } else { 7 };
        let row_delta: i32 = if flip_x { 1 } else { -1 };
        let col_start: i32 = if flip_y { 7 } else { 0 };
        let col_delta: i32 = if flip_y { -1 } else { 1 };
        let layer = (ctrl & 0x40) != 0;
        let layer_idx = if layer { 1 } else { 0 };
        let layer_mask = if layer { 0x03 } else { 0x0c };
        let is_2bpp = (ctrl & 0x80) != 0;
        let addr_incr = (self.auto_addr as u16) << (if is_2bpp { 1 } else { 0 });
        let stride = self.width as usize + PADDING;
        let total_height = self.height as usize + PADDING;
        let blend = (ctrl & 0x0f) as usize;
        let opaque_mask = blend % 5;
        let table = BLEND_LUT[blend][layer_idx];

        let mut cur_x = self.x as i32;
        let mut cur_y = self.y as i32;

        if std::env::var_os("UXR_DEBUG_SCREEN").is_some() {
            eprintln!(
                "[SCREEN sprite] ctrl={:#04x} pos=({}, {}) auto_x={} auto_y={} len={} addr={:#06x} dx={} dy={}",
                ctrl, self.x, self.y, self.auto_x, self.auto_y, self.auto_len, self.addr, dx, dy
            );
        }

        for _ in 0..=self.auto_len {
            let x0 = cur_x + HALF_PADDING as i32;
            let y0 = cur_y + HALF_PADDING as i32;

            if x0 + 8 < stride as i32 && y0 + 8 < total_height as i32 && x0 >= 0 && y0 >= 0 {
                for j in 0..8 {
                    let col_offset = col_start + j * col_delta;
                    let ram_addr1 = self.addr.wrapping_add(col_offset as u16);
                    let ch1 = memory.read_u8_at(ram_addr1) as u32;
                    let ch2 = if is_2bpp {
                        (memory.read_u8_at(ram_addr1.wrapping_add(8)) as u32) << 1
                    } else {
                        0
                    };

                    let dst_row = (y0 as usize + j as usize) * stride + (x0 as usize);
                    for k in 0..8 {
                        let bit_row = row_start + k * row_delta;
                        let color = (((ch1 >> bit_row) & 1) | ((ch2 >> bit_row) & 2)) as usize;
                        if opaque_mask != 0 || color != 0 {
                            let dst_idx = dst_row + k as usize;
                            self.screen_layers[dst_idx] =
                                (self.screen_layers[dst_idx] & layer_mask) | table[color];
                        }
                    }
                }
            }

            cur_x += dx;
            cur_y += dy;
            self.addr = self.addr.wrapping_add(addr_incr);
        }

        if self.auto_x {
            self.x = (self.x as i32
                + if flip_x {
                    -(self.dx as i32)
                } else {
                    self.dx as i32
                }) as i16;
        }
        if self.auto_y {
            self.y = (self.y as i32
                + if flip_y {
                    -(self.dy as i32)
                } else {
                    self.dy as i32
                }) as i16;
        }
        self.sync_coords_to_dev();
        self.needs_redraw = true;
    }

    fn update_datetime(&mut self) {
        let seconds = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        let mut tm = Tm::default();
        // SAFETY: Calling standard POSIX localtime_r
        unsafe {
            if !localtime_r(&seconds, &mut tm).is_null() {
                let year = (tm.tm_year + 1900) as u16;
                self.dev[0xc0] = (year >> 8) as u8;
                self.dev[0xc1] = year as u8;
                self.dev[0xc2] = tm.tm_mon as u8;
                self.dev[0xc3] = tm.tm_mday as u8;
                self.dev[0xc4] = tm.tm_hour as u8;
                self.dev[0xc5] = tm.tm_min as u8;
                self.dev[0xc6] = tm.tm_sec as u8;
                self.dev[0xc7] = tm.tm_wday as u8;
                let yday = tm.tm_yday as u16;
                self.dev[0xc8] = (yday >> 8) as u8;
                self.dev[0xc9] = yday as u8;
                self.dev[0xca] = if tm.tm_isdst > 0 { 1 } else { 0 };
            }
        }
    }

    fn set_file_success(&mut self, id: usize, success: u16) {
        let base = if id == 0 { 0xa0 } else { 0xb0 };
        self.dev[base + 0x02] = (success >> 8) as u8;
        self.dev[base + 0x03] = success as u8;
    }

    fn deo_name(&mut self, id: usize, memory: &Memory) {
        let base = if id == 0 { 0xa0 } else { 0xb0 };
        let addr = ((self.dev[base + 0x08] as u16) << 8) | (self.dev[base + 0x09] as u16);
        let path = read_ram_string(memory, addr);
        self.files[id].reset();
        self.files[id].filepath = path;
        self.set_file_success(id, 0);
    }

    fn deo_stat(&mut self, id: usize, memory: &mut Memory) {
        let base = if id == 0 { 0xa0 } else { 0xb0 };
        let addr = ((self.dev[base + 0x04] as u16) << 8) | (self.dev[base + 0x05] as u16);
        let mut len = ((self.dev[base + 0x0a] as usize) << 8) | (self.dev[base + 0x0b] as usize);
        if addr as usize + len > 0x10000 {
            len = 0x10000 - addr as usize;
        }

        if self.files[id].filepath.is_empty() {
            self.set_file_success(id, 0);
            return;
        }

        let path = Path::new(&self.files[id].filepath);
        let stat_data = format_stat(path, len);
        write_ram_slice(memory, addr, &stat_data);
        self.set_file_success(id, stat_data.len() as u16);
    }

    fn deo_delete(&mut self, id: usize) {
        self.files[id].reset();
        if self.files[id].filepath.is_empty() {
            self.set_file_success(id, 0);
            return;
        }

        let path = Path::new(&self.files[id].filepath);
        let ok = if path.is_dir() {
            std::fs::remove_dir(path).is_ok()
        } else {
            std::fs::remove_file(path).is_ok()
        };
        self.set_file_success(id, if ok { 1 } else { 0 });
    }

    fn deo_read(&mut self, id: usize, memory: &mut Memory) {
        let base = if id == 0 { 0xa0 } else { 0xb0 };
        let addr = ((self.dev[base + 0x0c] as u16) << 8) | (self.dev[base + 0x0d] as u16);
        let mut len = ((self.dev[base + 0x0a] as usize) << 8) | (self.dev[base + 0x0b] as usize);
        if addr as usize + len > 0x10000 {
            len = 0x10000 - addr as usize;
        }

        if self.files[id].filepath.is_empty() || len == 0 {
            self.set_file_success(id, 0);
            return;
        }

        let path = PathBuf::from(&self.files[id].filepath);

        if matches!(self.files[id].state, FileState::Idle) {
            if path.is_dir() {
                let data = read_dir_listing(&path);
                self.files[id].state = FileState::DirRead { data, offset: 0 };
            } else {
                match File::open(&path) {
                    Ok(file) => {
                        self.files[id].state = FileState::FileRead { file };
                    }
                    Err(_) => {
                        self.set_file_success(id, 0);
                        return;
                    }
                }
            }
        }

        let read_result = match &mut self.files[id].state {
            FileState::FileRead { file } => {
                let mut buffer = vec![0u8; len];
                match file.read(&mut buffer) {
                    Ok(n) => {
                        buffer.truncate(n);
                        Some(buffer)
                    }
                    Err(_) => None,
                }
            }
            FileState::DirRead { data, offset } => {
                let available = data.len().saturating_sub(*offset);
                let n = available.min(len);
                let chunk = data[*offset..*offset + n].to_vec();
                *offset += n;
                Some(chunk)
            }
            _ => None,
        };

        let bytes_read = match read_result {
            Some(buffer) => {
                let n = buffer.len();
                write_ram_slice(memory, addr, &buffer);
                n
            }
            None => 0,
        };

        if bytes_read == 0 {
            self.files[id].reset();
        }
        self.set_file_success(id, bytes_read as u16);
    }

    fn deo_write(&mut self, id: usize, memory: &Memory) {
        let base = if id == 0 { 0xa0 } else { 0xb0 };
        let addr = ((self.dev[base + 0x0e] as u16) << 8) | (self.dev[base + 0x0f] as u16);
        let mut len = ((self.dev[base + 0x0a] as usize) << 8) | (self.dev[base + 0x0b] as usize);
        let append = (self.dev[base + 0x07] & 0x01) != 0;
        if addr as usize + len > 0x10000 {
            len = 0x10000 - addr as usize;
        }

        if self.files[id].filepath.is_empty() {
            self.set_file_success(id, 0);
            return;
        }

        let filepath = self.files[id].filepath.clone();

        if filepath.ends_with('/') {
            let ok = std::fs::create_dir_all(&filepath).is_ok();
            self.set_file_success(id, if ok { 1 } else { 0 });
            return;
        }

        let path = Path::new(&filepath);

        if !matches!(self.files[id].state, FileState::FileWrite { .. }) {
            self.files[id].reset();
            if let Some(parent) = path.parent()
                && !parent.as_os_str().is_empty()
            {
                let _ = std::fs::create_dir_all(parent);
            }

            let open_result = OpenOptions::new()
                .create(true)
                .write(true)
                .append(append)
                .truncate(!append)
                .open(path);

            match open_result {
                Ok(file) => {
                    self.files[id].state = FileState::FileWrite { file };
                }
                Err(_) => {
                    self.set_file_success(id, 0);
                    return;
                }
            }
        }

        let mut buffer = vec![0u8; len];
        for (i, byte) in buffer.iter_mut().enumerate() {
            *byte = memory.read_u8_at(addr.wrapping_add(i as u16));
        }

        let bytes_written = if let FileState::FileWrite { file } = &mut self.files[id].state {
            match file.write_all(&buffer) {
                Ok(()) => {
                    let _ = file.flush();
                    len
                }
                Err(_) => 0,
            }
        } else {
            0
        };

        self.set_file_success(id, bytes_written as u16);
    }
}

fn format_stat(path: &Path, len: usize) -> Vec<u8> {
    match std::fs::metadata(path) {
        Ok(meta) => {
            if meta.is_dir() {
                vec![b'-'; len]
            } else {
                let size = meta.len();
                let max = if len >= 8 {
                    u64::MAX
                } else {
                    1u64 << (len * 4)
                };
                if size < max {
                    let hex = format!("{:0width$x}", size, width = len);
                    hex.into_bytes()
                } else {
                    vec![b'?'; len]
                }
            }
        }
        Err(_) => vec![b'!'; len],
    }
}

fn read_dir_listing(path: &Path) -> Vec<u8> {
    let mut listing = Vec::new();
    if let Ok(entries) = std::fs::read_dir(path) {
        let mut entry_list: Vec<_> = entries.filter_map(|e| e.ok()).collect();
        entry_list.sort_by_key(|e| e.file_name());

        for entry in entry_list {
            let file_name = entry.file_name();
            let name_str = file_name.to_string_lossy();
            if name_str == "." || name_str == ".." {
                continue;
            }

            let entry_path = entry.path();
            let stat_bytes = format_stat(&entry_path, 4);
            listing.extend_from_slice(&stat_bytes);
            listing.push(b'\t');
            listing.extend_from_slice(name_str.as_bytes());
            if entry_path.is_dir() {
                listing.push(b'/');
            }
            listing.push(b'\n');
        }
    }
    listing
}

impl Default for GuiDevice {
    fn default() -> Self {
        Self::new()
    }
}

impl VmDevice for GuiDevice {
    fn input(&mut self, port: u8) -> u8 {
        match port {
            // 00 system
            0x00..=0x0f => self.dev[port as usize],

            // 10 console
            0x10..=0x1f => self.dev[port as usize],

            // 20 screen
            0x22 => (self.width >> 8) as u8,
            0x23 => self.width as u8,
            0x24 => (self.height >> 8) as u8,
            0x25 => self.height as u8,
            0x28 => ((self.x >> 8) & 0xff) as u8,
            0x29 => (self.x & 0xff) as u8,
            0x2a => ((self.y >> 8) & 0xff) as u8,
            0x2b => (self.y & 0xff) as u8,
            0x2c => (self.addr >> 8) as u8,
            0x2d => self.addr as u8,
            0x20..=0x2f => self.dev[port as usize],

            // 90 mouse
            0x90..=0x9f => self.dev[port as usize],

            // a0 & b0 files
            0xa0..=0xbf => self.dev[port as usize],

            // c0 datetime
            0xc0..=0xca => {
                self.update_datetime();
                self.dev[port as usize]
            }
            0xcb..=0xcf => self.dev[port as usize],

            // Other subsystems
            _ => self.dev[port as usize],
        }
    }

    fn output(&mut self, port: u8, value: u8, memory: &mut Memory) {
        self.dev[port as usize] = value;

        match port {
            // 00 system
            0x03 => {
                let descriptor_address = ((self.dev[0x02] as u16) << 8) | (self.dev[0x03] as u16);
                memory.expansion_execute(descriptor_address);
            }
            0x08..=0x0d => {
                self.update_palette();
            }
            0x0f => {
                if value != 0 {
                    self.terminated = true;
                    self.exit_code = value & 0x7f;
                }
            }
            0x00..=0x0e => {
                // Handled in self.dev
            }

            // 10 console
            0x18 => {
                let mut stdout = io::stdout().lock();
                let _ = stdout.write_all(&[value]);
                let _ = stdout.flush();
            }
            0x19 => {
                let mut stderr = io::stderr().lock();
                let _ = stderr.write_all(&[value]);
                let _ = stderr.flush();
            }
            0x10..=0x17 | 0x1a..=0x1f => {
                // Handled in self.dev
            }

            // 20 screen
            0x22 | 0x23 => {
                let width = (((self.dev[0x22] as u16) << 8) | (self.dev[0x23] as u16)) & 0x0fff;
                self.resize_screen(width, self.height);
            }
            0x24 | 0x25 => {
                let height = (((self.dev[0x24] as u16) << 8) | (self.dev[0x25] as u16)) & 0x0fff;
                self.resize_screen(self.width, height);
            }
            0x26 => {
                self.auto_x = (value & 0x01) != 0;
                self.auto_y = (value & 0x02) != 0;
                self.auto_addr = (value & 0x04) << 1;
                self.auto_len = value >> 4;
                self.dx = if self.auto_x { 8 } else { 0 };
                self.dy = if self.auto_y { 8 } else { 0 };
            }
            0x28 | 0x29 => {
                self.x = ((self.dev[0x28] as i16) << 8) | (self.dev[0x29] as i16);
            }
            0x2a | 0x2b => {
                self.y = ((self.dev[0x2a] as i16) << 8) | (self.dev[0x2b] as i16);
            }
            0x2c | 0x2d => {
                self.addr = ((self.dev[0x2c] as u16) << 8) | (self.dev[0x2d] as u16);
            }
            0x2e => {
                self.deo_pixel();
            }
            0x2f => {
                self.deo_sprite(memory);
            }
            0x20..=0x21 | 0x27 => {
                // Handled in self.dev
            }

            // 90 mouse
            0x90..=0x9f => {
                // Handled in self.dev
            }

            // a0 file 1
            0xa5 => self.deo_stat(0, memory),
            0xa6 => self.deo_delete(0),
            0xa9 => self.deo_name(0, memory),
            0xad => self.deo_read(0, memory),
            0xaf => self.deo_write(0, memory),
            0xa0..=0xae => {
                // Handled in self.dev
            }

            // b0 file 2
            0xb5 => self.deo_stat(1, memory),
            0xb6 => self.deo_delete(1),
            0xb9 => self.deo_name(1, memory),
            0xbd => self.deo_read(1, memory),
            0xbf => self.deo_write(1, memory),
            0xb0..=0xbe => {
                // Handled in self.dev
            }

            // c0 datetime
            0xc0..=0xcf => {
                // Handled in self.dev
            }

            // Unknown sub-systems
            _ => {
                eprintln!("Unknown write to port 0x{port:02x} -> 0x{value:02x}");
            }
        }
    }
}
