use log::info;
use tokio::sync::mpsc;
use crate::util::testgen_util;

enum TestGenMessages{
    FilePath {
        file_path: String,
    },
}
struct TestGen{
    receiver: mpsc::Receiver<TestGenMessages>,
}

impl TestGen {
    pub fn new(receiver: mpsc::Receiver<TestGenMessages>) -> Self{
        Self {receiver}
    }

    async fn handle_message(&self, msg: TestGenMessages) {
        match msg {
            TestGenMessages::FilePath {
                file_path
            } => {
                testgen_util::exec_test(file_path).await;
            }
        }
    }
}

async fn run(mut actor: TestGen) {
    info!("Running TestGen");

    while let Some(msg) = actor.receiver.recv().await {
        actor.handle_message(msg).await;
    }
}

pub struct TestGenHandle{
    sender: mpsc::Sender<TestGenMessages>,
}

impl TestGenHandle{
    pub fn new() -> Self {
        let(sender, receiver) = mpsc::channel(8);

        let actor = TestGen::new(receiver);

        tokio::spawn(run(actor));

        Self{sender}
    }

    pub async fn send_test(&self, file_path: String) {
        let msg = TestGenMessages::FilePath { file_path };

        let _ = self.sender.send(msg).await;
    }
}