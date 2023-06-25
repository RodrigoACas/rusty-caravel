use std::collections::HashMap;
use std::fs::{File, self};
use std::process::{Command, Stdio};
use hex;
use itertools::Itertools;
use std::io::{Read, Write};
use std::path::{Path};
use anyhow::Result;
use log::{info, error, debug};
use serde_json::{self, Value};
use async_recursion::async_recursion;


use super::canutil::{send_isotp_frame, IsoTpSocket, ExtendedId, StandardId, Id, receive_isotp_frame, FlowControlOptions, send_can_frame, CANSocket, CANFrame};

pub async fn exec_test(file_path: String) -> Result<()>{
    let mut file = File::open(file_path).expect("Couldn't open test file");
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;

    let json: Value = serde_json::from_str(&contents)?;

    let mut dest:u32=0;
    let mut src_isotp: Id;
    let mut dest_isotp: Id; 
    if let Some(struc) = StandardId::new(0){
        src_isotp=Id::Standard(struc);
        dest_isotp=Id::Standard(struc.clone());
    } else {panic!("Couldn't create ids");}
    
    process_json(json, None, &mut src_isotp, &mut dest_isotp, &mut dest).await?;


    Ok(())
}

#[async_recursion]
async fn process_json(json: Value, key_op:Option<String>, src_isotp: &mut Id, dest_isotp: &mut Id, dest:&mut u32) -> Result<()>{
    match json {
        Value::String(value) => {
            if let Some(key) = key_op {
                match key.as_str() {
                    "TestSuitName" => {
                        info!("Initiating tests to {value}");
                    }
                    "ID" => {
                        let ids= value.split(',').collect_vec();
                        *dest=u32::from_str_radix(&ids[2][2..],16).unwrap();
                        match ids[0] {
                            "Extended" => {
                                let src_struc_opt = ExtendedId::new(u32::from_str_radix(&ids[1][2..],16).unwrap());
                                if let Some(src_struc) = src_struc_opt {
                                    *src_isotp= Id::Extended(src_struc);
                                } else {panic!("Panicked creating id from {}", ids[1])}
                                
                                let dest_struc_opt = ExtendedId::new(u32::from_str_radix(&ids[2][2..],16).unwrap());
                                if let Some(dest_struc) = dest_struc_opt {
                                    *dest_isotp = Id::Extended(dest_struc);
                                } else {panic!("Panicked creating id from {}", ids[2])}
                                
                            }
                            "Standard" => {
                                let src_struc_opt = StandardId::new(u16::from_str_radix(&ids[1][2..],16).unwrap());
                                if let Some(src_struc) = src_struc_opt {
                                    *src_isotp= Id::Standard(src_struc);
                                } else {panic!("Panicked creating id from {}", ids[1])}
                                
                                let dest_struc_opt = StandardId::new(u16::from_str_radix(&ids[2][2..],16).unwrap());
                                if let Some(dest_struc) = dest_struc_opt {
                                    *dest_isotp = Id::Standard(dest_struc);
                                } else {panic!("Panicked creating id from {}", ids[2])}
                            }
                            _ => {
                                panic!("Unknown ID type {}", ids[0]);
                            }
                        }
                    }
                    _ => {info!("Unknown key {}", key);}
                }
            }
        }
        Value::Array(values) => {
            if let Some(key) = key_op{
                match key.as_str(){
                    "Tests" => {
                        for value in values{
                            process_json(value, None, src_isotp, dest_isotp, dest).await?;
                        }
                    }
                    "Sequence" => {
                        process_sequence(values, src_isotp, dest_isotp, dest).await.expect("Failed at processing sequence");                        
                        std::thread::sleep(std::time::Duration::from_millis(50));                 
                    }
                    _ => {}
                }
            }
            
        }
        Value::Object(obj) => {
            for (key, value) in obj {
                process_json(value, Some(key), src_isotp, dest_isotp, dest).await?;
            } 
        }
        _ => {}
    }
    
    Ok(())
}

async fn process_sequence(objects: Vec<Value>, src_isotp: &mut Id, dest_isotp: &mut Id, dest: &mut u32) -> Result<()> {
    info!("Starting to process sequence");

    let mut vars: HashMap<String, Vec<u8>> = HashMap::new();
    let mut testname: String = "Placeholder Test Name".to_owned();

    let req_socket = CANSocket::open("can0").expect("Failed to open request socket");
    let mut res_socket = IsoTpSocket::open(
        "can0",
        src_isotp.to_owned(), 
        dest_isotp.to_owned(),
    ).expect("Failed to open ISO-TP socket");

    for object in objects {
        if let Value::Object(obj) = object {
            for (_key, value) in obj {
                if let Value::String(string) = value {
                    info!("Initiating test {string}");   
                    testname=string;    
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
                    
                    process_request(request_vec, &mut vars, &req_socket, dest).await?;
                    let res = process_response(response_vec, &mut vars, &mut res_socket).await?;
                    if !res {
                        info!("Messages didn't match");
                        info!("Failed {}", testname);
                        return Ok(());

                    }
                    else {
                        info!("Messages matched");
                    }
                    
                    
                }
            } 
        }
    }
    info!("Passed {}", testname);

    Ok(())
}

async fn process_request(request_vec: Vec<String>, variables:&mut HashMap<String, Vec<u8>>, socket: &CANSocket, id:&u32) -> Result<()>{
    let mut can_frame_vec: Vec<u8>= Vec::new();
    let mut len_next_flag = false;

    for value in request_vec {
        let value = value.replace(" ", "");

        if value.starts_with("0x"){
            can_frame_vec.push(u8::from_str_radix(&value[2..], 16).unwrap());
        }
        else if value.starts_with("FILE("){
            let value_len=value.chars().count();
            if !Path::new(&value[5..value_len-1]).is_file() {
                panic!("{} isn't a file", &value[5..value_len-1]);
            }
        
            let mut cert_vec = fs::read(&value[5..value_len-1]).expect("failed to read certificate");
            can_frame_vec.append(&mut cert_vec);
        }
        else if value.starts_with("LEN(RES(") {
            let len_key = value.chars().count();
            let pair = value[8..len_key-2].to_owned();

            let values=  pair.split(",").collect_vec();

            let challenge = variables.get(values[0]).unwrap();
            let priv_key_path = values[1];
            if !Path::new(&priv_key_path).is_file() {
                panic!("{} isn't a file", &priv_key_path);
            }

            let mut file = File::create("challenge").unwrap();
            file.write_all(challenge.as_slice())?;

            let mut openssl_cmd = Command::new("openssl");
            openssl_cmd.args(&[
                    "dgst",
                    "-sha256",
                    "-sign",
                    priv_key_path,
                    "-out",
                    "./signature.bin",
                    "./challenge"
                ])
                .output().expect("Couldn't execute openssl command");

            let mut signature: Vec<u8> = fs::read("signature.bin").expect("Couldn't read signature");

            // dbg!("signature is {}", signature.len());

        
            let len_var = signature.len() as u16;

            if len_next_flag {
                // dbg!("Introducing length of next field");
                let len_nextfield = (2+len_var).to_be_bytes();
                can_frame_vec.extend_from_slice(&len_nextfield);
                len_next_flag=false;
            }
            can_frame_vec.push(((len_var & 65280)>>8) as u8);
            can_frame_vec.push((len_var & 255) as u8);

            can_frame_vec.append(&mut signature);
            
            
            fs::remove_file("signature.bin")?;
            fs::remove_file("challenge")?;
        }
        else if value.starts_with("LEN_NEXT") {
            len_next_flag=true;
        }
    }

    
    //println!("Request frame: {:?}", can_frame_vec);
    // send_isotp_frame(socket, can_frame_vec.as_slice()).await;
    if can_frame_vec.len()>7 { //multi frame case
        let byte1 = ((1 as u8)<<4) | ((((can_frame_vec.len() as u16) & 0xf00)>>8) as u8);
        let byte2 = ((can_frame_vec.len() as u16) & 0x0ff) as u8;
        let mut dummy: Vec<u8> = vec![byte1, byte2];
        dummy.extend_from_slice(&can_frame_vec[0..6]);
        let frame=CANFrame::new(*id, dummy.as_slice(), false, false).expect("Couldn't construct SF");
        send_can_frame(socket, frame).await;

        std::thread::sleep(std::time::Duration::from_millis(5));

        let n_cfs = ((can_frame_vec.len()-6) as u16)/7+1; //number of consecutive frames to be sent
        for i in 0..n_cfs{
            let frame_index = if i % 16 == 15 { 0 } else { (i % 16) + 1 };
        
            let mut dummy:Vec<u8> = vec![0x20+frame_index as u8];
            if i==n_cfs-1{//last frame
                dummy.extend_from_slice(&can_frame_vec[6+7*i as usize..]);
                while dummy.len()<8 {
                    dummy.push(0 as u8);
                }
            }
            else {
                dummy.extend_from_slice(&can_frame_vec[6+7*i as usize..13+7*i as usize]);
            }
            
            let frame = CANFrame::new(*id, dummy.as_slice(), false, false).expect("Couldn't construct CF");
            send_can_frame(socket, frame).await;
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    }
    else{ //single frame case
        let mut dummy: Vec<u8> = vec![can_frame_vec.len() as u8];
        dummy.append(&mut can_frame_vec);

        while dummy.len()<8 {dummy.push(0 as u8)}
        let frame = CANFrame::new(*id, dummy.as_slice(), false, false).expect("Couldn't mount single frame");
        send_can_frame(socket, frame).await;
    } 

    Ok(())
}

async fn process_response(response_vec: Vec<String>, variables:&mut HashMap<String, Vec<u8>>, socket: &mut IsoTpSocket) -> Result<bool>{    
    // debug!("Reached response");
    let mut message: Vec<u8>;
    loop {
        message=receive_isotp_frame(socket).await?;

        if message[0]==127 && message[2]==120 {continue;} //ignore NRC 78 case
        break;
    }
    
    //debug!("Received message: {:?}", message);
    ///////////////Testing/////////////////////
    // let message: Vec<u8> = vec![105, 3, 17, 0, 2, 1, 1, 0, 0];
    // // let message = message_vec.as_slice();
    let mut skip_iter:bool=false;
    let mut i=0;
    'mainloop: for value in response_vec {
        let mut acceptable_values: Vec<u8> = Vec::new();
        let mut options: Vec<String> = Vec::new();

        if skip_iter {skip_iter=false; continue;}

        if value.contains('|') {
            let value = value.replace(" ", "");

            let values=  value.split("|").collect_vec();
            for (j,val) in values.iter().enumerate() {
                options.push(val[j*4+3..j*4+2].to_owned());
            }

        }
        else {options.push(value);}

        for option in options {
            let option=option.replace(" ", "");
            
            if option.starts_with("0x"){
                acceptable_values.push(u8::from_str_radix(&option[2..], 16).expect("Unknown value {option}"));               
            }
            else if option.starts_with("LEN(") {
                let map_key = option[4..option.len()-1].to_owned();
                let var_len=(message[i] as u16)<<8 | (message[i+1] as u16);
                
                let var_vec = message[i+2..i+2+var_len as usize].to_owned();
                i+=2+var_len as usize;
    
                variables.insert(map_key, var_vec);
                skip_iter=true;
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