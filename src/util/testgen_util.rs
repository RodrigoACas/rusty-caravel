use std::fs::File;

use std::io::BufReader;
use serde_json::Deserializer;

pub async fn exec_test(file_path: String) {
    let file = File::open(file_path).expect("Couldn't open test file");
    let reader = BufReader::new(file);

    // let mut testcase_de =Deserializer::from_reader(reader);
    
    // println!("{testcase}");
}