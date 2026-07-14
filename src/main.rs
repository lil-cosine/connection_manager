// Prevent console window in addition to Slint window in Windows release builds when, e.g., starting the app via file manager. Ignored on other platforms.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod bluetooth;
mod wifi;

use bluetooth::*;
use slint::VecModel;
use std::error::Error;
use wifi::*;

slint::include_modules!();

fn show_error(ui: &AppWindow, err: String) {
    ui.set_error_text(err.into());
    ui.invoke_show_error_popup();
}

fn main() -> Result<(), Box<dyn Error>> {
    let ui = AppWindow::new()?;

    let ui_weak = ui.as_weak();
    ui.on_toggle_wifi(move || {
        if let Some(ui) = ui_weak.upgrade() {
            if let Err(e) = toggle_wifi(&ui) {
                show_error(&ui, e);
            }
        }
    });

    let ui_weak = ui.as_weak();
    ui.on_refresh_networks(move || {
        if let Some(ui) = ui_weak.upgrade() {
            if let Err(e) = cur_network(&ui) {
                show_error(&ui, e);
            }
            if let Err(e) = avail_networks(&ui) {
                show_error(&ui, e);
            }
            if let Err(e) = saved_networks(&ui) {
                show_error(&ui, e);
            }
        }
    });

    let ui_weak = ui.as_weak();
    ui.on_disconnect(move |ssid| {
        if let Some(ui) = ui_weak.upgrade() {
            if let Err(e) = disconnect_cur_network(&ssid) {
                show_error(&ui, e);
            }
            ui.invoke_refresh_networks();
        }
    });

    let ui_weak = ui.as_weak();
    ui.on_connect(move |ssid| {
        if let Some(ui) = ui_weak.upgrade() {
            match try_connect_known(ssid.clone()) {
                Ok(true) => ui.invoke_refresh_networks(),
                Ok(false) => {
                    ui.set_pending_ssid(ssid);
                    ui.set_password_error(false);
                    ui.set_password_error_text("".into());
                    ui.invoke_show_password_popup();
                }
                Err(e) => show_error(&ui, e),
            }
        }
    });

    let ui_weak = ui.as_weak();
    ui.on_connect_new(move |ssid, password| {
        if let Some(ui) = ui_weak.upgrade() {
            match connect_new_network(&ssid, password.as_ref()) {
                Ok(()) => {
                    ui.set_password_error(false);
                    ui.invoke_refresh_networks();
                }
                Err(_e) => {
                    if let Err(forget_err) = forget_network(&ssid) {
                        show_error(&ui, forget_err);
                    }
                    ui.set_password_error(true);
                    ui.set_password_error_text("Incorrect password, or connection failed.".into());
                }
            }
        }
    });

    let ui_weak = ui.as_weak();
    ui.on_forget(move |ssid| {
        if let Some(ui) = ui_weak.upgrade() {
            if let Err(e) = forget_network(&ssid) {
                show_error(&ui, e);
            }
            ui.invoke_refresh_networks();
        }
    });

    let ui_weak = ui.as_weak();
    ui.on_toggle_bluetooth(move || {
        if let Some(ui) = ui_weak.upgrade() {
            if let Err(e) = toggle_bluetooth(&ui) {
                show_error(&ui, e);
            }
        }
    });

    let ui_weak = ui.as_weak();
    ui.on_connect_known_device(move |device| {
        if let Some(ui) = ui_weak.upgrade() {
            if let Err(e) = on_connect_device(&device.mac_address) {
                show_error(&ui, e);
            }
            if let Err(e) = saved_devices(&ui) {
                show_error(&ui, e);
            }
        }
    });

    let ui_weak = ui.as_weak();
    ui.on_disconnect_known_device(move |device| {
        if let Some(ui) = ui_weak.upgrade() {
            if let Err(e) = on_disconnect_known_device(&device.mac_address) {
                show_error(&ui, e);
            }
            if let Err(e) = saved_devices(&ui) {
                show_error(&ui, e);
            }
        }
    });

    let ui_weak = ui.as_weak();
    ui.on_refresh_bluetooth(move || {
        if let Some(ui) = ui_weak.upgrade() {
            if let Err(e) = saved_devices(&ui) {
                show_error(&ui, e);
            }
        }

        let ui_weak = ui_weak.clone();
        std::thread::spawn(move || {
            if let Some(ui) = ui_weak.upgrade() {
                if let Err(e) = scan_new_devices(5) {
                    show_error(&ui, e);
                }
            }

            match get_new_devices() {
                Ok(devices) => {
                    let ui_weak = ui_weak.clone();
                    let _ = slint::invoke_from_event_loop(move || {
                        if let Some(ui) = ui_weak.upgrade() {
                            display_new_devices(&ui, VecModel::from(devices));
                        }
                    });
                }
                Err(e) => {
                    let ui_weak = ui_weak.clone();
                    let _ = slint::invoke_from_event_loop(move || {
                        if let Some(ui) = ui_weak.upgrade() {
                            show_error(&ui, e);
                        }
                    });
                }
            }
        });
    });

    let ui_weak = ui.as_weak();
    ui.on_connect_new_device(move |device| {
        if let Some(ui) = ui_weak.upgrade() {
            if let Err(e) = on_connect_new_device(&device.mac_address) {
                show_error(&ui, e);
            }
            if let Err(e) = saved_devices(&ui) {
                show_error(&ui, e);
            }
            if let Err(e) = scan_new_devices(2) {
                show_error(&ui, e);
            }
            if let Err(e) = new_devices(&ui) {
                show_error(&ui, e);
            }
        }
    });

    let ui_weak = ui.as_weak();
    ui.on_forget_device(move |device| {
        if let Some(ui) = ui_weak.upgrade() {
            if let Err(e) = on_forget_device(&device.mac_address) {
                show_error(&ui, e);
            }
            if let Err(e) = saved_devices(&ui) {
                show_error(&ui, e);
            }
            if let Err(e) = scan_new_devices(2) {
                show_error(&ui, e);
            }
            if let Err(e) = new_devices(&ui) {
                show_error(&ui, e);
            }
        }
    });

    let timer = slint::Timer::default();
    let ui_weak = ui.as_weak();
    timer.start(
        slint::TimerMode::Repeated,
        std::time::Duration::from_secs(15),
        move || {
            let ui_weak = ui_weak.clone();
            std::thread::spawn(move || {
                if let Some(ui) = ui_weak.upgrade() {
                    if let Err(e) = scan_new_devices(3) {
                        show_error(&ui, e);
                    }
                }

                match get_new_devices() {
                    Ok(devices) => {
                        let ui_weak = ui_weak.clone();
                        let _ = slint::invoke_from_event_loop(move || {
                            if let Some(ui) = ui_weak.upgrade() {
                                display_new_devices(&ui, VecModel::from(devices));
                            }
                        });
                    }
                    Err(e) => {
                        let ui_weak = ui_weak.clone();
                        let _ = slint::invoke_from_event_loop(move || {
                            if let Some(ui) = ui_weak.upgrade() {
                                show_error(&ui, e);
                            }
                        });
                    }
                }
            });
        },
    );

    if let Err(e) = set_wifi_on(&ui) {
        show_error(&ui, e);
    }
    if let Err(e) = cur_network(&ui) {
        show_error(&ui, e);
    }
    if let Err(e) = avail_networks(&ui) {
        show_error(&ui, e);
    }
    if let Err(e) = saved_networks(&ui) {
        show_error(&ui, e);
    }

    if let Err(e) = set_bluetooth_on(&ui) {
        show_error(&ui, e);
    }
    if let Err(e) = saved_devices(&ui) {
        show_error(&ui, e);
    }
    if let Err(e) = new_devices(&ui) {
        show_error(&ui, e);
    }

    ui.run()?;

    Ok(())
}
