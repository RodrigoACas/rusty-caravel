#![allow(unused_imports)]

use log::{debug, error, info, Level};
use anyhow::{Result, anyhow};
use futures_util::stream::StreamExt;
pub use tokio_socketcan::{CANFrame, CANSocket};
pub use socketcan_isotp::{Id, IsoTpSocket, StandardId};

pub async fn send_can_frame(socket: &CANSocket, frame: CANFrame) {
    match socket.write_frame(frame).expect("Writing is busted").await {
        Ok(_) => {
            debug!("Wrote {:?} # {:?}", socket, frame);
        }
        Err(e) => {
            debug!("Failed writing {:?} # {:?} Error: {}", socket, frame, e);
        }
    }

}

pub async fn send_isotp_frame(socket: IsoTpSocket, data: &[u8]) -> Result<()>{
    match socket.write(data) {
        Ok(_) => {
            debug!("Wrote {:?}", data);
        }
        Err(e) => {
            debug!("Failed writing # {:?} Error: {}", data, e);
        }
    }

    Ok(())
}

pub async fn receive_isotp_frame(mut socket:IsoTpSocket) -> Result<Vec<u8>> {
    match socket.read() {
        Ok(val) => return Ok(val.to_vec()),
        Err(e) => return Err(e.into()),
    }
}
// pub async fn receive_can_frame(socket: &CANSocket) -> Result<CANFrame>{
    
// }
