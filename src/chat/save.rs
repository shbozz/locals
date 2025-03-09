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

use rusqlite::{Connection, Result};

// Open/Create the database and tables for storing data of group chat
pub fn init_db(access: String, username: &String) -> (Connection, bool) {
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
        "CREATE TABLE IF NOT EXISTS peers (id INTEGER NOT NULL PRIMARY KEY, ip TEXT NOT NULL, username TEXT UNIQUE NOT NULL)",
        (),
    ).expect("Err 2002: Failed to create peers table");
    conn.execute(
        "CREATE TABLE IF NOT EXISTS messages (id INTEGER PRIMARY KEY, time INTEGER NOT NULL, data BLOB NOT NULL, sender INTEGER NOT NULL REFERENCES peers(id))",
        (),
    ).expect("Err 2003: Failed to create messages table");
    //The user is inserted as a peer so messages typed can also be saved
    if !database_exists {
        conn.execute("INSERT INTO peers (id, ip, username) VALUES (?1, ?2, ?3)", (0, "/ip4/127.0.0.1/tcp/0", username)).expect("Err 2004: Failed to store user as a peer");
    }
    return (conn, database_exists)
}

// Function for storing group chat data (to be read when the app is re-opened)
pub fn put_message_parts(time: u64, data: String, sender_username: String, conn: &Connection) {
    let mut sender_id:u8 = 100; // init with 100 for debug since 0 is taken
    let mut stmt = conn.prepare("SELECT id FROM peers WHERE username=:username;").expect("Err 2005: Failed to prepare statement");
    let _ = stmt.query_row(&[(":username", sender_username.as_str())], |row| {
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
        "INSERT OR IGNORE INTO peers (ip, username) VALUES (?1, ?2)",
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

pub fn get_last_row_id(conn: &Connection) -> Result<u32> {
    // Prepare the query to select the row with the highest id (assuming 'id' is the primary key)
    let mut stmt = conn.prepare("SELECT id FROM messages ORDER BY id DESC LIMIT 1")?;

    // Execute the query and get the id of the last row
    let row = stmt.query_row([], |row| row.get::<_, u32>(0))?;

    Ok(row)
}
pub fn read_messages(conn: &Connection) {
    let mut stmt = conn.prepare("SELECT messages.time, messages.data, peers.username FROM messages JOIN peers ON messages.sender = peers.id WHERE messages.id=:id;").expect("Err 2005: Failed to prepare statement");
    let last_id = get_last_row_id(&conn).expect("Err 2021: Failed to get last row id");
    let mut id_increment;
    let mut data = String::new();
    let mut username = String::new();
    let mut time: u64 = 0;
    
    if last_id < 50 {
        println!("Reading Last {last_id} Messages:");
        id_increment = 1;
        while id_increment < last_id {
            let _ = stmt.query_row(&[(":id", &id_increment.to_string())], |row| {
                data = row.get(1)?; username = row.get(2)?; time = row.get(0)?;
                Ok(())
            });
            println!("'{}' from: '{}' at {} | #{id_increment}", data, username, time);
            id_increment += 1;
    }
    } else {
        println!("Reading Last 50 Messages:");
        id_increment = last_id - 50;
        while id_increment <= last_id {
            let _ = stmt.query_row(&[(":id", &id_increment.to_string())], |row| {
                data = row.get(1)?; username = row.get(2)?; time = row.get(0)?;
                Ok(())
            });
            println!("'{}' from: '{}' at {} | #{id_increment}", data, username, time);
            id_increment += 1;
        }
    }
}

// pub fn get_peer_from_ip(peer_ip: String, conn: &Connection) -> String {
//     let username = conn.execute("SELECT username FROM peers WHERE ip = (?1)", [peer_ip]).expect("Err ?: Failed to get peer username").to_string();
//     return username
// }

pub fn stop_db(conn: Connection) {
    while conn.is_busy() {
        println!("Warning: DB is busy, Waiting for 1 second, then closing...");
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
    conn.close().expect("Err 2012: Failed to close database");
}
