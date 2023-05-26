use std::collections::HashMap;
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
async fn process_json(json: Value, key_op:Option<String>) -> Result<()>{
    match json {
        Value::String(value) => {
            if let Some(key) = key_op {
                match key.as_str() {
                    "TestSuitName" => {
                        info!("Initiating tests to {value}");
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
                        process_sequence(values).await.expect("Failed at processing sequence");                        
                                                
                    }
                    // "PairArray" => {
                    //     let mut divide: bool = false;
                    //     let mut request_vec: Vec<String>=Vec::new();
                    //     let mut response_vec: Vec<String>=Vec::new();

                    //     for value in values {
                    //         if let Value::String(string) = value {
                    //             if string == "Res" {
                    //                 divide=true;
                    //                 continue;
                    //             }

                    //             if divide {
                    //                 response_vec.push(string);
                    //             }
                    //             else{
                    //                 request_vec.push(string);
                    //             }
                    //         }
                    //     }
                        
                    //     process_request(request_vec, Some(&mut map)).await?;
                    //     process_response(response_vec, Some(&mut map)).await?;    
                        
                                            
                    // }
                    _ => {}
                }
            }
            
        }
        Value::Object(obj) => {
            for (key, value) in obj {
                process_json(value, Some(key)).await?;
            } 
        }
        _ => {}
    }
    
    Ok(())
}

async fn process_sequence(objects: Vec<Value>) -> Result<()> {
    let mut vars: HashMap<String, Vec<u8>> = HashMap::new();
    
    for object in objects {
        if let Value::Object(obj) = object {
            for (_key, value) in obj {
                if let Value::String(string) = value {
                    info!("Initiating test {string}");       
                }
                else if let Value::Array(elems) = value {
                    let mut divide: bool = false;
                    let mut request_vec: Vec<String>=Vec::new();
                    let mut response_vec: Vec<String>=Vec::new();

                    for elem in elems {
                        if let Value::String(string) = elem {
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
                    
                    process_request(request_vec, &mut vars).await?;
                    process_response(response_vec, &mut vars).await?;    
                    
                }
            } 
        }
    }

    Ok(())
}

async fn process_request(request_vec: Vec<String>, variables:&mut HashMap<String, Vec<u8>>) -> Result<()>{
    let mut can_frame_vec: Vec<u8>= Vec::new();

    for value in request_vec {
        if value.starts_with("0x"){
            can_frame_vec.push(u8::from_str_radix(&value[2..], 16).unwrap());
        }
        else if value.ends_with(".der"){
            if !Path::new(&value).is_file() {
                
            }
        
            let cert_string = fs::read_to_string(value).unwrap();
            let cert_string= cert_string.replace("\r\n", "");
            let cert_string= cert_string.replace(" ", "");
            let mut cert_vec: Vec<u8> = hex::decode(cert_string).unwrap();
            can_frame_vec.append(&mut cert_vec);
        }
        else if value.starts_with("LEN(RES(") {
            let len_key = value.chars().count();
            let key = value[8..len_key-1].to_owned();
            let var = variables.get(&key).unwrap();

            // Missing solving CHALLENGE in var
            let mut sol=var.to_owned();

            let len_var = sol.len() as u16;
            can_frame_vec.push(((len_var & 65280)>>8) as u8);
            can_frame_vec.push((len_var & 255) as u8);

            can_frame_vec.append(&mut sol);
        }

    }

    let frame = CANFrame::new(0, can_frame_vec.as_slice(), false, false).expect("Couldn't create CAN Frame");
    let socket = CANSocket::open("can0").expect("Couldn't open CAN socket");
    socket.send_can_frame(frame).await;

    Ok(())
}

async fn process_response(response_vec: Vec<String>, variables:&mut HashMap<String, Vec<u8>>) -> Result<bool>{
    
    let socket = CANSocket::open("can0").expect("Couldn't open CAN socket");
   
    //Receives CAN Frame
    let frame = socket.receive_can_frame().await.unwrap();
    
    //After theoretical appending of all CAN frames (max data payolad of 8 bytes so entire message will be divided into multiple frames)
    let message= frame.get_data();
    
    
    let mut i=0;
    for value in response_vec {
        if value.starts_with("0x"){
            let hex_value = u8::from_str_radix(&value[2..], 16).unwrap();  

            if hex_value != message[i] {
                return Ok(false);
            }
        }

        if value.starts_with("LEN(") {
            let map_key = value[5..value.len()-1].to_owned();
            let var_len=(message[i] as u16)<<8 | (message[i+1] as u16);
            
            let var_vec = message[i+2..=i+2+var_len as usize].to_owned();
            i+=2+var_len as usize;

            variables.insert(map_key, var_vec);

            continue;
        }


        i+=1;
    }

    Ok(true)
}