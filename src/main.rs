// Copyright 2024 Shbozz.
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

mod chat;

//Runs chat function and handles errors by printing them
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let err = chat::chat();
    let fmterr: String = format!("{:?}", err);
    println!("{}", fmterr);
    println!("Bye!");
    Ok(())
}
