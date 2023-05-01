use tokio::sync::mpsc;

enum TestGenMessages{
    FilePath {
        file_path: String,
    },
}
struct TestGen{

}

struct TestGenHandle{
    sender: mpsc::Sender<TestGenMessages>,
}

impl TestGenHandle{

}