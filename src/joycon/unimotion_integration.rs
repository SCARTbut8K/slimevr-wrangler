use super::communication::ChannelData;
use super::imu::UniSensorAxisData;
use super::{Battery, ChannelInfo, JoyconDesign, JoyconDesignType};
use crate::settings;
use unimotion_rs::prelude::*;
use unimotion_rs::unimotion::device::UniSensorDevice;
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::Duration;

// Gyro: 2000dps
// Accel: 8G
// https://github.com/dekuNukem/Nintendo_Switch_Reverse_Engineering/blob/master/imu_sensor_notes.md

// Convert to acceleration in G
fn acc(n: i16, offset: i16) -> f64 {
    let n = n.saturating_sub(offset);
    n as f64 * 0.00024414435f64 // 16000/65535/1000
}
// Convert to acceleration in radians/s
fn gyro(n: i16, offset: i16, scale: f64) -> f64 {
    n.saturating_sub(offset) as f64
    * scale
    // NOTE: 13371 is technically a value present in flash, in practice it seems to be constant.
    //* (936.0 / (13371 - offset) as f64) // to degrees/s
    * 0.07000839246f64 // 4588/65535 - degrees/s
    .to_radians() // radians/s
}

fn unimotion_thread(
    m: Arc<Mutex<UnimotionManager>>,
    tx: mpsc::Sender<ChannelData>,
    settings: settings::Handler,
) {
    let mut manager = match m.lock() {
        Ok(m) => m,
        Err(m) => m.into_inner(),
    };

    let sensors = manager.sensors();
    for sensor in sensors {
        let design = JoyconDesign {
            color: format!(
                "#{:02x}{:02x}{:02x}",
                30, 220, 0
            ),
            design_type: JoyconDesignType::Left,
        };

        tx.send(ChannelData {
            serial_number: sensor.mac_addr.to_string(),
            info: ChannelInfo::Connected(design),
        })
        .unwrap();

        // TODO: Battery voltage is sent with every UniSensor update, convert to the appropriate Battery
        tx.send(ChannelData::new(
            sensor.mac_addr.to_string(),
            ChannelInfo::Battery(Battery::Full),
        ))
        .unwrap();
    }

    loop {
        let (s, d) = manager.update();

        // let gyro_scale_factor = settings.load().joycon_scale_get(&serial_number);
        // TODO: Send 1, 2 or 4 data points as provided by the UniSensor
        let imu_data = UniSensorAxisData {
            gyro_x: gyro(d.quaternions[0].unwrap().x, 0, 1.0),
            gyro_y: gyro(d.quaternions[0].unwrap().y, 0, 1.0),
            gyro_z: gyro(d.quaternions[0].unwrap().z, 0, 1.0),
            gyro_w: gyro(d.quaternions[0].unwrap().w, 0, 1.0),
        }; 
        tx.send(ChannelData::new(
            s.mac_addr.to_string(),
            ChannelInfo::UniSensorImuData(imu_data),
        ))
        .unwrap();
    }
}

pub fn spawn_thread(tx: mpsc::Sender<ChannelData>, settings: settings::Handler) {
    let manager = UnimotionManager::get_instance();
    let tx = tx.clone();
    let settings = settings.clone();
    thread::spawn(move || unimotion_thread(manager, tx, settings));
}
