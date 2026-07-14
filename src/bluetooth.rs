use crate::AppWindow;
use crate::BluetoothDevice;
use slint::{ModelRc, VecModel};
use std::process::Command;

// public functions

/// Toggles Bluetooth power on or off based on current UI
/// state, and updates the UI to reflect the new state.
///
/// In: `ui` - the app window to read/update state from.
/// Out: `Ok(())` on success, `Err(String)` if the toggle
/// command failed to run.
pub fn toggle_bluetooth(ui: &AppWindow) -> Result<(), String> {
    if ui.get_bluetooth_on() {
        let result = disable_bluetooth();
        match result {
            Ok(_) => {
                ui.set_bluetooth_on(false);
                Ok(())
            }
            Err(e) => Err(e),
        }
    } else {
        let result = enable_bluetooth();
        match result {
            Ok(_) => {
                ui.set_bluetooth_on(true);
                Ok(())
            }
            Err(e) => Err(e),
        }
    }
}

/// Checks whether the Bluetooth adapter is currently powered
/// on and updates the UI state accordingly.
///
/// In: `ui` - the app window to update.
/// Out: `Ok(())` on success, `Err(String)` if bluetoothctl
/// failed to run (UI defaults to `false` on any failure).
pub fn set_bluetooth_on(ui: &AppWindow) -> Result<(), String> {
    let output = Command::new("bluetoothctl").args(["show"]).output();

    let powered = match output {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout)
            .lines()
            .find_map(|l| l.trim().strip_prefix("Powered:"))
            .map(|v| v.trim() == "yes")
            .unwrap_or(false),
        Ok(_) => false,
        Err(e) => return Err(e.to_string()),
    };

    ui.set_bluetooth_on(powered);
    Ok(())
}

/// Fetches saved (paired) Bluetooth devices and displays
/// them in the UI.
///
/// In: `ui` - the app window to update.
/// Out: `Ok(())` on success, `Err(String)` if bluetoothctl
/// failed.
pub fn saved_devices(ui: &AppWindow) -> Result<(), String> {
    let devices: VecModel<BluetoothDevice> = VecModel::from(get_saved_devices()?);
    display_saved_devices(ui, devices);
    Ok(())
}

/// Fetches newly discovered (unpaired) Bluetooth devices
/// and displays them in the UI.
///
/// In: `ui` - the app window to update.
/// Out: `Ok(())` on success, `Err(String)` if bluetoothctl
/// failed.
pub fn new_devices(ui: &AppWindow) -> Result<(), String> {
    let devices: VecModel<BluetoothDevice> = VecModel::from(get_new_devices()?);
    display_new_devices(ui, devices);
    Ok(())
}

/// Lists nearby Bluetooth devices from a scan, filtering out
/// nameless, invalid, or already-paired devices.
///
/// Out: `Ok(Vec<BluetoothDevice>)` of unpaired devices found,
/// `Err(String)` if bluetoothctl failed to run.
pub fn get_new_devices() -> Result<Vec<BluetoothDevice>, String> {
    let devices = Command::new("bluetoothctl")
        .args(["devices"])
        .output()
        .map_err(|e| format!("failed to run bluetoothctl: {e}"))?;

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
                || is_device_paired(&mac_address).ok()?
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

    Ok(new_devices)
}

/// Connects to a device that is already known/paired.
///
/// In: `mac_address` - MAC address of the target device.
/// Out: `Ok(())` on success, `Err(String)` if bluetoothctl
/// failed.
pub fn on_connect_device(mac_address: &str) -> Result<(), String> {
    Command::new("bluetoothctl")
        .args(["connect", mac_address])
        .output()
        .map(|_| ())
        .map_err(|e| format!("unable to connect to device: {e}"))
}

/// Connects to and trusts a new (previously unpaired) device.
///
/// In: `mac_address` - MAC address of the target device.
/// Out: `Ok(())` if both steps succeeded, `Err(String)` if
/// either the connect or trust step failed.
pub fn on_connect_new_device(mac_address: &str) -> Result<(), String> {
    on_connect_device(mac_address)?;
    on_trust_device(mac_address)?;
    Ok(())
}

/// Disconnects an already-connected known device.
///
/// In: `mac_address` - MAC address of the target device.
/// Out: `Ok(())` on success, `Err(String)` if bluetoothctl
/// failed.
pub fn on_disconnect_known_device(mac_address: &str) -> Result<(), String> {
    Command::new("bluetoothctl")
        .args(["disconnect", mac_address])
        .output()
        .map(|_| ())
        .map_err(|e| format!("failed to run bluetoothctl: {e}"))
}

/// Removes (unpairs) a device entirely.
///
/// In: `mac_address` - MAC address of the target device.
/// Out: `Ok(())` on success, `Err(String)` if bluetoothctl
/// failed.
pub fn on_forget_device(mac_address: &str) -> Result<(), String> {
    Command::new("bluetoothctl")
        .args(["remove", mac_address])
        .output()
        .map(|_| ())
        .map_err(|e| format!("unable to forget device: {e}"))
}

/// Scans for nearby Bluetooth devices for a fixed duration.
///
/// In: `timeout` - scan duration in seconds.
/// Out: `Ok(())` on success, `Err(String)` if bluetoothctl
/// failed.
pub fn scan_new_devices(timeout: i32) -> Result<(), String> {
    Command::new("bluetoothctl")
        .args(["--timeout", &timeout.to_string(), "scan", "on"])
        .output()
        .map(|_| ())
        .map_err(|e| format!("failed to run bluetoothctl: {e}"))
}

/// Sets the list of newly discovered devices in the UI.
///
/// In: `ui` - the app window to update.
/// In: `devices` - model of discovered devices to display.
pub fn display_new_devices(ui: &AppWindow, devices: VecModel<BluetoothDevice>) {
    ui.set_new_devices(ModelRc::new(devices));
}

// private functions

// Powers on the Bluetooth adapter via bluetoothctl.
//
// Out: Ok(()) on success, Err(String) if the command
// failed to run.
fn enable_bluetooth() -> Result<(), String> {
    Command::new("bluetoothctl")
        .args(["power", "on"])
        .output()
        .map(|_| ())
        .map_err(|e| format!("failed to enable bluetooth: {e}"))
}

// Powers off the Bluetooth adapter via bluetoothctl.
//
// Out: Ok(()) on success, Err(String) if the command
// failed to run.
fn disable_bluetooth() -> Result<(), String> {
    Command::new("bluetoothctl")
        .args(["power", "off"])
        .output()
        .map(|_| ())
        .map_err(|e| format!("failed to disable bluetooth: {e}"))
}

// Lists paired Bluetooth devices and their connection
// status, filtering out nameless or invalid entries.
//
// Out: Ok(Vec<BluetoothDevice>) of paired devices,
// Err(String) if bluetoothctl failed to run.
fn get_saved_devices() -> Result<Vec<BluetoothDevice>, String> {
    let devices = Command::new("bluetoothctl")
        .args(["devices", "Paired"])
        .output()
        .map_err(|e| format!("failed to run bluetoothctl: {e}"))?;

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
                connected: is_device_connected(mac_address.as_str()).ok()?,
                mac_address: mac_address.into(),
            })
        })
        .collect();

    Ok(saved_devices)
}

// Checks whether a device's name is just its MAC address
// in disguise (no real friendly name was set).
//
// In: name - the device's reported name.
// In: mac_address - the device's MAC address.
// Out: true if the name is effectively just the MAC address.
fn is_nameless(name: &str, mac_address: &str) -> bool {
    if name == mac_address {
        return true;
    }
    if name.len() == 17 && name.replace('-', ":") == mac_address {
        return true;
    }

    false
}

// Validates that a string is a well-formed MAC address
// in colon-separated hex format (e.g. "AA:BB:CC:DD:EE:FF").
//
// In: mac_address - the string to validate.
// Out: true if the string is a valid MAC address format.
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

// Checks whether a device is currently connected.
//
// In: mac_address - MAC address of the device to check.
// Out: Ok(bool) connection status, Err(String) if
// bluetoothctl failed to run or returned a nonzero exit.
fn is_device_connected(mac_address: &str) -> Result<bool, String> {
    let info = Command::new("bluetoothctl")
        .args(["info", mac_address])
        .output();
    let output = match info {
        Ok(o) if o.status.success() => o,
        Ok(o) => {
            let err_msg = String::from_utf8_lossy(&o.stderr);
            return Err(format!("bluetoothctl exited with error: {err_msg}"));
        }
        Err(e) => {
            return Err(format!("bluetoothctl exited with error: {e}"));
        }
    };

    let stdout = String::from_utf8_lossy(&output.stdout);

    Ok(stdout
        .lines()
        .find_map(|line| line.trim().strip_prefix("Connected:"))
        .map(|value| value.trim() == "yes")
        .unwrap_or(false))
}

// Checks whether a device is currently paired.
//
// In: mac_address - MAC address of the device to check.
// Out: Ok(bool) paired status, Err(String) if bluetoothctl
// failed to run or returned a nonzero exit code.
fn is_device_paired(mac_address: &str) -> Result<bool, String> {
    let info = Command::new("bluetoothctl")
        .args(["info", mac_address])
        .output();
    let output = match info {
        Ok(o) if o.status.success() => o,
        Ok(o) => {
            let err_msg = String::from_utf8_lossy(&o.stderr);
            return Err(format!("bluetoothctl exited with error: {err_msg}"));
        }
        Err(e) => {
            return Err(format!("bluetoothctl exited with error: {e}"));
        }
    };

    let stdout = String::from_utf8_lossy(&output.stdout);

    Ok(stdout
        .lines()
        .find_map(|line| line.trim().strip_prefix("Paired:"))
        .map(|value| value.trim() == "yes")
        .unwrap_or(false))
}

// Marks a device as trusted so future connections don't
// require manual confirmation.
//
// In: mac_address - MAC address of the target device.
// Out: Ok(()) on success, Err(String) if bluetoothctl
// failed.
fn on_trust_device(mac_address: &str) -> Result<(), String> {
    Command::new("bluetoothctl")
        .args(["trust", mac_address])
        .output()
        .map(|_| ())
        .map_err(|e| format!("failed to trust device: {e}"))
}

// Sets the list of saved (paired) devices in the UI.
//
// In: ui - the app window to update.
// In: devices - model of paired devices to display.
fn display_saved_devices(ui: &AppWindow, devices: VecModel<BluetoothDevice>) {
    ui.set_saved_devices(ModelRc::new(devices));
}
