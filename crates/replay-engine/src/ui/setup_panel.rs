//! PIN / long-press gated technician setup (camera, format, HDMI display).

use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use egui::RichText;
use replay_core::config::OperatorConfig;
use replay_core::types::{DisplayInfo, VideoFormat};

use crate::devices::{list_devices, CaptureDevice};
use crate::ui::operator_shell::{send_cmd_with_feedback, OperatorCmd};

const LONG_PRESS: Duration = Duration::from_secs(3);

#[derive(Debug, Clone)]
pub struct SetupSelection {
    pub device_id: String,
    pub width: u32,
    pub height: u32,
    pub fps: u32,
    pub pixel_format: String,
    pub display_id: u32,
}

#[derive(Debug)]
pub struct SetupUiState {
    pub unlocked_until: Option<Instant>,
    pub pin_buffer: String,
    pub show_pin_dialog: bool,
    pub banner_press_start: Option<Instant>,
    pub devices: Vec<CaptureDevice>,
    pub formats: Vec<VideoFormat>,
    pub displays: Vec<DisplayInfo>,
    pub selected_device: usize,
    pub selected_format: usize,
    pub selected_display: usize,
}

impl SetupUiState {
    pub fn new() -> Self {
        Self {
            unlocked_until: None,
            pin_buffer: String::new(),
            show_pin_dialog: false,
            banner_press_start: None,
            devices: Vec::new(),
            formats: Vec::new(),
            displays: Vec::new(),
            selected_device: 0,
            selected_format: 0,
            selected_display: 0,
        }
    }

    pub fn is_unlocked(&self) -> bool {
        self.unlocked_until
            .map(|t| Instant::now() < t)
            .unwrap_or(false)
    }

    pub fn lock(&mut self) {
        self.unlocked_until = None;
        self.show_pin_dialog = false;
        self.pin_buffer.clear();
    }

    pub fn unlock_for(&mut self, secs: u64) {
        self.unlocked_until = Some(Instant::now() + Duration::from_secs(secs.max(60)));
        self.show_pin_dialog = false;
        self.pin_buffer.clear();
    }

    pub fn refresh_devices(&mut self, test_mode: bool) {
        self.devices = list_devices(test_mode);
        if self.selected_device >= self.devices.len() {
            self.selected_device = 0;
        }
        self.refresh_formats();
    }

    pub fn refresh_formats(&mut self) {
        if let Some(dev) = self.devices.get(self.selected_device) {
            self.formats = crate::devices::list_formats(&dev.id);
        } else {
            self.formats.clear();
        }
        if self.selected_format >= self.formats.len() {
            self.selected_format = 0;
        }
    }

    pub fn set_displays(&mut self, displays: Vec<DisplayInfo>) {
        self.displays = displays;
        if self.selected_display >= self.displays.len() {
            self.selected_display = 0;
        }
    }

    pub fn selection(&self) -> Option<SetupSelection> {
        let dev = self.devices.get(self.selected_device)?;
        let fmt = self.formats.get(self.selected_format)?;
        let display_id = self
            .displays
            .get(self.selected_display)
            .map(|d| d.id)
            .unwrap_or(0);
        let fps = if fmt.fps_den > 0 {
            (fmt.fps_num / fmt.fps_den.max(1)) as u32
        } else {
            30
        };
        Some(SetupSelection {
            device_id: dev.id.clone(),
            width: fmt.width as u32,
            height: fmt.height as u32,
            fps,
            pixel_format: fmt.pixel_format.clone(),
            display_id,
        })
    }
}

pub fn format_label(f: &VideoFormat) -> String {
    let fps = if f.fps_den > 0 {
        f.fps_num / f.fps_den.max(1)
    } else {
        0
    };
    format!(
        "{}×{} @ {} {} fps",
        f.width, f.height, f.pixel_format, fps
    )
}

pub fn paint_setup_controls(
    ui: &mut egui::Ui,
    setup: &mut SetupUiState,
    op_cfg: &OperatorConfig,
    cmd_tx: &Sender<OperatorCmd>,
    toast: Arc<Mutex<Option<(String, bool, Instant)>>>,
    test_mode: bool,
) {
    ui.separator();
    ui.label(RichText::new("Setup (technician)").strong());

    if setup.is_unlocked() {
        if ui.button("Lock setup").clicked() {
            setup.lock();
        }
        ui.add_space(4.0);

        if ui.button("Refresh devices").clicked() {
            setup.refresh_devices(test_mode);
            send_cmd_with_feedback(cmd_tx, toast.clone(), OperatorCmd::RefreshSetup, "Scanning…");
        }

        egui::ComboBox::from_label("Camera")
            .selected_text(
                setup
                    .devices
                    .get(setup.selected_device)
                    .map(|d| d.display_name.as_str())
                    .unwrap_or("(none)"),
            )
            .show_ui(ui, |ui| {
                let mut changed = false;
                for (i, d) in setup.devices.iter().enumerate() {
                    if ui
                        .selectable_value(&mut setup.selected_device, i, &d.display_name)
                        .clicked()
                    {
                        changed = true;
                    }
                }
                if changed {
                    setup.refresh_formats();
                }
            });

        egui::ComboBox::from_label("Format")
            .selected_text(
                setup
                    .formats
                    .get(setup.selected_format)
                    .map(format_label)
                    .unwrap_or_else(|| "(none)".into()),
            )
            .show_ui(ui, |ui| {
                for (i, f) in setup.formats.iter().enumerate() {
                    ui.selectable_value(&mut setup.selected_format, i, format_label(f));
                }
            });

        egui::ComboBox::from_label("Audience HDMI")
            .selected_text(
                setup
                    .displays
                    .get(setup.selected_display)
                    .map(|d| d.name.as_str())
                    .unwrap_or("Display 0"),
            )
            .show_ui(ui, |ui| {
                for (i, d) in setup.displays.iter().enumerate() {
                    ui.selectable_value(
                        &mut setup.selected_display,
                        i,
                        format!("{} ({}×{})", d.name, d.width, d.height),
                    );
                }
            });

        if ui.button("Apply & go live").clicked() {
            send_cmd_with_feedback(
                cmd_tx,
                toast.clone(),
                OperatorCmd::ApplySetup,
                "Applying…",
            );
        }
    } else {
        if ui.button("Unlock setup (PIN)").clicked() {
            if op_cfg.setup_pin.is_empty() {
                setup.unlock_for(op_cfg.setup_unlock_seconds);
                setup.refresh_devices(test_mode);
            } else {
                setup.show_pin_dialog = true;
            }
        }
    }
}

pub fn paint_pin_dialog(
    ctx: &egui::Context,
    setup: &mut SetupUiState,
    op_cfg: &OperatorConfig,
    test_mode: bool,
) {
    if !setup.show_pin_dialog {
        return;
    }
    egui::Window::new("Setup PIN")
        .collapsible(false)
        .resizable(false)
        .show(ctx, |ui| {
            ui.label("Enter PIN:");
            ui.text_edit_singleline(&mut setup.pin_buffer);
            ui.horizontal(|ui| {
                if ui.button("OK").clicked()
                    && !op_cfg.setup_pin.is_empty()
                    && setup.pin_buffer == op_cfg.setup_pin
                {
                    setup.unlock_for(op_cfg.setup_unlock_seconds);
                    setup.refresh_devices(test_mode);
                }
                if ui.button("Cancel").clicked() {
                    setup.show_pin_dialog = false;
                    setup.pin_buffer.clear();
                }
            });
        });
}

pub fn handle_banner_press(
    setup: &mut SetupUiState,
    op_cfg: &OperatorConfig,
    pressed: bool,
    test_mode: bool,
) {
    if setup.is_unlocked() {
        return;
    }
    if pressed {
        if setup.banner_press_start.is_none() {
            setup.banner_press_start = Some(Instant::now());
        } else if let Some(start) = setup.banner_press_start {
            if start.elapsed() >= LONG_PRESS {
                setup.unlock_for(op_cfg.setup_unlock_seconds);
                setup.refresh_devices(test_mode);
                setup.banner_press_start = None;
            }
        }
    } else {
        setup.banner_press_start = None;
    }
}
