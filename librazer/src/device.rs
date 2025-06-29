use crate::descriptor::{Descriptor, SUPPORTED};
use crate::packet::Packet;

use anyhow::{anyhow, Context, Result};
use log::{debug};
use std::{thread, time};
use std::fs;

pub struct Device {
    device: hidapi::HidDevice,
    pub info: Descriptor,
}

// Read the model id and clip to conform with https://mysupport.razer.com/app/answers/detail/a_id/5481
#[cfg(target_os = "windows")]
fn read_device_model() -> Result<String> {
    let hklm = winreg::RegKey::predef(winreg::enums::HKEY_LOCAL_MACHINE);
    let bios = hklm.open_subkey("HARDWARE\\DESCRIPTION\\System\\BIOS")?;
    let system_sku: String = bios.get_value("SystemSKU")?;
    Ok(system_sku.chars().take(10).collect())
}

#[cfg(target_os = "linux")]
fn read_device_model() -> Result<String> {
    let path = "/sys/devices/virtual/dmi/id/product_sku";
    
    match fs::read_to_string(path) {
        Ok(sku) => {
            let sku = sku.trim().to_string();
            
            if sku.starts_with("RZ") {
                Ok(sku)
            } else {
                debug!("Invalid Razer SKU prefix: {}", sku);
                Err(anyhow!("Invalid Razer SKU: {}", sku))
            }
        }
        Err(e) => {
            debug!("Failed to read {}: {}", path, e);
            Err(anyhow!("DMI read error: {}", e))
        }
    }
}

#[cfg(not(any(target_os = "windows", target_os = "linux")))]
fn read_device_model() -> Result<String> {
    debug!("Unsupported platform detected");
    anyhow::bail!("Automatic model detection is not implemented for this platform")
}

impl Device {
    const RAZER_VID: u16 = 0x1532;

    pub fn info(&self) -> &Descriptor {
        &self.info
    }

    pub fn new(descriptor: Descriptor) -> Result<Device> {
        let api = hidapi::HidApi::new().context("Failed to create hid api")?;

        for info in api.device_list().filter(|info| {
            (info.vendor_id(), info.product_id()) == (Device::RAZER_VID, descriptor.pid)
        }) {
            let path = info.path();
            let device = api.open_path(path)?;
            if device.send_feature_report(&[0, 0]).is_ok() {
                return Ok(Device {
                    device,
                    info: descriptor.clone(),
                });
            }
        }
        anyhow::bail!("Failed to open device {:?}", descriptor)
    }

    pub fn send(&self, report: Packet) -> Result<Packet> {
        // extra byte for report id
        let mut response_buf: Vec<u8> = vec![0x00; 1 + std::mem::size_of::<Packet>()];

        thread::sleep(time::Duration::from_micros(1000));
        self.device
            .send_feature_report(
                [0_u8; 1] // report id
                    .iter()
                    .copied()
                    .chain(Into::<Vec<u8>>::into(&report).into_iter())
                    .collect::<Vec<_>>()
                    .as_slice(),
            )
            .context("Failed to send feature report")?;

        thread::sleep(time::Duration::from_micros(2000));
        if response_buf.len() != self.device.get_feature_report(&mut response_buf)? {
            return Err(anyhow!("Response size != {}", response_buf.len()));
        }

        // skip report id byte
        let response = <&[u8] as TryInto<Packet>>::try_into(&response_buf[1..])?;
        response.ensure_matches_report(&report)
    }

    pub fn enumerate() -> Result<(Vec<u16>, String)> {
        let api = match hidapi::HidApi::new() {
            Ok(api) => api,
            Err(e) => {
                debug!("Failed to create HID API: {}", e);
                return Err(anyhow!("HID API error: {}", e));
            }
        };

        let devices = api.device_list().collect::<Vec<_>>();
        
        let razer_devices: Vec<_> = devices
            .iter()
            .filter(|info| info.vendor_id() == Device::RAZER_VID)
            .collect();
        
        if razer_devices.is_empty() {
            debug!("No Razer devices found");
            return Err(anyhow!("No Razer devices found"));
        }

        // Extract unique PIDs
        let pids: Vec<u16> = razer_devices.iter()
            .map(|info| info.product_id())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();

        // Get device model
        let model = match read_device_model() {
            Ok(m) => m,
            Err(e) => {
                debug!("Failed to detect model: {}", e);
                return Err(anyhow!("Failed to detect model: {}", e));
            }
        };
        
        if !model.starts_with("RZ09-") {
            debug!("Detected model is not a Razer laptop: {}", model);
            return Err(anyhow!("Detected model is not a Razer laptop: {}", model));
        }

        Ok((pids, model))
    }
    pub fn detect() -> Result<Device> {
        let (pid_list, model_number_prefix) = Device::enumerate()?;

        // Find matching descriptor
        let supported = SUPPORTED.iter().find(|d| 
            model_number_prefix.starts_with(d.model_number_prefix)
        );

        match supported {
            Some(desc) => {
                Device::new(desc.clone())
            }
            None => {
                let pids_fmt = pid_list.iter()
                    .map(|pid| format!("{:#06x}", pid))
                    .collect::<Vec<_>>()
                    .join(", ");
                
                Err(anyhow!(
                    "Model {} with PIDs [{}] is not supported",
                    model_number_prefix,
                    pids_fmt
                ))
            }
        }
    }
}
