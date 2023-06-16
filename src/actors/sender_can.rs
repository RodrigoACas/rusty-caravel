//! Sender CAN actor
//!
//! Actor responsible for sending can messages into a can socket

use tokio::sync::mpsc;

use crate::util::canutil::{CANFrame, CANSocket, send_can_frame, Id, IsoTpSocket, send_isotp_frame};

use log::info;

/// Messages that the Actor Can Receive
enum SenderCANMessages {
    SendToID {
        id: u32,
        message: Vec<u8>,
        _cycle_time: u64,
    },
    SendIsotp {
        src: Id,
        dest: Id,
        message: Vec<u8>,
    }
}  


struct SenderCAN {
    socket: CANSocket,
    receiver: mpsc::Receiver<SenderCANMessages>,
    _messages_sent: u32,
}

impl SenderCAN {
    fn new(receiver: mpsc::Receiver<SenderCANMessages>) -> Self {
        let socket = CANSocket::open("can0").expect("Panicked trying to open CAN socket");

        SenderCAN {
            socket,
            receiver,
            _messages_sent: 0,
        }
    }

    async fn handle_message(&mut self, msg: SenderCANMessages) {
        match msg {
            SenderCANMessages::SendToID { id, message, _cycle_time: _,} => {
                let frame = CANFrame::new(id, message.as_slice(), false, false).unwrap();
                send_can_frame(&self.socket, frame).await;

            }

            SenderCANMessages::SendIsotp { src, dest, message } => {
                let socket = IsoTpSocket::open("can0", src, dest).expect("Couldn't open ISO-TP socket");

                send_isotp_frame(socket, message.as_slice()).await;

            }
        }
    }
}

async fn run(mut actor: SenderCAN) {
    info!("Running");

    while let Some(msg) = actor.receiver.recv().await {
        actor.handle_message(msg).await;
    }
}

#[derive(Clone)]
pub struct SenderCANHandle {
    sender: mpsc::Sender<SenderCANMessages>,
}

impl SenderCANHandle {
    pub fn new() -> Self {
        let (sender, receiver) = mpsc::channel(8);
        let actor = SenderCAN::new(receiver);

        tokio::spawn(run(actor));

        Self { sender }
    }

    pub async fn send_can_message(&self, id: u32, message: Vec<u8>, _cycle_time: u64) {
        let msg = SenderCANMessages::SendToID {
            id,
            message,
            _cycle_time,
        };

        let _ = self.sender.send(msg).await;
    }

    pub async fn send_isotp_message(&self, src: Id, dest:Id, message: Vec<u8>) {
        let msg = SenderCANMessages::SendIsotp {
            src,
            dest,
            message,
        };

        let _ = self.sender.send(msg).await;
    }
}
