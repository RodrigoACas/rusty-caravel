//! SocketCAN wrapper for Windows builds
//!
//! SocketCan is not available in windows.
//! Libs using SocketCan do not build on windows, this provides a way to still 
//! build this project on windows.

use log::{info};
use std::{fmt, convert::TryInto};
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

pub async fn send_can_frame(socket: &CANSocket, frame: CANFrame) {
    info!("[MOCK] [{}] Write {}", socket, frame);
}

impl CANSocket {
    pub fn open(ifname: &str) -> Result<Self> {
        Ok(CANSocket{ifname: String::from_str(ifname)?})
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

///////////////////////////////////////////////// ISO-TP WRAPPER /////////////////////////////////////////
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum Id {
    Standard(StandardId),
    Extended(ExtendedId),
}
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct StandardId(u16);
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct ExtendedId(u32);

pub struct FlowControlOptions;
impl FlowControlOptions {
    pub fn new(bs: u8, stmin: u8, wftmax: u8) -> Self {
        Self
    }
}
impl StandardId {
    pub fn new(raw :u16) -> Option<Self> {
        return Some(Self(raw));
    }
    pub fn new_unchecked(raw :u16) -> Self {
        return Self(raw);
    }
}
impl ExtendedId {
    pub fn new(raw :u32) -> Option<Self> {
        return Some(Self(raw));
    }
    pub fn new_unchecked(raw :u32) ->Self {
        return Self(raw);
    }
}
impl From<StandardId> for Id {
    #[inline]
    fn from(id: StandardId) -> Self {
        Id::Standard(id)
    }
}

impl From<ExtendedId> for Id {
    #[inline]
    fn from(id: ExtendedId) -> Self {
        Id::Extended(id)
    }
}
pub struct IsoTpSocket {
    ifname: String,
    src: Id,
    dest: Id,
    isotp_options: Option<String>, 
    rx_flow_control_options: Option<FlowControlOptions>,
    link_layer_options: Option<String>,
}
impl IsoTpSocket {
    pub fn open(ifname: &str, src:impl Into<Id>, dest: impl Into<Id>) -> Result<Self> {
        Ok(Self { 
            ifname:ifname.to_owned(), 
            src: src.into(), 
            dest: dest.into(),
            isotp_options: None,
            rx_flow_control_options: None,
            link_layer_options: None,
        })
    }
    pub fn open_with_opts(ifname: &str, 
                            src:impl Into<Id>, 
                            dest: impl Into<Id>, 
                            isotp_options: Option<String>, 
                            rx_flow_control_options: Option<FlowControlOptions>,
                            link_layer_options: Option<String>,
        ) -> Result<Self> {

        Ok(Self { 
            ifname:ifname.to_owned(), 
            src: src.into(), 
            dest: dest.into(),
            isotp_options: None,
            rx_flow_control_options: None,
            link_layer_options: None,
        })
    }
}
pub async fn send_isotp_frame(socket: &IsoTpSocket, data: &[u8]){
    info!("[MOCK] [{}] Write {:?}", socket.ifname, data);
}

pub async fn receive_isotp_frame(socket: &mut IsoTpSocket) -> Result<Vec<u8>> {
    Ok(vec![1,2,3])
}