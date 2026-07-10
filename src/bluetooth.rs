use crate::AppWindow;
use crate::BluetoothDevice;
use slint::{ModelRc, VecModel};
use std::process::Command;

// public functions
pub fn toggle_bluetooth(ui: &AppWindow) {
    if ui.get_bluetooth_on() {
        disable_bluetooth();
        ui.set_bluetooth_on(false);
    } else {
        enable_bluetooth();
        ui.set_bluetooth_on(true);
    }
}

pub fn set_bluetooth_on(ui: &AppWindow) {
    let output = Command::new("bluetoothctl").args(["show"]).output();

    let powered = match output {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout)
            .lines()
            .find_map(|l| l.trim().strip_prefix("Powered:"))
            .map(|v| v.trim() == "yes")
            .unwrap_or(false),
        _ => false,
    };

    ui.set_bluetooth_on(powered);
}

pub fn saved_devices(ui: &AppWindow) {
    let devices: VecModel<BluetoothDevice> = VecModel::from(get_saved_devices());
    display_saved_devices(ui, devices);
}

pub fn new_devices(ui: &AppWindow) {
    let devices: VecModel<BluetoothDevice> = VecModel::from(get_new_devices());
    display_new_devices(ui, devices);
}

pub fn get_new_devices() -> Vec<BluetoothDevice> {
    let devices = Command::new("bluetoothctl")
        .args(["devices"])
        .output()
        .expect("failed to run bluetoothctl");

    let stdout = String::from_utf8_lossy(&devices.stdout);

    let new_devices: Vec<BluetoothDevice> = stdout
        .lines()
        .filter_map(|line| {
            let mut result = line.split_whitespace();
            result.next()?;
            let mac_address = result.next()?.to_string();
            let name = result.collect::<Vec<_>>().join(" ");

            if name.is_empty()
                || is_nameless(&name, &mac_address)
                || is_device_paired(&mac_address)
                || !is_valid_mac(&mac_address)
            {
                return None;
            };
            Some(BluetoothDevice {
                name: name.into(),
                connected: false,
                mac_address: mac_address.into(),
            })
        })
        .collect();

    new_devices
}

pub fn on_connect_device(mac_address: &str) {
    Command::new("bluetoothctl")
        .args(["connect", mac_address])
        .output()
        .expect("unable to connect to device");
}

pub fn on_connect_new_device(mac_address: &str) {
    on_connect_device(mac_address);
    on_trust_device(mac_address);
}

pub fn on_disconnect_known_device(mac_address: &str) {
    Command::new("bluetoothctl")
        .args(["disconnect", mac_address])
        .output()
        .expect("failed to run bluetoothctl");
}

pub fn on_forget_device(mac_address: &str) {
    Command::new("bluetoothctl")
        .args(["remove", mac_address])
        .output()
        .expect("unable to forget device");
}

pub fn scan_new_devices(timeout: i32) {
    Command::new("bluetoothctl")
        .args(["--timeout", &timeout.to_string(), "scan", "on"])
        .output()
        .expect("failed to run bluetoothctl");
}

pub fn display_new_devices(ui: &AppWindow, devices: VecModel<BluetoothDevice>) {
    ui.set_new_devices(ModelRc::new(devices));
}

// private functions
fn enable_bluetooth() {
    Command::new("bluetoothctl")
        .args(["power", "on"])
        .output()
        .expect("unable to enable bluetooth");
}

fn disable_bluetooth() {
    Command::new("bluetoothctl")
        .args(["power", "off"])
        .output()
        .expect("unable to disable bluetooth");
}

fn get_saved_devices() -> Vec<BluetoothDevice> {
    let devices = Command::new("bluetoothctl")
        .args(["devices", "Paired"])
        .output()
        .expect("failed to run bluetoothctl");

    let stdout = String::from_utf8_lossy(&devices.stdout);

    let saved_devices: Vec<BluetoothDevice> = stdout
        .lines()
        .filter_map(|line| {
            let mut result = line.split_whitespace();
            result.next()?;
            let mac_address = result.next()?.to_string();
            let name = result.collect::<Vec<_>>().join(" ");

            if name.is_empty() || is_nameless(&name, &mac_address) || !is_valid_mac(&mac_address) {
                return None;
            };
            Some(BluetoothDevice {
                name: name.into(),
                connected: is_device_connected(mac_address.as_str()),
                mac_address: mac_address.into(),
            })
        })
        .collect();

    saved_devices
}

fn is_nameless(name: &str, mac_address: &str) -> bool {
    if name == mac_address {
        return true;
    }
    if name.len() == 17 && name.replace('-', ":") == mac_address {
        return true;
    }

    false
}

fn is_valid_mac(mac_address: &str) -> bool {
    mac_address.len() == 17
        && mac_address
            .as_bytes()
            .chunks(3)
            .enumerate()
            .all(|(i, chunk)| {
                if i < 5 {
                    chunk.len() == 3
                        && chunk[2] == b':'
                        && chunk[0].is_ascii_hexdigit()
                        && chunk[1].is_ascii_hexdigit()
                } else {
                    chunk.len() == 2 && chunk[0].is_ascii_hexdigit() && chunk[1].is_ascii_hexdigit()
                }
            })
}

fn is_device_connected(mac_address: &str) -> bool {
    let info = Command::new("bluetoothctl")
        .args(["info", mac_address])
        .output();
    let output = match info {
        Ok(o) if o.status.success() => o,
        Ok(o) => {
            eprintln!(
                "bluetoothctl exited with error: {}",
                String::from_utf8_lossy(&o.stderr)
            );
            return false;
        }
        Err(e) => {
            eprintln!("failed to run bluetoothctl: {e}");
            return false;
        }
    };

    let stdout = String::from_utf8_lossy(&output.stdout);

    stdout
        .lines()
        .find_map(|line| line.trim().strip_prefix("Connected:"))
        .map(|value| value.trim() == "yes")
        .unwrap_or(false)
}

fn is_device_paired(mac_address: &str) -> bool {
    let info = Command::new("bluetoothctl")
        .args(["info", mac_address])
        .output();
    let output = match info {
        Ok(o) if o.status.success() => o,
        Ok(o) => {
            eprintln!(
                "bluetoothctl exited with error: {}",
                String::from_utf8_lossy(&o.stderr)
            );
            return false;
        }
        Err(e) => {
            eprintln!("failed to run bluetoothctl: {e}");
            return false;
        }
    };

    let stdout = String::from_utf8_lossy(&output.stdout);

    stdout
        .lines()
        .find_map(|line| line.trim().strip_prefix("Paired:"))
        .map(|value| value.trim() == "yes")
        .unwrap_or(false)
}

fn on_trust_device(mac_address: &str) {
    Command::new("bluetoothctl")
        .args(["trust", mac_address])
        .output()
        .expect("unable to trust new device");
}

fn display_saved_devices(ui: &AppWindow, devices: VecModel<BluetoothDevice>) {
    ui.set_saved_devices(ModelRc::new(devices));
}
