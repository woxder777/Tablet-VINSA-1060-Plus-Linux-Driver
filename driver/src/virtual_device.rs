// VINSA 1600 Plus Driver (by feveal)
use std::io::Error;
use std::{collections::HashMap, u16};

use evdev::{
    uinput::{VirtualDevice, VirtualDeviceBuilder},
    AbsInfo, AbsoluteAxisType, AttributeSet, EventType, InputEvent, Key, Synchronization,
    UinputAbsSetup,
};

#[derive(Default)]
pub struct RawDataReader {
    pub data: Vec<u8>,
}

impl RawDataReader {
    const X_AXIS_HIGH: usize = 1;
    const X_AXIS_LOW: usize = 2;
    const Y_AXIS_HIGH: usize = 3;
    const Y_AXIS_LOW: usize = 4;
    const PRESSURE_HIGH: usize = 5;
    const PRESSURE_LOW: usize = 6;
    const PEN_BUTTONS: usize = 9;
    const TABLET_BUTTONS_HIGH: usize = 12;
    const TABLET_BUTTONS_LOW: usize = 11;

    pub fn new() -> Self {
        RawDataReader {
            data: vec![0u8; 64],
        }
    }

    fn u16_from_2_u8(&self, high: u8, low: u8) -> u16 {
        (high as u16) << 8 | low as u16
    }

    fn x_axis(&self) -> i32 {
        self.u16_from_2_u8(self.data[Self::X_AXIS_HIGH], self.data[Self::X_AXIS_LOW]) as i32
    }

    fn y_axis(&self) -> i32 {
        self.u16_from_2_u8(self.data[Self::Y_AXIS_HIGH], self.data[Self::Y_AXIS_LOW]) as i32
    }

    fn pressure(&self) -> i32 {
        self.u16_from_2_u8(
            self.data[Self::PRESSURE_HIGH],
            self.data[Self::PRESSURE_LOW],
        ) as i32
    }

    fn tablet_buttons_as_binary_flags(&self) -> u16 {
        self.u16_from_2_u8(
            self.data[Self::TABLET_BUTTONS_HIGH],
            self.data[Self::TABLET_BUTTONS_LOW],
        ) | (0xcc << 8)
    }

    fn pen_buttons(&self) -> u8 {
        self.data[Self::PEN_BUTTONS]
    }
}

pub struct DeviceDispatcher {
    tablet_last_raw_pressed_buttons: u16,
    pen_last_raw_pressed_button: u8,
    tablet_button_id_to_key_code_map: HashMap<u8, Vec<Key>>,
    pen_button_id_to_key_code_map: HashMap<u8, Vec<Key>>,
    virtual_pen: VirtualDevice,
    virtual_keyboard: VirtualDevice,
    was_touching: bool,
    is_mouse_mode: bool,            // <- Mouse
    last_x: i32,        // ← for smooth_coordinates
    last_y: i32,        // ← for smooth_coordinates
    mouse_area_scale: f32,
}

impl Default for DeviceDispatcher {
    fn default() -> Self {
        Self::new()
    }
}

impl DeviceDispatcher {
    const PRESSED: i32 = 1;
    const RELEASED: i32 = 0;
    const HOLD: i32 = 2;

    pub fn new() -> Self {
        let default_tablet_button_id_to_key_code_map: HashMap<u8, Vec<Key>> = [
            (0, vec![Key::KEY_TAB]),        //TAB
            (1, vec![Key::KEY_SPACE]),      //SPACE
            (2, vec![Key::KEY_LEFTALT]),    //ALT
            (3, vec![Key::KEY_LEFTCTRL]),   //CTRL
            (4, vec![Key::KEY_SCROLLUP]),   //MOUSE, SCROLL UP
            (5, vec![Key::KEY_SCROLLDOWN]), //MOUSE, SCROLL DOWN
            (6, vec![Key::KEY_LEFTBRACE]),  //MOUSE AREA -
            (7, vec![Key::KEY_LEFTCTRL, Key::KEY_KPMINUS]), //CTRL-, ZOOM -
            (8, vec![Key::KEY_LEFTCTRL, Key::KEY_KPPLUS]),  //CTRL+, ZOOM +
            (9, vec![Key::KEY_ESC]),           //ESC, CANCEL
            //10 This code is not emitted by physical device
            //11 This code is not emitted by physical device
            (12, vec![Key::KEY_B]),         //TOOGLE MOUSE/TABLET
            (13, vec![Key::KEY_RIGHTBRACE]),    //MOUSE AREA +
        ]
        .iter()
        .cloned()
        .collect();

        let default_pen_button_id_to_key_code_map: HashMap<u8, Vec<Key>> =
            [(4, vec![Key::BTN_STYLUS]), (6, vec![Key::BTN_STYLUS2])]
                .iter()
                .cloned()
                .collect();

        DeviceDispatcher {
                tablet_last_raw_pressed_buttons: 0xFFFF,
                pen_last_raw_pressed_button: 0,
                tablet_button_id_to_key_code_map: default_tablet_button_id_to_key_code_map.clone(),
                pen_button_id_to_key_code_map: default_pen_button_id_to_key_code_map.clone(),
                virtual_pen: Self::virtual_pen_builder(
                    &default_pen_button_id_to_key_code_map
                        .values()
                        .flatten()
                        .cloned()
                        .collect::<Vec<Key>>(),
                )
                .expect("Error building virtual pen"),
                virtual_keyboard: Self::virtual_keyboard_builder(
                    &default_tablet_button_id_to_key_code_map
                        .values()
                        .flatten()
                        .cloned()
                        .collect::<Vec<Key>>(),
                )
                .expect("Error building virtual keyborad"),
                was_touching: false,
                is_mouse_mode: true,    // <- Start in mouse mode
                last_x: 2048,  // ← Initialize center
                last_y: 2048,  // ← Initialize center
                //Mouse Scale
                mouse_area_scale: 0.3,  // ← Init 30%
        }
    }

    fn smooth_coordinates(&mut self, x: i32, y: i32) -> (i32, i32) {
        if self.is_mouse_mode {
            // MOUSE MODE: Soft smoothed
            let smoothed_x = (self.last_x * 1 + x) / 2;  // 50%/50%
            let smoothed_y = (self.last_y * 1 + y) / 2;

            self.last_x = smoothed_x;
            self.last_y = smoothed_y;

            (smoothed_x, smoothed_y)
        } else {
            // TABLET MODE: hard smoothed
            let smoothed_x = (self.last_x * 3 + x) / 4;  // 75%/25%
            let smoothed_y = (self.last_y * 3 + y) / 4;

            self.last_x = smoothed_x;
            self.last_y = smoothed_y;

            (smoothed_x, smoothed_y)
        }
    }

    pub fn syn(&mut self) -> Result<(), Error> {
        self.virtual_keyboard.emit(&[InputEvent::new(
            EventType::SYNCHRONIZATION,
            Synchronization::SYN_REPORT.0,
            0,
        )])?;
        self.virtual_pen.emit(&[InputEvent::new(
            EventType::SYNCHRONIZATION,
            Synchronization::SYN_REPORT.0,
            0,
        )])?;
        Ok(())
    }

    pub fn dispatch(&mut self, raw_data: &RawDataReader) {
        self.emit_pen_events(raw_data);
        self.emit_tablet_events(raw_data);
    }

    fn emit_tablet_events(&mut self, raw_data: &RawDataReader) {
        let raw_button_as_binary_flags = raw_data.tablet_buttons_as_binary_flags();
        self.binary_flags_to_tablet_key_events(raw_button_as_binary_flags);
        self.tablet_last_raw_pressed_buttons = raw_button_as_binary_flags;
    }

    fn virtual_keyboard_builder(tablet_emitted_keys: &[Key]) -> Result<VirtualDevice, Error> {
        let mut key_set = AttributeSet::<Key>::new();
        for key in tablet_emitted_keys {
            key_set.insert(*key);
        }

        VirtualDeviceBuilder::new()?
            .name("virtual_tablet")
            .with_keys(&key_set)?
            .build()
    }

    fn binary_flags_to_tablet_key_events(&mut self, raw_button_as_flags: u16) {
        (0..14)
            .filter(|i| ![10, 11].contains(i))
            .for_each(|i| self.emit_tablet_key_event(i, raw_button_as_flags));
    }

pub fn emit_tablet_key_event(&mut self, i: u8, raw_button_as_flags: u16) {
    let id_as_binary_mask = 1 << i;
    let is_pressed = (raw_button_as_flags & id_as_binary_mask) == 0;
    let was_pressed = (self.tablet_last_raw_pressed_buttons & id_as_binary_mask) == 0;

    if let Some(state) = match (was_pressed, is_pressed) {
        (false, true) => Some(Self::PRESSED),
        (true, false) => Some(Self::RELEASED),
        (true, true) => Some(Self::HOLD),
        _ => None,
    } {

        // BUTTON [ - Reduce mouse area
        if i == 6 && state == Self::PRESSED {
            self.mouse_area_scale = (self.mouse_area_scale * 0.8).max(0.1);
            eprintln!("Mouse area reduced: {:.0}%", self.mouse_area_scale * 100.0);
            return;
        }

        // BUTTON ] - Enlarge mouse area
        if i == 13 && state == Self::PRESSED {
            self.mouse_area_scale = (self.mouse_area_scale * 1.2).min(0.4);
            eprintln!("Mouse area increased: {:.0}%", self.mouse_area_scale * 100.0);
            return;
        }
        // Toggle with B button (ID 12)
        if i == 12 && state == Self::PRESSED {
            self.is_mouse_mode = !self.is_mouse_mode;
            eprintln!("Modo: {}", if self.is_mouse_mode { "MOUSE (small area)" } else { "TABLET (full area)" });
            return;  // Exit without processing as normal key
        }

        if let Some(keys) = self.tablet_button_id_to_key_code_map.get(&i) {
            for &key in keys {
                self.virtual_keyboard
                    .emit(&[InputEvent::new(EventType::KEY, key.code(), state)])
                    .expect("Error emitting vitual keyboard key.");
            }

            self.virtual_keyboard
                .emit(&[InputEvent::new(
                    EventType::SYNCHRONIZATION,
                    Synchronization::SYN_REPORT.0,
                    0,
                )])
                .expect("Error emitting SYN.");
        }
    }
}


    fn virtual_pen_builder(pen_emitted_keys: &[Key]) -> Result<VirtualDevice, Error> {
        let abs_x_setup =
            UinputAbsSetup::new(AbsoluteAxisType::ABS_X, AbsInfo::new(0, 0, 4096, 0, 0, 1));
        let abs_y_setup =
            UinputAbsSetup::new(AbsoluteAxisType::ABS_Y, AbsInfo::new(0, 0, 4096, 0, 0, 1));
        let abs_pressure_setup = UinputAbsSetup::new(
            AbsoluteAxisType::ABS_PRESSURE,
            AbsInfo::new(0, 0, 5000, 0, 0, 1),
        );

        let mut key_set = AttributeSet::<Key>::new();
        for key in pen_emitted_keys {
            key_set.insert(*key);
        }

        for key in &[Key::BTN_TOOL_PEN, Key::BTN_LEFT, Key::BTN_RIGHT] {
            key_set.insert(*key);
        }

        VirtualDeviceBuilder::new()?
            .name("virtual_tablet")
            .with_absolute_axis(&abs_x_setup)?
            .with_absolute_axis(&abs_y_setup)?
            .with_absolute_axis(&abs_pressure_setup)?
            .with_keys(&key_set)?
            .build()
    }

fn emit_pen_events(&mut self, raw_data: &RawDataReader) {
    let raw_pen_buttons = raw_data.pen_buttons();
    self.raw_pen_buttons_to_pen_key_events(raw_pen_buttons);
    self.pen_last_raw_pressed_button = raw_pen_buttons;

    // Pressure normalization by mode
    let normalized_pressure = if self.is_mouse_mode {
        // MOUSE MODE: sensitivity 900
        Self::normalize_pressure_mode(raw_data.pressure(), 800, 2)
    } else {
        // TABLET MODE: sensitivity 510
        Self::normalize_pressure_mode(raw_data.pressure(), 510, 3)
    };

    // Apply smoothing to coordinates
    let (smoothed_x, smoothed_y) = self.smooth_coordinates(
        raw_data.x_axis(),
        raw_data.y_axis()
    );

    self.raw_pen_abs_to_pen_abs_events(
        smoothed_x,      // ← Smoothed coordinates
        smoothed_y,
        normalized_pressure,
    );

    self.pen_emit_touch(raw_data);
}

// Normalization by mode
    fn normalize_pressure_mode(raw_pressure: i32, threshold: i32, scaling: i32) -> i32 {
        match 2000 - raw_pressure {
            x if x <= threshold => 0,
            x => x * scaling,
        }
    }

fn normalize_pressure(raw_pressure: i32) -> i32 {
    let proximity_threshold = 700; // Adjust for proximity sensitivity (510 Draw, 600 Mouse)
    let strength_scaling = 3; // Adjust for strength of the press
    let base_value = 2000;

    match base_value - raw_pressure {
        x if x <= proximity_threshold => 0,  // Highest threshold = less sensitivity at distance
        x => x * strength_scaling,
    }
}

// New mouse area
fn raw_pen_abs_to_pen_abs_events(&mut self, x_axis: i32, y_axis: i32, pressure: i32) {
    let (x, y) = if self.is_mouse_mode {
        // MOUSE MODE: Only 25% of tablet area
        let center_x = 1024;  // Tablet center
        let center_y = 2048;
//        let range = 1230;     // Small Range (4096 * 0.3 = 1229 (30%))
//        let scale_factor = 4096 / range;  // ≈ 3.33
        let range = (4096.0 * self.mouse_area_scale) as i32;  // Use mouse_area_scale
        let scale_factor = 4096 / range.max(1);  // No zero divide

        // Map small area [center±512] to [0-4096]
        let scaled_x = ((x_axis - center_x) * scale_factor) + 2048;
        let scaled_y = ((y_axis - center_y) * scale_factor) + 2048;

        // Limit screen edges
        (scaled_x.clamp(0, 4096), scaled_y.clamp(0, 4096))
    } else {
        // TABLET MODE: Full area 1:1
        (x_axis, y_axis)
    };

    self.virtual_pen.emit(&[InputEvent::new(
        EventType::ABSOLUTE,
        AbsoluteAxisType::ABS_X.0,
        x,
    )]).expect("Error emitting ABS_X.");

    self.virtual_pen.emit(&[InputEvent::new(
        EventType::ABSOLUTE,
        AbsoluteAxisType::ABS_Y.0,
        y,
    )]).expect("Error emitting ABS_Y.");

    self.virtual_pen.emit(&[InputEvent::new(
        EventType::ABSOLUTE,
        AbsoluteAxisType::ABS_PRESSURE.0,
        pressure,
    )]).expect("Error emitting Pressure.");
}

    fn pen_emit_touch(&mut self, raw_data: &RawDataReader) {
        let is_touching = Self::normalize_pressure(raw_data.pressure()) > 0;
        if let Some(state) = match (self.was_touching, is_touching) {
            (false, true) => Some(Self::PRESSED),
            (true, false) => Some(Self::RELEASED),
            _ => None,
        } {
            self.virtual_pen.emit(&[InputEvent::new(
                EventType::KEY,
                Key::BTN_TOUCH.code(),
                state,
            )]).expect("Error emitting Touch");
        }
        self.was_touching = is_touching;
    }

    fn raw_pen_buttons_to_pen_key_events(&mut self, pen_button: u8) {
        if let Some((state, id)) = match (self.pen_last_raw_pressed_button, pen_button) {
            (2, x) if x == 6 || x == 4 => Some((Self::PRESSED, x)),
            (x, 2) if x == 6 || x == 4 => Some((Self::RELEASED, x)),
            (x, y) if x != 2 && x == y => Some((Self::HOLD, x)),
            _ => None,
        } {
            let keys = self
                .pen_button_id_to_key_code_map
                .get(&id)
                .expect("Error mapping pen keys.");
            for key in keys {
                self.virtual_pen
                    .emit(&[InputEvent::new(EventType::KEY, key.code(), state)])
                    .expect("Error emitting pen keys.")
            }
        }
    }
}
