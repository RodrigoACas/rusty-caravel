//! SocketCAN wrapper for Windows builds
//!
//! SocketCan is not available in windows.
//! Libs using SocketCan do not build on windows, this provides a way to still 
//! build this project on windows.

use log::{info};
use std::fmt;
use anyhow::Result;

use std::str::FromStr;

/// CANFrame dummy struct
#[derive(Debug)]
pub struct CANFrame {
    id: u32,
    data_len: u8,
    data: [u8; 8],
}

impl CANFrame {
    pub fn new(id: u32, data: &[u8], _rtr: bool, _err: bool) -> Result<Self> {

        let mut tmp = [0; 8];
        tmp[..data.len()].clone_from_slice(data);

        Ok(CANFrame{
            id,
            data_len: data.len() as u8,
            data: tmp 
        })
    }

    pub fn get_data(&self) -> [u8; 8] {
        self.data
    }
}

impl fmt::Display for CANFrame {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}#{:x?} {}", self.id, self.data, self.data_len)
    }
}

/// CANSocket dummy struct
#[derive(Debug)]
pub struct CANSocket {
    ifname: String
}

impl CANSocket {
    pub fn open(ifname: &str) -> Result<Self> {
        Ok(CANSocket{ifname: String::from_str(ifname)?})
    }
    pub async fn send_can_frame(&self, frame: CANFrame) {
        info!("[MOCK] [{}] Write {}", self, frame);
    }
    
    pub async fn receive_can_frame(&self) -> Result<CANFrame>{
        Ok(CANFrame { 
            id: 0, 
            data_len: 1, 
            data: [0,1,2,3,4,5,6,7] 
        })
    }
}

impl fmt::Display for CANSocket {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.ifname)
    }
}

