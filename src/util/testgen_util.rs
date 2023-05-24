use std::fs::{File, self};
use hex;
use std::io::{Read};
use std::path::{Path};
use anyhow::Result;
use log::{info};
use serde_json::{self, Value};
use async_recursion::async_recursion;

use super::canutil::{CANFrame, CANSocket};

pub async fn exec_test(file_path: String) -> Result<()>{
    let mut file = File::open(file_path).expect("Couldn't open test file");
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;

    let json: Value = serde_json::from_str(&contents)?;
    process_json(json, None).await?;


    Ok(())
}

#[async_recursion]
async fn process_json(json: Value, key_op:Option<String>) -> Result<Option<String>>{
    match json {
        Value::String(value) => {
            if let Some(key) = key_op {
                match key.as_str() {
                    "TestSuitName" => {
                        info!("Initiating tests to {value}");
                    }
                    "TestName" => {
                        info!("Initiating test {value}");
                    }
                    _ => {}
                }
            }
        }
        Value::Array(values) => {
            if let Some(key) = key_op{
                match key.as_str(){
                    "Tests" => {
                        for value in values{
                            process_json(value, None).await?;
                        }
                    }
                    "Sequence" => {
                        for value in values {
                            process_json(value, None).await?;
                        }
                    }
                    "PairArray" => {
                        let mut divide: bool = false;
                        let mut request_vec: Vec<String>=Vec::new();
                        let mut response_vec: Vec<String>=Vec::new();

                        for value in values {
                            if let Value::String(string) = value {
                                if string == "Res" {
                                    divide=true;
                                    continue;
                                }

                                if divide {
                                    response_vec.push(string);
                                }
                                else{
                                    request_vec.push(string);
                                }
                            }
                        }
                        process_request(request_vec).await?;
                        process_response(response_vec).await?;                        
                    }
                    _ => {}
                }
            }
            
        }
        Value::Object(obj) => {
            // Handle object value
            for (key, value) in obj {
                process_json(value, Some(key)).await?;
            }
        }
        _ => {}
    }
    
    Ok(None)
}

async fn process_request(request_vec: Vec<String>) -> Result<()>{
    let mut can_frame_vec: Vec<u8>= Vec::new();

    for value in request_vec {
        if value.starts_with("0x"){
            can_frame_vec.push(u8::from_str_radix(&value[2..], 16).unwrap());
        }

        if value.ends_with(".der"){
            if !Path::new(&value).is_file() {
                
            }
        
            let cert_string = fs::read_to_string(value).unwrap();
            let mut cert_vec: Vec<u8> = hex::decode(cert_string).unwrap();
            can_frame_vec.append(&mut cert_vec);
        }

    }

    let frame = CANFrame::new(0, can_frame_vec.as_slice(), false, false).expect("Couldn't create CAN Frame");
    let socket = CANSocket::open("can0").expect("Couldn't open CAN socket");
    socket.send_can_frame(frame).await;

    Ok(())
}

async fn process_response(response_vec: Vec<String>) -> Result<()>{
    let mut can_frame_vec: Vec<u8>= Vec::new();
    
    let socket = CANSocket::open("can0").expect("Couldn't open CAN socket");
   
    //Receives CAN Frame
    let frame = socket.receive_can_frame().await.unwrap();
    
    //After theoretical appending of all CAN frames (max data payolad of 8 bytes so entire message will be divided into multiple frames)
    let message= frame;

    for value in response_vec {
        if value.starts_with("0x"){
            can_frame_vec.push(u8::from_str_radix(&value[2..], 16).unwrap());
        }

        // if value.ends_with(".der"){
        //     if !Path::new(&value).is_file() {
                
        //     }
        
        //     let cert_string = fs::read_to_string(value).unwrap();
        //     let mut cert_vec: Vec<u8> = hex::decode(cert_string).unwrap();
        //     can_frame_vec.append(&mut cert_vec);
        // }

    }

    Ok(())
}