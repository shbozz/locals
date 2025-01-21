// Copyright 2025 Shbozz.
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

mod save;
use futures::stream::StreamExt;
use libp2p::{gossipsub, mdns, noise, swarm::NetworkBehaviour, swarm::SwarmEvent, tcp, yamux};
use core::str;
use std::error::Error;
use std::time::{Duration, SystemTime};
use tokio::{io, io::AsyncBufReadExt, select};
use tracing_subscriber::EnvFilter;
use std::collections::HashMap;
use libp2p::gossipsub::MessageId;
use sha3_rust::sha3_256;
use crate::chat::save::{put_message_parts, put_message_parts_with_id, put_peer_parts, stop_db};

// We create a custom network behaviour that combines Gossipsub and Mdns.
#[derive(NetworkBehaviour)]
struct ChatBehaviour {
    gossipsub: gossipsub::Behaviour,
    mdns: mdns::tokio::Behaviour,
}

pub fn sane_id(id: MessageId) -> String {
    let mut s = format!("{}", id);
    let keep = [false, true];
    let mut iter = keep.iter().cycle();
    s.retain(|_| *iter.next().unwrap());
    return s
}

#[tokio::main]
pub async fn chat() -> Result<(), Box<dyn Error>> {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .try_init();    

    // Prompt the user to input a username
    println!("Enter the name for your group:");
    let mut group_name = String::new();
    std::io::stdin().read_line(&mut group_name).expect("Err 1001: Failed to read name");
    group_name = group_name.trim().to_lowercase();
    let access_name = group_name.to_owned();

    // Prompt the user to input a username
    println!("Enter a username:");
    let mut users_username = String::new();
    std::io::stdin().read_line(&mut users_username).expect("Err 1002: Failed to read name");
    users_username = users_username.trim().to_lowercase();
    let conn = save::init_db(access_name, &users_username);
    
    let mut swarm = libp2p::SwarmBuilder::with_new_identity()
        .with_tokio()
        .with_tcp(
            tcp::Config::default(),
            noise::Config::new,
            yamux::Config::default,
        )?
        .with_quic()
        .with_behaviour(|key| {
            // To content-address message, we can take the hash of message and use it as an ID.
            let message_id_fn = |message: &gossipsub::Message| {
                let msg_time = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).expect("Err 1003: Time went backwards").as_secs();
                let hash = sha3_256(message.data.as_slice());
                let (cut_hash, _) = hash.split_at(16);
                let mut btr_hash = String::new();
                for num in cut_hash {
                    btr_hash.push_str(&num.to_string())
                }
                let time_and_hash = msg_time.to_string() + btr_hash.as_str();
                gossipsub::MessageId::from(time_and_hash)
            };

            // Set a custom gossipsub configuration
            let gossipsub_config = gossipsub::ConfigBuilder::default()
                .heartbeat_interval(Duration::from_secs(10)) // This is set to aid debugging by not cluttering the log space
                .validation_mode(gossipsub::ValidationMode::Strict) // This sets the kind of message validation. The default is Strict (enforce message signing)
                .message_id_fn(message_id_fn) // Content-address messages. No two messages of the same content will be propagated.
                .build()
                .map_err(|msg| io::Error::new(io::ErrorKind::Other, msg))?; // Temporary hack because `build` does not return a proper `std::error::Error`.

            // build a gossipsub network behaviour
            let gossipsub = gossipsub::Behaviour::new(
                gossipsub::MessageAuthenticity::Signed(key.clone()),
                gossipsub_config,
            )?;

            let mdns =
                mdns::tokio::Behaviour::new(mdns::Config::default(), key.public().to_peer_id())?;
            Ok(ChatBehaviour { gossipsub, mdns })
        })?
        .with_swarm_config(|c| c.with_idle_connection_timeout(Duration::from_secs(60)))
        .build();

    // Create a Gossipsub topic with group name
    let topic = gossipsub::IdentTopic::new(group_name);
    // subscribes to our topic
    swarm.behaviour_mut().gossipsub.subscribe(&topic)?;

    // Read full lines from stdin
    let mut stdin = io::BufReader::new(io::stdin()).lines();

    // Listen on all interfaces and whatever port the OS assigns
    swarm.listen_on("/ip4/0.0.0.0/udp/0/quic-v1".parse()?)?;
    swarm.listen_on("/ip4/0.0.0.0/tcp/0".parse()?)?;

    println!("Enter messages here and they will be sent to connected peers using Gossipsub");
    println!("To quit, type: q \
              then hit enter.");
    
    // Create variables for chatroom
    let mut addr: String;
    let mut known_peers: HashMap<String, String> = HashMap::new();
    let mut firstmsg = true;
    if swarm.external_addresses().next().is_none() {
        addr = "/ip4/127.0.0.1/tcp/0".to_owned();
    } else {
        addr = swarm.external_addresses().next().unwrap().to_string();
    }
    loop {
        select! {
            Ok(Some(line)) = stdin.next_line() => {
                if firstmsg {
                    firstmsg = false;
                    println!("Your IP address is: {}", addr);
                    let send_message = format!("{}*@{}*@{}", addr, users_username, line);
                    if let Err(e) = swarm
                        .behaviour_mut().gossipsub
                        .publish(topic.clone(), send_message.as_bytes()) {
                        println!("Publish error: {e:?}");
                    }
                } else if line == "q" {
                    stop_db(conn);
                    std::process::exit(0);
                } else {
                    if swarm.external_addresses().next().is_none() {
                        addr = "/ip4/127.0.0.1/tcp/0".to_owned();
                    } else { 
                        addr = swarm.external_addresses().next().unwrap().to_string();
                    }
                    let send_message = format!("{}*@{}", addr, line);
                    if let Err(e) = swarm
                        .behaviour_mut().gossipsub
                        .publish(topic.clone(), send_message.as_bytes()) {
                        println!("Publish error: {e:?}");
                    }
                }
            put_message_parts_with_id(SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).expect("Err 1004: Time went backwards").as_secs(), line, 0, &conn);    
            }
            event = swarm.select_next_some() => match event {
                SwarmEvent::Behaviour(ChatBehaviourEvent::Mdns(mdns::Event::Discovered(list))) => {
                    for (peer_id, _multiaddr) in list {
                        println!("mDNS discovered a new peer: {peer_id}");
                        swarm.behaviour_mut().gossipsub.add_explicit_peer(&peer_id);
                    }
                },
                SwarmEvent::Behaviour(ChatBehaviourEvent::Mdns(mdns::Event::Expired(list))) => {
                    for (peer_id, _multiaddr) in list {
                        println!("mDNS discover peer has expired: {peer_id}");
                        swarm.behaviour_mut().gossipsub.remove_explicit_peer(&peer_id);
                    }
                },
                SwarmEvent::Behaviour(ChatBehaviourEvent::Gossipsub(gossipsub::Event::Message {
                    propagation_source: peer_id,
                    message_id: id,
                    message,
                })) => {
                    let message_str = match String::from_utf8(message.data.clone()) {
                        Ok(s) => s,
                        Err(_) => {
                            println!("Err 1005: Failed conversion from UTF-8 to String \
                              Using lossy conversion");
                            String::from_utf8_lossy(&message.data.clone()).to_string()
                        },
                    };
                    let message_parts: Vec<&str> = message_str.splitn(3, "*@").collect();
                    let msg_id = sane_id(id.clone());
                    let (msg_time, received_hash) = msg_id.split_at(10);
                    
                    let hash = sha3_256(message.data.as_slice());
                    let (cut_hash, _) = hash.split_at(16);
                    let mut btr_hash = String::new();
                    for num in cut_hash {
                        btr_hash.push_str(&num.to_string())
                    }
                    if btr_hash != received_hash {
                        println!("Warning: The hash of the message that was received is incorrect. \
                                  The message may have been tampered with.");
                        println!("hash: {}", btr_hash);
                        println!("received hash: {received_hash}");
                    }

                    // For First Message
                    // Message_Parts[0] = IP
                    // Message_Parts[1] = Username
                    // Message_Parts[2] = Message

                    // For Subsequent Messages
                    // Message_Parts[0] = IP
                    // Message_Parts[1] = Message

                    // Check if peer is known
                    if !known_peers.contains_key(&message_parts[0].to_string()) {
                        // Add peer IPs and usernames to the vector
                        known_peers.insert(message_parts[0].to_string(), message_parts[1].to_string());                        

                        // Print the message and its data
                        println!(
                        "Got message: '{}' with id: {msg_id} from: '{}' | peer: {peer_id} at {msg_time}", &message_parts[2], &message_parts[1],);
                        // Add to the database
                        put_peer_parts(message_parts[0].to_string(), message_parts[1].to_string(), &conn);
                        put_message_parts(msg_time.parse::<u64>().expect("Err 1006: Letters inside of numbers"), message_parts[2].to_owned(), message_parts[1].to_owned(), &conn);
                    } else {                                             
                        // Print the message and its data
                        let sender = known_peers.get(&message_parts[0].to_string()).expect("Err 0007: Failed to find username");
                        println!(
                            "Got message: '{}' with id: {msg_id} from: '{}' | peer: {peer_id} at {msg_time}", &message_parts[1], sender,
                        );
                        // Add to the database
                        put_message_parts(msg_time.parse::<u64>().expect("Err 1008: Letters inside of numbers"), message_str.clone(), sender.to_owned(), &conn);
                    }
                },
                SwarmEvent::NewListenAddr { address, .. } => {
                    println!("Local node is listening on {address}");
                }
                _ => {}
            }
        }
    }
    }
