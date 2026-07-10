// Prevent console window in addition to Slint window in Windows release builds when, e.g., starting the app via file manager. Ignored on other platforms.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use slint::{ModelRc, SharedString, VecModel};
use std::collections::HashMap;
use std::error::Error;
use std::process::Command;

slint::include_modules!();

#[derive(Debug)]

struct AccessPoint {
    ssid: String,
    signal: u8,
    in_use: bool,
}

fn main() -> Result<(), Box<dyn Error>> {
    let ui = AppWindow::new()?;

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
    ui.on_toggle_wifi(move || {
        if let Some(ui) = ui_weak.upgrade() {
            toggle_wifi(&ui);
        }
    });

    let ui_weak = ui.as_weak();
    ui.on_toggle_bluetooth(move || {
        if let Some(ui) = ui_weak.upgrade() {
            toggle_bluetooth(&ui);
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
            scan_new_devices(5);
            new_devices(&ui);
        }
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

    set_wifi_on(&ui);
    cur_network(&ui);
    avail_networks(&ui);
    saved_networks(&ui);
    set_bluetooth_on(&ui);
    saved_devices(&ui);
    scan_new_devices(2);
    new_devices(&ui);

    ui.run()?;

    Ok(())
}

// Model
fn try_connect_known(ssid: SharedString) -> bool {
    let saved_ssids: Vec<SharedString> = get_saved_networks();
    for saved_ssid in saved_ssids.iter() {
        if *saved_ssid == ssid {
            connect_saved_network(&ssid);
            return true;
        }
    }
    false
}

fn connect_saved_network(ssid: &SharedString) {
    Command::new("nmcli")
        .args(["con", "up", ssid])
        .output()
        .expect("failed to connect to saved networks");
}

fn connect_new_network(ssid: &SharedString, password: &str) -> bool {
    let output = Command::new("nmcli")
        .args(["device", "wifi", "connect", ssid, "password", password])
        .output();

    match output {
        Ok(o) if o.status.success() => true,
        Ok(o) => {
            eprintln!(
                "nmcli exited with error: {}",
                String::from_utf8_lossy(&o.stderr)
            );
            false
        }
        Err(e) => {
            eprintln!("failed to run nmcli: {e}");
            false
        }
    }
}

fn disconnect_cur_network(ssid: &SharedString) {
    Command::new("nmcli")
        .args(["con", "down", ssid])
        .output()
        .expect("failed to disconnect from current network");
}

fn forget_network(ssid: &SharedString) {
    Command::new("nmcli")
        .args(["con", "delete", ssid])
        .output()
        .expect("failed to forget network");
}

fn get_cur_network() -> Option<SharedString> {
    let output = Command::new("nmcli")
        .args(["-t", "-f", "ACTIVE,NAME,TYPE", "connection", "show"])
        .output();

    let networks = match output {
        Ok(o) if o.status.success() => o,
        Ok(o) => {
            eprintln!(
                "nmcli exited with error: {}",
                String::from_utf8_lossy(&o.stderr)
            );
            return None;
        }
        Err(e) => {
            eprintln!("failed to run nmcli: {e}");
            return None;
        }
    };

    let stdout = String::from_utf8_lossy(&networks.stdout);

    let wifi_networks: Vec<&str> = stdout
        .lines()
        .filter_map(|line| {
            let mut result = line.split(":");
            let active = result.next()?;
            let name = result.next()?;
            let ty = result.next()?;

            (ty == "802-11-wireless" && active == "yes").then_some(name)
        })
        .collect();

    wifi_networks.first().map(|s| SharedString::from(*s))
}

fn get_avail_networks() -> Vec<SharedString> {
    let networks = Command::new("nmcli")
        .args(["-t", "-f", "IN-USE,SSID,SIGNAL", "dev", "wifi", "list"])
        .output()
        .expect("failed to run nmcli");

    let networks = String::from_utf8_lossy(&networks.stdout);

    let mut wifi_networks: Vec<&str> = networks.lines().collect();
    wifi_networks.sort();
    wifi_networks.dedup();

    let mut access_points: Vec<AccessPoint> = vec![];
    for net in wifi_networks {
        let split: Vec<&str> = net.split(':').collect();
        if split[1].is_empty() || split[2].parse::<u8>().unwrap() < 35 {
            continue;
        }
        access_points.push(AccessPoint {
            ssid: (split[1].to_string()),
            signal: (split[2].parse::<u8>().unwrap()),
            in_use: (split[0] == "*"),
        });
    }

    let mut filtered_access_points: HashMap<String, AccessPoint> = HashMap::new();

    for point in access_points {
        if !filtered_access_points.contains_key(&point.ssid) {
            filtered_access_points.insert(point.ssid.clone(), point);
        } else if point.in_use {
            filtered_access_points.remove(&point.ssid);
            filtered_access_points.insert(point.ssid.clone(), point);
        } else if filtered_access_points.get(&point.ssid).unwrap().in_use {
            continue;
        } else if filtered_access_points.get(&point.ssid).unwrap().signal < point.signal {
            filtered_access_points.remove(&point.ssid);
            filtered_access_points.insert(point.ssid.clone(), point);
        }
    }

    let vec_of_ap = filtered_access_points.values().collect::<Vec<_>>();

    let mut sorted_vec = vec_of_ap
        .into_iter()
        .filter(|ap| !ap.in_use)
        .collect::<Vec<_>>();

    sorted_vec.sort_by(|a, b| b.signal.cmp(&a.signal));

    sorted_vec
        .iter()
        .map(|ap| SharedString::from(ap.ssid.clone()))
        .collect()
}

fn get_saved_networks() -> Vec<SharedString> {
    let networks = Command::new("nmcli")
        .args(["-t", "-f", "ACTIVE,NAME,TYPE", "connection", "show"])
        .output()
        .expect("failed to run nmcli");

    let stdout = String::from_utf8_lossy(&networks.stdout);

    let wifi_networks: Vec<&str> = stdout
        .lines()
        .filter_map(|line| {
            let mut result = line.split(":");
            let active = result.next()?;
            let name = result.next()?;
            let ty = result.next()?;

            (ty == "802-11-wireless" && active == "no").then_some(name)
        })
        .collect();

    wifi_networks
        .into_iter()
        .map(SharedString::from)
        .collect::<Vec<_>>()
}

fn enable_wifi() {
    Command::new("nmcli")
        .args(["radio", "wifi", "on"])
        .output()
        .expect("unable to enable wifi");
}

fn disable_wifi() {
    Command::new("nmcli")
        .args(["radio", "wifi", "off"])
        .output()
        .expect("unable to disable wifi");
}

fn toggle_wifi(ui: &AppWindow) {
    if ui.get_wifi_on() {
        disable_wifi();
        ui.set_wifi_on(false);
    } else {
        enable_wifi();
        ui.set_wifi_on(true);
    }
}

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

fn toggle_bluetooth(ui: &AppWindow) {
    if ui.get_bluetooth_on() {
        disable_bluetooth();
        ui.set_bluetooth_on(false);
    } else {
        enable_bluetooth();
        ui.set_bluetooth_on(true);
    }
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

            if name.is_empty() || is_nameless(&name, &mac_address) {
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

fn scan_new_devices(timeout: i32) {
    Command::new("bluetoothctl")
        .args(["--timeout", &timeout.to_string(), "scan", "on"])
        .output()
        .expect("failed to run bluetoothctl");
}

fn get_new_devices() -> Vec<BluetoothDevice> {
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

            if name.is_empty() || is_nameless(&name, &mac_address) || is_device_paired(&mac_address)
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

fn is_nameless(name: &str, mac_address: &str) -> bool {
    if name == mac_address {
        return true;
    }
    if name.len() == 17 && name.replace('-', ":") == mac_address {
        return true;
    }

    false
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

fn on_connect_device(mac_address: &str) {
    Command::new("bluetoothctl")
        .args(["connect", mac_address])
        .output()
        .expect("unable to connect to device");
}

fn on_trust_device(mac_address: &str) {
    Command::new("bluetoothctl")
        .args(["trust", mac_address])
        .output()
        .expect("unable to trust new device");
}

fn on_forget_device(mac_address: &str) {
    Command::new("bluetoothctl")
        .args(["remove", mac_address])
        .output()
        .expect("unable to forget device");
}

fn on_connect_new_device(mac_address: &str) {
    on_connect_device(mac_address);
    on_trust_device(mac_address);
}

fn on_disconnect_known_device(mac_address: &str) {
    Command::new("bluetoothctl")
        .args(["disconnect", mac_address])
        .output()
        .expect("failed to run bluetoothctl");
}

// Display
fn display_cur_network(ui: &AppWindow, ssid: SharedString) {
    ui.set_curnet(ssid);
}

fn display_avail_networks(ui: &AppWindow, ssids: VecModel<SharedString>) {
    ui.set_avail_networks(ModelRc::new(ssids));
}

fn display_saved_networks(ui: &AppWindow, ssids: VecModel<SharedString>) {
    ui.set_networks(ModelRc::new(ssids));
}

fn display_saved_devices(ui: &AppWindow, devices: VecModel<BluetoothDevice>) {
    ui.set_saved_devices(ModelRc::new(devices));
}

fn display_new_devices(ui: &AppWindow, devices: VecModel<BluetoothDevice>) {
    ui.set_new_devices(ModelRc::new(devices));
}

// Mains
fn cur_network(ui: &AppWindow) {
    let network: SharedString = get_cur_network().unwrap_or_else(|| SharedString::from("None"));
    display_cur_network(ui, network);
}

fn avail_networks(ui: &AppWindow) {
    let networks: VecModel<SharedString> = VecModel::from(get_avail_networks());
    display_avail_networks(ui, networks);
}

fn saved_networks(ui: &AppWindow) {
    let networks: VecModel<SharedString> = VecModel::from(get_saved_networks());
    display_saved_networks(ui, networks);
}

fn set_wifi_on(ui: &AppWindow) {
    let output = Command::new("nmcli").args(["radio", "wifi"]).output();

    match output {
        Ok(o) => {
            let status = String::from_utf8_lossy(&o.stdout);
            ui.set_wifi_on(status.trim() == "enabled");
        }
        Err(e) => {
            eprintln!("failed to run nmcli: {e}");
            ui.set_wifi_on(false);
        }
    }
}

fn set_bluetooth_on(ui: &AppWindow) {
    let output = Command::new("bluetoothctl")
        .args(["--timeout", "0", "scan", "on"])
        .output();

    match output {
        Ok(o) if o.status.success() => ui.set_bluetooth_on(true),
        Ok(_o) => ui.set_bluetooth_on(false),
        Err(e) => {
            eprintln!("failed to run bluetoothctl: {e}");
            ui.set_bluetooth_on(false);
        }
    }
}

fn saved_devices(ui: &AppWindow) {
    let devices: VecModel<BluetoothDevice> = VecModel::from(get_saved_devices());
    display_saved_devices(ui, devices);
}

fn new_devices(ui: &AppWindow) {
    let devices: VecModel<BluetoothDevice> = VecModel::from(get_new_devices());
    display_new_devices(ui, devices);
}
