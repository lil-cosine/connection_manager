use crate::AppWindow;
use slint::{ModelRc, SharedString, VecModel};
use std::collections::HashMap;
use std::process::Command;

#[derive(Debug)]
struct AccessPoint {
    ssid: String,
    signal: u8,
    in_use: bool,
}

// public functions

/// Checks whether the Wi-Fi radio is currently enabled and
/// updates the UI state accordingly.
///
/// In: `ui` - the app window to update.
/// Out: `Ok(())` on success, `Err(String)` if nmcli failed
/// to run (UI is still set to `false` in that case).
pub fn set_wifi_on(ui: &AppWindow) -> Result<(), String> {
    let output = Command::new("nmcli").args(["radio", "wifi"]).output();

    match output {
        Ok(o) => {
            let status = String::from_utf8_lossy(&o.stdout);
            ui.set_wifi_on(status.trim() == "enabled");
            Ok(())
        }
        Err(e) => {
            ui.set_wifi_on(false);
            Err(e.to_string())
        }
    }
}

/// Toggles the Wi-Fi radio on or off based on current UI
/// state, and updates the UI to reflect the new state.
///
/// In: `ui` - the app window to read/update state from.
/// Out: `Ok(())` on success, `Err(String)` if the toggle
/// command failed to run.
pub fn toggle_wifi(ui: &AppWindow) -> Result<(), String> {
    if ui.get_wifi_on() {
        let result = disable_wifi();
        match result {
            Ok(_) => {
                ui.set_wifi_on(false);
                Ok(())
            }
            Err(e) => Err(e),
        }
    } else {
        let result = enable_wifi();
        match result {
            Ok(_o) => {
                ui.set_wifi_on(true);
                Ok(())
            }
            Err(e) => Err(e),
        }
    }
}

/// Fetches the currently active Wi-Fi network and displays
/// it in the UI.
///
/// In: `ui` - the app window to update.
/// Out: `Ok(())` on success, `Err(String)` if nmcli failed.
pub fn cur_network(ui: &AppWindow) -> Result<(), String> {
    let network = get_cur_network()?;
    display_cur_network(ui, network);
    Ok(())
}

/// Fetches nearby available Wi-Fi networks and displays
/// them in the UI.
///
/// In: `ui` - the app window to update.
/// Out: `Ok(())` on success, `Err(String)` if nmcli failed.
pub fn avail_networks(ui: &AppWindow) -> Result<(), String> {
    let networks = VecModel::from(get_avail_networks()?);
    display_avail_networks(ui, networks);
    Ok(())
}

/// Fetches previously saved (known) Wi-Fi networks and
/// displays them in the UI.
///
/// In: `ui` - the app window to update.
/// Out: `Ok(())` on success, `Err(String)` if nmcli failed.
pub fn saved_networks(ui: &AppWindow) -> Result<(), String> {
    let networks = VecModel::from(get_saved_networks()?);
    display_saved_networks(ui, networks);
    Ok(())
}

/// Connects to a new (not yet saved) network using the
/// given SSID and password.
///
/// In: `ssid` - network name to connect to.
/// In: `password` - password for the network.
/// Out: `Ok(())` if the connection succeeded, `Err(String)`
/// with the failure reason otherwise.
pub fn connect_new_network(ssid: &SharedString, password: &str) -> Result<(), String> {
    let output = Command::new("nmcli")
        .args(["device", "wifi", "connect", ssid, "password", password])
        .output();

    match output {
        Ok(o) if o.status.success() => Ok(()),
        Ok(o) => {
            let err_msg = String::from_utf8_lossy(&o.stderr).to_string();
            Err(format!("nmcli exited with error: {err_msg}"))
        }
        Err(e) => Err(format!("nmcli exited with error: {e}")),
    }
}

/// Attempts to connect to an SSID only if it is already
/// saved as a known network.
///
/// In: `ssid` - network name to look for and connect to.
/// Out: `Ok(true)` if a matching saved network was found
/// and connected, `Ok(false)` if no match was found,
/// `Err(String)` if a command failed.
pub fn try_connect_known(ssid: SharedString) -> Result<bool, String> {
    let saved_ssids = get_saved_networks()?;
    for saved_ssid in saved_ssids.iter() {
        if *saved_ssid == ssid {
            connect_saved_network(&ssid)?;
            return Ok(true);
        }
    }
    Ok(false)
}

/// Disconnects from the given active network connection.
///
/// In: `ssid` - name of the connection to bring down.
/// Out: `Ok(())` on success, `Err(String)` if nmcli failed.
pub fn disconnect_cur_network(ssid: &SharedString) -> Result<(), String> {
    Command::new("nmcli")
        .args(["con", "down", ssid])
        .output()
        .map(|_| ())
        .map_err(|e| format!("failed to disconnect from current network: {e}"))
}

/// Removes a saved network connection profile entirely.
///
/// In: `ssid` - name of the connection to delete.
/// Out: `Ok(())` on success, `Err(String)` if nmcli failed.
pub fn forget_network(ssid: &SharedString) -> Result<(), String> {
    Command::new("nmcli")
        .args(["con", "delete", ssid])
        .output()
        .map(|_| ())
        .map_err(|e| format!("failed to forget network: {e}"))
}

// private functions

// Enables the Wi-Fi radio via nmcli.
//
// Out: Ok(()) on success, Err(String) if the command
// failed to run.
fn enable_wifi() -> Result<(), String> {
    Command::new("nmcli")
        .args(["radio", "wifi", "on"])
        .output()
        .map(|_| ())
        .map_err(|e| format!("failed to enable wifi: {e}"))
}

// Disables the Wi-Fi radio via nmcli.
//
// Out: Ok(()) on success, Err(String) if the command
// failed to run.
fn disable_wifi() -> Result<(), String> {
    Command::new("nmcli")
        .args(["radio", "wifi", "off"])
        .output()
        .map(|_| ())
        .map_err(|e| format!("failed to disable wifi: {e}"))
}

// Queries nmcli for the currently active Wi-Fi connection
// name, defaulting to "None" if nothing is connected.
//
// Out: Ok(SharedString) with the SSID or "None", Err(String)
// if nmcli failed to run or returned a nonzero exit code.
fn get_cur_network() -> Result<SharedString, String> {
    let output = Command::new("nmcli")
        .args(["-t", "-f", "ACTIVE,NAME,TYPE", "connection", "show"])
        .output();

    let networks = match output {
        Ok(o) if o.status.success() => o,
        Ok(o) => {
            let err_msg = String::from_utf8_lossy(&o.stderr);
            return Err(format!("nmcli exited with error: {err_msg}"));
        }
        Err(e) => {
            return Err(format!("failed to run nmcli: {e}"));
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

    Ok(wifi_networks
        .first()
        .map(|s| SharedString::from(*s))
        .unwrap_or_else(|| SharedString::from("None")))
}

// Queries nmcli for all saved (known, not currently active)
// Wi-Fi connection profiles.
//
// Out: Ok(Vec<SharedString>) of saved SSIDs, Err(String) if
// nmcli failed to run or returned a nonzero exit code.
fn get_saved_networks() -> Result<Vec<SharedString>, String> {
    let networks = Command::new("nmcli")
        .args(["-t", "-f", "ACTIVE,NAME,TYPE", "connection", "show"])
        .output()
        .map_err(|e| format!("failed to disconnect from current network: {e}"))?;

    if !networks.status.success() {
        return Err(String::from_utf8_lossy(&networks.stderr).to_string());
    }

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

    Ok(wifi_networks
        .into_iter()
        .map(SharedString::from)
        .collect::<Vec<_>>())
}

// Scans for nearby Wi-Fi access points, filters weak or
// duplicate entries, and returns them sorted by signal
// strength (strongest first), excluding the in-use network.
//
// Out: Ok(Vec<SharedString>) of available SSIDs, Err(String)
// if nmcli failed to run or returned a nonzero exit code.
fn get_avail_networks() -> Result<Vec<SharedString>, String> {
    let networks = Command::new("nmcli")
        .args(["-t", "-f", "IN-USE,SSID,SIGNAL", "dev", "wifi", "list"])
        .output()
        .map_err(|e| format!("failed to disconnect from current network: {e}"))?;

    if !networks.status.success() {
        return Err(String::from_utf8_lossy(&networks.stderr).to_string());
    }

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

    Ok(sorted_vec
        .iter()
        .map(|ap| SharedString::from(ap.ssid.clone()))
        .collect())
}

// Brings up an existing (already saved) network connection.
//
// In: ssid - name of the saved connection to activate.
// Out: Ok(()) on success, Err(String) if nmcli failed.
fn connect_saved_network(ssid: &SharedString) -> Result<(), String> {
    Command::new("nmcli")
        .args(["con", "up", ssid])
        .output()
        .map(|_| ())
        .map_err(|e| format!("failed to connect to saved network: {e}"))
}

// Sets the current network SSID field in the UI.
//
// In: ui - the app window to update.
// In: ssid - the SSID string to display.
fn display_cur_network(ui: &AppWindow, ssid: SharedString) {
    ui.set_curnet(ssid);
}

// Sets the list of available networks in the UI.
//
// In: ui - the app window to update.
// In: ssids - model of available network SSIDs.
fn display_avail_networks(ui: &AppWindow, ssids: VecModel<SharedString>) {
    ui.set_avail_networks(ModelRc::new(ssids));
}

// Sets the list of saved networks in the UI.
//
// In: ui - the app window to update.
// In: ssids - model of saved network SSIDs.
fn display_saved_networks(ui: &AppWindow, ssids: VecModel<SharedString>) {
    ui.set_networks(ModelRc::new(ssids));
}
