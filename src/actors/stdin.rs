use super::monitor::MonitorHandle;
use super::sender_can::SenderCANHandle;
use super::receiver_can::ReceiverCANHandle;
use super::test_gen::TestGenHandle;


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
}

#[derive(Parser)]
struct Send {
    #[clap(short)]
    id: String,
    #[clap(short)]
    message: String,
    #[clap(short)]
    cycletime: String,
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
            SubCommand::Send(t) => {
                println!("id: {} message: {} cycletime: {}", t.id, t.message, t.cycletime);
                let id = match u32::from_str_radix(t.id.as_str(),16) {
                    Ok(number) => number,
                    Err(e) => {
                        panic!("Issue parsing number {} || Got error {}",  t.id,e);
                    }
                };

                let message = match u64::from_str_radix(t.message.as_str(),16) {
                    Ok(number) => number,
                    Err(e) => {
                        panic!("Issue parsing message {} || Got error {}",  t.message,e);
                    }
                };

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
            SubCommand::Receive(t) => {
                self.receivercan_handle.receive_can_msg(t.id, t.nr_of_messages).await;
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
