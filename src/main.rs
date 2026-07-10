// Prevent console window in addition to Slint window in Windows release builds when, e.g., starting the app via file manager. Ignored on other platforms.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod bluetooth;
mod wifi;

use bluetooth::*;
use slint::VecModel;
use std::error::Error;
use wifi::*;

slint::include_modules!();

fn main() -> Result<(), Box<dyn Error>> {
    let ui = AppWindow::new()?;

    let ui_weak = ui.as_weak();
    ui.on_toggle_wifi(move || {
        if let Some(ui) = ui_weak.upgrade() {
            toggle_wifi(&ui);
        }
    });

    let ui_weak = ui.as_weak();
    ui.on_refresh_networks(move || {
        if let Some(ui) = ui_weak.upgrade() {
            cur_network(&ui);
            avail_networks(&ui);
            saved_networks(&ui);
        }
    });

    let ui_weak = ui.as_weak();
    ui.on_disconnect(move |ssid| {
        disconnect_cur_network(&ssid);
        if let Some(ui) = ui_weak.upgrade() {
            ui.invoke_refresh_networks();
        }
    });

    let ui_weak = ui.as_weak();
    ui.on_connect(move |ssid| {
        let found = try_connect_known(ssid.clone());

        if let Some(ui) = ui_weak.upgrade() {
            if found {
                ui.invoke_refresh_networks();
            } else {
                ui.set_pending_ssid(ssid);
                ui.set_password_error(false);
                ui.set_password_error_text("".into());
                ui.invoke_show_password_popup();
            }
        }
    });

    let ui_weak = ui.as_weak();
    ui.on_connect_new(move |ssid, password| {
        if let Some(ui) = ui_weak.upgrade() {
            if connect_new_network(&ssid, password.as_ref()) {
                ui.set_password_error(false);
                ui.invoke_refresh_networks();
            } else {
                forget_network(&ssid); // removes the network from the saved list
                ui.set_password_error(true);
                ui.set_password_error_text("Incorrect password, or connection failed.".into());
            }
        }
    });

    let ui_weak = ui.as_weak();
    ui.on_forget(move |ssid| {
        forget_network(&ssid);
        if let Some(ui) = ui_weak.upgrade() {
            ui.invoke_refresh_networks();
        }
    });

    let ui_weak = ui.as_weak();
    ui.on_toggle_bluetooth(move || {
        if let Some(ui) = ui_weak.upgrade() {
            toggle_bluetooth(&ui);
        }
    });

    let ui_weak = ui.as_weak();
    ui.on_connect_known_device(move |device| {
        on_connect_device(&device.mac_address);
        if let Some(ui) = ui_weak.upgrade() {
            saved_devices(&ui);
        }
    });

    let ui_weak = ui.as_weak();
    ui.on_disconnect_known_device(move |device| {
        on_disconnect_known_device(&device.mac_address);
        if let Some(ui) = ui_weak.upgrade() {
            saved_devices(&ui);
        }
    });

    let ui_weak = ui.as_weak();
    ui.on_refresh_bluetooth(move || {
        if let Some(ui) = ui_weak.upgrade() {
            saved_devices(&ui);
        }

        let ui_weak = ui_weak.clone();
        std::thread::spawn(move || {
            scan_new_devices(5);
            let devices = get_new_devices();

            let _ = slint::invoke_from_event_loop(move || {
                if let Some(ui) = ui_weak.upgrade() {
                    display_new_devices(&ui, VecModel::from(devices));
                }
            });
        });
    });

    let ui_weak = ui.as_weak();
    ui.on_connect_new_device(move |device| {
        on_connect_new_device(&device.mac_address);
        if let Some(ui) = ui_weak.upgrade() {
            saved_devices(&ui);
            scan_new_devices(2);
            new_devices(&ui);
        }
    });

    let ui_weak = ui.as_weak();
    ui.on_forget_device(move |device| {
        on_forget_device(&device.mac_address);
        if let Some(ui) = ui_weak.upgrade() {
            saved_devices(&ui);
            scan_new_devices(2);
            new_devices(&ui);
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
                scan_new_devices(3);
                let devices = get_new_devices();
                let _ = slint::invoke_from_event_loop(move || {
                    if let Some(ui) = ui_weak.upgrade() {
                        display_new_devices(&ui, VecModel::from(devices));
                    }
                });
            });
        },
    );

    set_wifi_on(&ui);
    cur_network(&ui);
    avail_networks(&ui);
    saved_networks(&ui);

    set_bluetooth_on(&ui);
    saved_devices(&ui);
    new_devices(&ui);

    ui.run()?;

    Ok(())
}
