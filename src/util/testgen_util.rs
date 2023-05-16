use std::fs::File;

use std::io::{BufReader, Read};
use anyhow::Result;
use serde_json::{self, Value};

pub async fn exec_test(file_path: String) -> Result<()>{
    let mut file = File::open(file_path).expect("Couldn't open test file");
    let mut contents = String::new();
    file.read_to_string(&mut contents);

    // let testcase = serde_json::from_reader::<BufReader<File>,String>(reader).expect("Panicked trying to deserialize JSON");
    let json: Value = serde_json::from_str(&contents)?;
    process_json(json)?;


    Ok(())
}

fn process_json(json: Value) -> Result<()>{
    match json {
        Value::Null => {
            // Handle null value
        }
        Value::Bool(value) => {
            // Handle boolean value
        }
        Value::Number(value) => {
            // Handle number value
        }
        Value::String(value) => {
            // Handle string value
        }
        Value::Array(values) => {
            // Handle array value
            for value in values {
                process_json(value)?;
            }
        }
        Value::Object(obj) => {
            // Handle object value
            for (key, value) in obj {
                process_json(value)?;
            }
        }
    }
    
    Ok(())
}