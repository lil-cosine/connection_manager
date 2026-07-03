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
                ui.invoke_show_password_popup();
            }
        }
    });

    let ui_weak = ui.as_weak();
    ui.on_connect_new(move |ssid, password| {
        connect_new_network(&ssid, password.as_ref());
        if let Some(ui) = ui_weak.upgrade() {
            ui.invoke_refresh_networks();
        }
    });

    let ui_weak = ui.as_weak();
    ui.on_forget(move |ssid| {
        forget_network(&ssid);
        if let Some(ui) = ui_weak.upgrade() {
            ui.invoke_refresh_networks();
        }
    });

    cur_network(&ui);
    avail_networks(&ui);
    saved_networks(&ui);

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

// TODO handle wrong passwords
fn connect_new_network(ssid: &SharedString, password: &str) {
    Command::new("nmcli")
        .args(["device", "wifi", "connect", ssid, "password", password])
        .output()
        .expect("failed to connect to new network");
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
