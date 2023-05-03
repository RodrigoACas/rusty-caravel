use tokio::sync::mpsc;
use crate::util::testgen_util::{self, exec_test};

enum TestGenMessages{
    FilePath {
        file_path: String,
    },
}
pub struct TestGen{
    receiver: mpsc::Sender<TestGenMessages>,
}

impl TestGen {
    pub fn new(receiver: mpsc::Receiver<TestGenMessages>) -> Self{
        Self {receiver}
    }

    async fn handle_message(msg: TestGenMessages) {
        match msg {
            TestGenMessages::FilePath {
                file_path
            } => {
                exec_test(file_path);
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