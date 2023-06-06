#![allow(unused_imports)]

use log::{debug, error, info, Level};
use anyhow::{Result, anyhow};
use futures_util::stream::StreamExt;
use std::process::Command;
pub use tokio_socketcan::{CANFrame, CANSocket};

pub async fn send_can_frame(socket: &CANSocket, frame: CANFrame) {
    Command::new("sudo")
            .arg("ifconfig")
            .arg("can0")
            .arg("up")
            .status();

    match socket.write_frame(frame).expect("Writing is busted").await {
        Ok(_) => {
            debug!("Wrote {:?} # {:?}", socket, frame);
        }
        Err(e) => {
            debug!("Failed writing {:?} # {:?} Error: {}", socket, frame, e);
        }
    }

}

// pub async fn receive_can_frame(socket: &CANSocket) -> Result<CANFrame>{
    
// }
