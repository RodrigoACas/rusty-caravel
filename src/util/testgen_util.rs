use std::collections::HashMap;
use std::fs::{File, self};
use hex;
use itertools::Itertools;
use std::io::{Read};
use std::path::{Path};
use anyhow::Result;
use log::{info, error};
use serde_json::{self, Value};
use async_recursion::async_recursion;

use super::canutil::{send_isotp_frame, IsoTpSocket, StandardId, receive_isotp_frame};

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
    info!("Starting to process sequence");

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
                            if string == "Response" {
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
                    let res = process_response(response_vec, &mut vars).await?;
                    if !res {
                        info!("Messages didn't match");
                    }
                    else {
                        info!("Messages matched");
                    }
                    
                    
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
        else if value.starts_with("FILE("){
            let value_len=value.chars().count();
            if !Path::new(&value[6..value_len]).is_file() {
                
            }
        
            let cert_string = fs::read_to_string(&value[6..value_len]).unwrap();
            let cert_string= cert_string.replace("\r\n", "");
            let cert_string= cert_string.replace(" ", "");
            let mut cert_vec: Vec<u8> = hex::decode(cert_string).unwrap();
            can_frame_vec.append(&mut cert_vec);
        }
        else if value.starts_with("LEN(RES(") {
            let len_key = value.chars().count();
            let key = value[8..len_key-2].to_owned();
            let var = variables.get(&key).unwrap();

            // Missing solving CHALLENGE in var
            let mut sol=var.to_owned();

            let len_var = sol.len() as u16;
            can_frame_vec.push(((len_var & 65280)>>8) as u8);
            can_frame_vec.push((len_var & 255) as u8);

            can_frame_vec.append(&mut sol);
        }

    }

    println!("Request frame: {:?}", can_frame_vec);

    let socket = IsoTpSocket::open(
        "can0",
         StandardId::new(0x123).unwrap(), 
         StandardId::new(0x321).unwrap(),
    ).unwrap();
    send_isotp_frame(socket, can_frame_vec.as_slice()).await;

    Ok(())
}

async fn process_response(response_vec: Vec<String>, variables:&mut HashMap<String, Vec<u8>>) -> Result<bool>{
    let mut socket = IsoTpSocket::open(
        "can0",
         StandardId::new(0x123).unwrap(), 
         StandardId::new(0x321).unwrap(),
    ).unwrap();
    let message = receive_isotp_frame(socket).await?;
    
    ///////////////Testing/////////////////////
    // let message: Vec<u8> = vec![105, 3, 17, 0, 2, 1, 1, 0, 0];
    // // let message = message_vec.as_slice();
    
    let mut i=0;
    'mainloop: for value in response_vec {
        let mut acceptable_values: Vec<u8> = Vec::new();
        let mut options: Vec<String> = Vec::new();

        if value.contains('|') {
            let value = value.replace(" ", "");

            let values=  value.split("|").collect_vec();
            for (j,val) in values.iter().enumerate() {
                options.push(val[j*4+3..j*4+2].to_owned());
            }

        }
        else {options.push(value);}

        for option in options {
            if option.starts_with("0x"){
                acceptable_values.push(u8::from_str_radix(&option[2..], 16).expect("Unknown value {option}"));               
            }
            else if option.starts_with("LEN(") {
                let map_key = option[4..option.len()-1].to_owned();
                let var_len=(message[i] as u16)<<8 | (message[i+1] as u16);
                
                let var_vec = message[i+2..i+2+var_len as usize].to_owned();
                i+=2+var_len as usize;
    
                variables.insert(map_key, var_vec);

                continue 'mainloop;
            }
            else if option.starts_with("RANGE(") {
                let lower = u8::from_str_radix(&option[8..10],16).unwrap();
                let higher = u8::from_str_radix(&option[13..15], 16).unwrap();

                for val in lower..=higher{
                    acceptable_values.push(val);
                }
            }
            else {
                error!("Command {option} unknown");
                continue 'mainloop;
            }
        }

        for acc_value in acceptable_values {
            if acc_value == message[i] {
                i+=1;
                continue 'mainloop;
            }
        }
        return Ok(false);
    }

    Ok(true)
}