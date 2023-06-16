use super::monitor::MonitorHandle;
use super::sender_can::SenderCANHandle;
use super::receiver_can::ReceiverCANHandle;
use super::test_gen::TestGenHandle;
use crate::util::canutil::{ExtendedId, Id, StandardId};

use shell_words;
use tokio::sync::mpsc;

use clap::{AppSettings, Parser};

use log::info;

#[derive(Parser)]
#[clap(version = "0.2.0", author = "marujos", setting=AppSettings::NoBinaryName)]
pub struct Opts {
    /// Sets a custom config file. Could have been an Option<T> with no default too
    #[clap(short, long)]
    config: Option<String>,
    #[clap(subcommand)]
    subcmd: SubCommand,
}

#[derive(Parser)]
enum SubCommand {
    Send(Send),
    Receive(Receive),
    Exit(Exit),
    GenTest(Test),
    SendIsotp(SendIsotp),
}

#[derive(Parser)]
struct Send {
    #[clap(short)]
    id: String,
    #[clap(short, multiple_values(true), max_values(8), min_values(1))]
    message: Vec<String>,
    #[clap(short)]
    cycletime: String,
}

#[derive(Parser)]
struct SendIsotp {
    /// Id mode: 0/false for standard, 1/true for extended
    #[clap(short)]
    id_mode: String,
    #[clap(short)]
    src_id: String,
    #[clap(short)]
    dest_id: String,
    #[clap(short, multiple_values(true), min_values(1))]
    message: Vec<String>,
}

#[derive(Parser)]
struct Exit {}

#[derive(Parser)]
struct Receive {
    id: Option<String>,
    nr_of_messages: Option<String>,
}

#[derive(Parser)]
struct Test {
    file_path: String,
}

#[derive(Debug)]
enum Messages {
    Line { line: String },
    Shutdown,
}

struct StdInLines {
    inbox: mpsc::Receiver<Messages>,
    monitor: MonitorHandle,
    sendercan_handle: SenderCANHandle,
    receivercan_handle: ReceiverCANHandle,
    testgen_handle: TestGenHandle,
}

impl StdInLines {
    fn new(inbox: mpsc::Receiver<Messages>, monitor: MonitorHandle) -> Self {
        let sendercan_handle = SenderCANHandle::new();
        let receivercan_handle = ReceiverCANHandle::new();
        let testgen_handle = TestGenHandle::new();
        StdInLines { 
            inbox, 
            monitor,
            sendercan_handle,
            receivercan_handle,
            testgen_handle, 
        }
    }

    async fn tell_monitor(&self) {
        self.monitor.exit_received().await.unwrap();
    }

    async fn handle_message(&mut self, msg: Messages) -> bool {
        match msg {
            Messages::Shutdown => false,
            Messages::Line { line } => self.handle_command(line).await,
        }
    }

    async fn handle_command(&mut self, msg: String) -> bool {
        let words = shell_words::split(&msg).expect("cmd split went bust");
        
        let cmd: Opts = match Opts::try_parse_from(words) {
            Ok(opts) => opts,
            Err(error) => {
                println!("{}", error);
                return true;
            }
        };

        match cmd.subcmd {
            SubCommand::SendIsotp(t) => {
                let src:Id;
                let dest:Id;
                //Conversion from String to ID enum
                if t.id_mode=="true" || t.id_mode=="1" {

                    let src_struc_opt = ExtendedId::new(u32::from_str_radix(t.src_id.as_str(),16).unwrap());
                    if let Some(src_struc) = src_struc_opt {
                        src= Id::Extended(src_struc);
                    } else {panic!("Panicked creating id from {}", t.src_id)}
                    
                    let dest_struc_opt = ExtendedId::new(u32::from_str_radix(t.dest_id.as_str(),16).unwrap());
                    if let Some(dest_struc) = dest_struc_opt {
                        dest = Id::Extended(dest_struc);
                    } else {panic!("Panicked creating id from {}", t.dest_id)}
                } 
                else if t.id_mode=="false" || t.id_mode=="0" {
                    let src_struc_opt = StandardId::new(u16::from_str_radix(t.src_id.as_str(),16).unwrap());
                    if let Some(src_struc) = src_struc_opt {
                        src= Id::Standard(src_struc);
                    } else {panic!("Panicked creating id from {}", t.src_id)}
                    
                    let dest_struc_opt = StandardId::new(u16::from_str_radix(t.dest_id.as_str(),16).unwrap());
                    if let Some(dest_struc) = dest_struc_opt {
                        dest = Id::Standard(dest_struc);
                    } else {panic!("Panicked creating id from {}", t.dest_id)}
                }
                else {
                    info!("Unknown parameter {}", t.id_mode);
                    return true;
                }

                let mut message: Vec<u8> = Vec::new();
                for value in t.message {
                    match u8::from_str_radix(value.as_str(),16) {
                        Ok(number) => message.push(number),
                        Err(e) => {
                            panic!("Issue parsing message {} || Got error {}",  value,e);
                        }
                    };
                }

                self.sendercan_handle.send_isotp_message(src, dest, message).await;
                true
            }
            SubCommand::Send(t) => {
                println!("id: {} message: {:?} cycletime: {}", t.id, t.message, t.cycletime);
                let id = match u32::from_str_radix(t.id.as_str(),16) {
                    Ok(number) => number,
                    Err(e) => {
                        panic!("Issue parsing number {} || Got error {}",  t.id,e);
                    }
                };

                let mut message: Vec<u8> = Vec::new();
                for value in t.message {
                    match u8::from_str_radix(value.as_str(),16) {
                        Ok(number) => message.push(number),
                        Err(e) => {
                            panic!("Issue parsing message {} || Got error {}",  value,e);
                        }
                    };
                }

                let cycle_time: u64 = t.cycletime.parse().expect("TODO handle errors");

                self.sendercan_handle.send_can_message(id, message, cycle_time).await;
                //if cycletime == 0 {
                //    self.sender.send_can_message(id, message, cycletime).await;
                //    true
                //} else {
                //    tokio::spawn(cyclic_sender(self.sender.clone(), id, message, cycletime));
                //    true
                //}
                true
            }
            SubCommand::Receive(_t) => {
                println!("Receive function is not implemented");
                //self.receivercan_handle.receive_can_msg(t.id, t.nr_of_messages).await;
                true
            }
            SubCommand::GenTest(t) => {
                self.testgen_handle.send_test(t.file_path).await;
                true
            }
            SubCommand::Exit(_t) => {
                self.tell_monitor().await;
                false
            }
           
        }
    }
}

//async fn cyclic_sender(sender: SenderCANHandle, id: u32, message: u64, cycletime: u64) {
//    loop {
//        sleep(Duration::from_millis(cycletime)).await;
//        sender.send_can_message(id, message, cycletime).await
//    }
//}

async fn run(mut actor: StdInLines) {
    info!("Running");

    while let Some(msg) = actor.inbox.recv().await {
        if !actor.handle_message(msg).await {
            break;
        }
    }

    info!("Shutting Down");
}

fn reading_stdin_lines(sender: mpsc::Sender<Messages>) {
    let runtime = tokio::runtime::Handle::current();
    std::thread::spawn(move || {
        let sender = sender.clone();
        let stdin = std::io::stdin();
        let mut line_buf = String::new();
        while let Ok(_) = stdin.read_line(&mut line_buf) {
            let sender = sender.clone();
            let line = line_buf.trim_end().to_string();
            line_buf.clear();

            runtime.spawn(async move {
                let message = Messages::Line { line };
                let result = sender.send(message).await;
                if let Err(error) = result {
                    println!("start_reading_stdin_lines send error: {:?}", error);
                }
            });
        }
    });
}

pub struct StdInLinesHandle {
    sender: mpsc::Sender<Messages>,
}

impl StdInLinesHandle {
    pub fn new(
        // runtime: tokio::runtime::Handle,
        //watch_receiver: CtrlCActorHandle,
        //sender: SenderCANHandle,
        //receiver: ReceiverCANHandle
        monitor: MonitorHandle,
    ) -> StdInLinesHandle {
        let (sender, inbox) = tokio::sync::mpsc::channel(5);

        reading_stdin_lines(sender.clone());

        let actor = StdInLines::new(inbox, monitor);
        

        tokio::spawn(run(actor));
        
        Self { 
            sender,
        }
    }

    pub async fn shutdown(&self) {
        let msg = Messages::Shutdown;

        self.sender.try_send(msg).expect("What ?");
    }
}
