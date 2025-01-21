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

use rusqlite::Connection;

// Open/Create the database and tables for storing data of group chat
pub fn init_db(access: String, username: &String) -> Connection {
    let database_exists = std::path::Path::new(&(access.clone() + ".dat")).exists();
    let conn = match Connection::open(access.clone() + ".dat") {
        Ok(conn) => conn,
        Err(_) => {
            println!("Err 2001: Failed to open database \
                      If {access} is open in another program, close that program.\
                      Trying again in 10 seconds");
            std::thread::sleep(std::time::Duration::from_secs(10));
            Connection::open(access.clone() + ".dat").expect("Err 2001: Failed to open database after retrying")
        },
    };

    conn.execute(
        "CREATE TABLE IF NOT EXISTS peers (id INTEGER NOT NULL PRIMARY KEY, ip TEXT NOT NULL, username TEXT NOT NULL)",
        (),
    ).expect("Err 2002: Failed to create peers table");
    conn.execute(
        "CREATE TABLE IF NOT EXISTS messages (id INTEGER PRIMARY KEY, time INTEGER NOT NULL, data BLOB NOT NULL, sender INTEGER REFERENCES peers(id))",
        (),
    ).expect("Err 2003: Failed to create messages table");
    //The user is inserted as a peer so messages typed can also be saved
    if !database_exists {
        conn.execute("INSERT INTO peers (id, ip, username) VALUES (?1, ?2, ?3)", (0, "/ip4/127.0.0.1/tcp/0", username)).expect("Err 2004: Failed to store user as a peer");
    }
    return conn
}

// Function for storing group chat data (to be read when the app is re-opened)
pub fn put_message_parts(time: u64, data: String, sender_username: String, conn: &Connection) {
    let mut sender_id:u8 = 0;
    let mut stmt = conn.prepare("SELECT id FROM peers WHERE username=:username;").expect("Err 2005: Failed to prepare statement");
    let _ = stmt.query_map(&[(":username", sender_username.as_str())], |row| {
        Ok(sender_id = row.get(0)?)
    });
    
    match conn.execute(
        "INSERT INTO messages (time, data, sender) VALUES (?1, ?2, ?3)",
        (time, data.clone(), sender_id),
    ) {
        Ok(_) => {},
        Err(_) => {
            println!("Err 2007: Failed to store message, Trying again");
            std::thread::sleep(std::time::Duration::from_millis(500));
            conn.execute(
                "INSERT INTO messages (time, data, sender) VALUES (?1, ?2, ?3)",
                (time, data, sender_id),
            ).expect("Err 2007: Failed to store message after retrying");
        }
    }
}

pub fn put_message_parts_with_id(time: u64, data: String, sender: u8, conn: &Connection) {
    match conn.execute(
        "INSERT INTO messages (time, data, sender) VALUES (?1, ?2, ?3)",
        (time, data.clone(), sender),
    ) {
        Ok(_) => {},
        Err(_) => {
            println!("Err 2008: Failed to store message, Trying again");
            std::thread::sleep(std::time::Duration::from_millis(500));
            conn.execute(
                "INSERT INTO messages (time, data, sender) VALUES (?1, ?2, ?3)",
                (time, data, sender),
            ).expect("Err 2008: Failed to store message after retrying");
        }
    }
}

pub fn put_peer_parts(peer_ip: String, peer_username: String, conn: &Connection) {
    match conn.execute(
        "INSERT INTO peers (ip, username) VALUES (?1, ?2)",
        (peer_ip.clone(), peer_username.clone()),
    ) {
        Ok(_) => {},
        Err(_) => {
            println!("Err 2007: Failed to store peer data, Trying again");
            std::thread::sleep(std::time::Duration::from_millis(500));
            conn.execute(
                "INSERT INTO peers (ip, username) VALUES (?1, ?2)",
                (peer_ip, peer_username),
            ).expect("Err 2009: Failed to store peer data after retrying");
        }
    }
}

// pub fn get_peer_from_ip(peer_ip: String, conn: &Connection) -> String {
//     let username = conn.execute("SELECT username FROM peers WHERE ip = (?1)", [peer_ip]).expect("Err ?: Failed to get peer username").to_string();
//     return username
// }

pub fn stop_db (conn: Connection) {
    while conn.is_busy() {
        println!("Warning: DB is busy, Waiting for 1 second, then closing...");
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
    conn.close().expect("Err 2010: Failed to close database");
}
