use anyhow::{anyhow, Context, Result};
use nwd::NwgUi;
use nwg::{
    Button, CheckBox, CheckBoxState, ColorDialog, Font, Frame, GridLayout, Label, NativeUi,
    TrackBar, Window,
};
use once_cell::unsync::Lazy;
use std::cell::{Cell, RefCell};
use winapi::shared::windef::HWND;

use crate::config::{Config, ConfigHandle};
use crate::installer::{self, DCSVersion, InstallStatus};

const HEADING_FONT: Lazy<Font> = Lazy::new(|| {
    let mut font = Default::default();
    Font::builder()
        .family("Segoe UI")
        .size(28)
        .weight(700)
        .build(&mut font)
        .expect("Failed to set heading font");
    font
});

#[derive(Default, NwgUi)]
pub struct ControlWindow {
    config: RefCell<Option<ConfigHandle>>,

    #[nwg_control(size: (320, 400), title: "DCS Hemmecs", flags: "WINDOW|VISIBLE")]
    #[nwg_events(OnWindowClose: [ControlWindow::on_close])]
    pub window: Window,

    #[nwg_layout(parent: window, max_row: Some(12), max_column: Some(8))]
    grid: GridLayout,

    #[nwg_control]
    #[nwg_layout_item(layout: grid, row: 0, col_span: 8)]
    status_frame: Frame,

    #[nwg_control(text: "", parent: status_frame, size: (1000, 40))]
    status_text: Label,

    #[nwg_control(font: Some(&HEADING_FONT), text: "Installer")]
    #[nwg_layout_item(layout: grid, row: 1, col_span: 8)]
    install_title: Label,

    #[nwg_control(text: "Stable:")]
    #[nwg_layout_item(layout: grid, row: 2, col_span: 3)]
    install_stable_label: Label,

    #[nwg_control(text: "Not Detected", enabled: false)]
    #[nwg_events(
        OnButtonClick: [ControlWindow::install_stable]
    )]
    #[nwg_layout_item(layout: grid, row: 2, col: 3, col_span: 5)]
    install_stable_button: Button,

    #[nwg_control(text: "Openbeta:")]
    #[nwg_layout_item(layout: grid, row: 3, col_span: 3)]
    install_openbeta_label: Label,

    #[nwg_control(text: "Not Found", enabled: false)]
    #[nwg_events(
        OnButtonClick: [ControlWindow::install_openbeta]
    )]
    #[nwg_layout_item(layout: grid, row: 3, col: 3, col_span: 5)]
    install_openbeta_button: Button,

    #[nwg_control(font: Some(&HEADING_FONT), text: "Settings")]
    #[nwg_layout_item(layout: grid, row: 4, col_span: 8)]
    settings_title: Label,

    #[nwg_control(text: "Color")]
    #[nwg_layout_item(layout: grid, row: 5, col_span: 3)]
    color_label: Label,

    #[nwg_control(text: "")]
    #[nwg_events(
        OnButtonClick: [ControlWindow::on_color_button_click]
    )]
    #[nwg_layout_item(layout: grid, row: 5, col: 3, col_span: 5)]
    color_button: Button,
    color_value: Cell<(u8, u8, u8)>,

    #[nwg_control(text: "Brightness")]
    #[nwg_layout_item(layout: grid, row: 6, col_span: 3)]
    brightness_label: Label,

    #[nwg_control(flags: "VISIBLE|HORIZONTAL")]
    #[nwg_events(
        OnHorizontalScroll: [ControlWindow::save_config]
    )]
    #[nwg_layout_item(layout: grid, row: 6, col: 3, col_span: 5)]
    brightness_input: TrackBar,

    #[nwg_control(text: "Hide on HUD")]
    #[nwg_layout_item(layout: grid, row: 7, col_span: 3)]
    hide_on_hud_label: Label,

    #[nwg_control(text: "")]
    #[nwg_events(
        OnButtonClick: [ControlWindow::save_config]
    )]
    #[nwg_layout_item(layout: grid, row: 7, col: 3, col_span: 5)]
    hide_on_hud_checkbox: CheckBox,

    #[nwg_control(text: "Hide in cockpit")]
    #[nwg_layout_item(layout: grid, row: 8, col_span: 3)]
    hide_in_cockpit_label: Label,

    #[nwg_control(text: "")]
    #[nwg_events(
        OnButtonClick: [ControlWindow::save_config]
    )]
    #[nwg_layout_item(layout: grid, row: 8, col: 3, col_span: 5)]
    hide_in_cockpit_checkbox: CheckBox,

    #[nwg_control(text: "Show sample data")]
    #[nwg_events(
        OnButtonClick: [ControlWindow::save_config]
    )]
    #[nwg_layout_item(layout: grid, row: 11, col_span: 8)]
    sample_checkbox: CheckBox,
}

impl ControlWindow {
    fn on_close(&self) {
        nwg::stop_thread_dispatch();
    }

    fn update_color_button_label(&self) {
        self.color_button
            .set_text(&format!("{:?}", self.color_value.get()));
    }

    fn on_color_button_click(&self) {
        let mut color_dialog = Default::default();
        ColorDialog::builder().build(&mut color_dialog).unwrap();
        if color_dialog.run(None::<&Window>) {
            let color = color_dialog.color();
            self.color_value.set((color[0], color[1], color[2]));
            self.update_color_button_label();
            self.save_config();
        }
    }

    fn load_config(&self, config: &Config) {
        self.color_value.set(config.appearance.color);
        self.update_color_button_label();
        self.brightness_input
            .set_pos((config.appearance.brightness as f64 / (255.0 / 100.0)) as usize);
        self.hide_on_hud_checkbox
            .set_check_state(match config.occlusion.hide_on_hud {
                true => CheckBoxState::Checked,
                false => CheckBoxState::Unchecked,
            });
        self.hide_in_cockpit_checkbox
            .set_check_state(match config.occlusion.hide_in_cockpit {
                true => CheckBoxState::Checked,
                false => CheckBoxState::Unchecked,
            });
        self.sample_checkbox
            .set_check_state(match config.show_sample_data {
                true => CheckBoxState::Checked,
                false => CheckBoxState::Unchecked,
            });
    }

    fn save_config(&self) {
        if let Some(config) = &*self.config.borrow() {
            let config = &mut config.lock().unwrap();
            config.appearance.color = self.color_value.get();
            config.appearance.brightness =
                (self.brightness_input.pos() as f64 / (100.0 / 255.0)) as u8;
            config.occlusion.hide_on_hud =
                self.hide_on_hud_checkbox.check_state() == CheckBoxState::Checked;
            config.occlusion.hide_in_cockpit =
                self.hide_in_cockpit_checkbox.check_state() == CheckBoxState::Checked;
            config.show_sample_data = self.sample_checkbox.check_state() == CheckBoxState::Checked;
        }
    }

    fn set_installer_state(&self, install_button: &Button, status: &InstallStatus) {
        match status {
            InstallStatus::DCSNotFound => {
                install_button.set_enabled(false);
                install_button.set_text("Not Found");
            }
            InstallStatus::NotInstalled => {
                install_button.set_enabled(true);
                install_button.set_text("Install");
            }
            InstallStatus::RequiresUpdate => {
                install_button.set_enabled(true);
                install_button.set_text("Update");
            }
            InstallStatus::Installed => {
                install_button.set_enabled(true);
                install_button.set_text("Uninstall");
            }
        }
    }

    fn run_installer(&self, dcs_version: &DCSVersion) -> Result<()> {
        match dcs_version.install_status()? {
            InstallStatus::NotInstalled => installer::install(&dcs_version),
            InstallStatus::RequiresUpdate => {
                installer::uninstall(&dcs_version)?;
                installer::install(&dcs_version)
            }
            InstallStatus::Installed => installer::uninstall(&dcs_version),
            InstallStatus::DCSNotFound => Err(anyhow!("Cannot install in non existing DCS folder")),
        }
    }

    fn install_stable(&self) {
        self.run_installer(&DCSVersion::Stable).unwrap()
    }

    fn install_openbeta(&self) {
        self.run_installer(&DCSVersion::Openbeta).unwrap()
    }

    pub fn update_install_status(&self) {
        if let Ok(status) = DCSVersion::Stable.install_status() {
            self.set_installer_state(&self.install_stable_button, &status);
        }
        if let Ok(status) = DCSVersion::Openbeta.install_status() {
            self.set_installer_state(&self.install_openbeta_button, &status);
        }
    }

    pub fn set_config(&self, config: Option<ConfigHandle>) {
        if let Some(config) = &config {
            self.load_config(&config.lock().unwrap());
        }
        *self.config.borrow_mut() = config;
    }

    pub fn set_status_text(&self, status: &str) {
        self.status_text.set_text(&format!(" {}", status));
    }

    pub fn hwnd(&self) -> HWND {
        self.window.handle.hwnd().unwrap()
    }
}

pub use control_window_ui::ControlWindowUi;

pub fn create() -> Result<ControlWindowUi> {
    nwg::init().context("Failed to init Native Widnows GUI")?;
    nwg::Font::set_global_family("Segoe UI").context("Failed to set default font")?;
    Ok(ControlWindow::build_ui(Default::default()).context("Failed to build UI")?)
}
