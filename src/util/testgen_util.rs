use std::fs::File;

use futures_util::io::BufReader;
use serde_json::Deserializer;

pub async fn exec_test(file_path: String) {
    let file = File::open(file_path);
    let reader = BufReader::new(file);

    testcase::<String> = serde_json::from_reader(reader);

    println!("{testcase}");
}