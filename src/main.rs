// Prevent console window in addition to Slint window in Windows release builds when, e.g., starting the app via file manager. Ignored on other platforms.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::collections::HashMap;
use std::{error::Error};
use std::process::Command;
use slint::{SharedString, ModelRc, VecModel};

slint::include_modules!();

#[derive(Debug)]

struct AccessPoint{
    ssid: String,
    signal: u8,
    in_use: bool
}

fn main() -> Result<(), Box<dyn Error>> {
    let ui = AppWindow::new()?;

    get_saved_networks(&ui);
    get_avail_networks(&ui);

    
    ui.run()?;

    Ok(())
}


fn get_avail_networks(ui: &AppWindow){
    let networks = Command::new("nmcli")
                                    .args(["-t", "-f", "IN-USE,SSID,SIGNAL", "dev", "wifi", "list"])
                                    .output()
                                    .expect("failed to run nmcli");
    
    let networks = String::from_utf8_lossy(&networks.stdout);

    let mut wifi_networks: Vec<&str> = networks
        .lines()
        .collect();
        wifi_networks.sort();
        wifi_networks.dedup();

    let mut access_points: Vec<AccessPoint> = vec![];
    for net in wifi_networks{
        let split: Vec<&str> = net.split(':').collect();
        if split[1].to_string() == "" || split[2].parse::<u8>().unwrap() < 35{
            continue;
        }
        access_points.push(AccessPoint { ssid: (split[1].to_string()), signal: (split[2].parse::<u8>().unwrap()), in_use: (split[0] == "*") });
    }

    let mut filtered_access_points: HashMap<String, AccessPoint> = HashMap::new();

    for point in access_points{
        if !filtered_access_points.contains_key(&point.ssid){
            filtered_access_points.insert(point.ssid.clone(), point);
        }
        else if point.in_use {
            filtered_access_points.remove(&point.ssid);
            filtered_access_points.insert(point.ssid.clone(), point);
        }
        else if filtered_access_points.get(&point.ssid).unwrap().in_use {
            continue;
        }
        else if filtered_access_points.get(&point.ssid).unwrap().signal < point.signal {
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

    let out_networks:Vec<SharedString> = sorted_vec
                                        .iter()
                                        .map(|ap| SharedString::from(ap.ssid.clone()))
                                        .collect();

    let out_networks = VecModel::from(out_networks);

    ui.set_avail_networks(ModelRc::new(out_networks));    

    
}

fn get_saved_networks(ui: &AppWindow){
   
    let networks = Command::new("nmcli")
                                    .args(["-t", "-f", "NAME,TYPE", "connection", "show"])        
                                    .output()
                                    .expect("failed to run nmcli");

    let stdout = String::from_utf8_lossy(&networks.stdout);

    let wifi_networks: Vec<&str> = stdout
        .lines()
        .filter_map(|line| {
            let (name, ty) = line.rsplit_once(':')?;
            (ty == "802-11-wireless").then_some(name)
        })
        .collect();

    let out_networks = VecModel::from(
        wifi_networks
        .into_iter()
        .map(SharedString::from)
        .collect::<Vec<_>>(),
    );

    ui.set_networks(ModelRc::new(out_networks));

}