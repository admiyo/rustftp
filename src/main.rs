// Copyright 2016, Adam Young

// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.


extern crate byteorder;
use std::io::Cursor;
use std::fs::File;
use std::io::BufReader;
use std::io::prelude::*;
use std::net;
use std::collections::HashMap;
use std::io::SeekFrom;
use byteorder::ReadBytesExt;
use byteorder::{BigEndian, WriteBytesExt};
use std::error::Error;
use std::str;

struct Connection<'a>{
    src:  &'a net::SocketAddr,
    socket: &'a net::UdpSocket,
}


impl<'a> Connection<'a>{
    pub fn send_response(&self,   data: Vec<u8>) {
        let result = self.socket.send_to(&data, &self.src);
        match result {
            Ok(amt) => println!("Sent response with {} bytes", amt),
            Err(err) => panic!("Write error: {}", err)
        }
    }
}

struct FileStream{
    reader: BufReader<File>,
}


impl  FileStream{
    pub fn new(data: &mut Cursor<&Vec<u8>>, amt: &usize,) -> FileStream {

        let mut index = 2;
        for x in 2..20 {
            if data.get_ref().as_slice()[x] == 0{
                index = x;
                break;
            }
        }
        let mut full_path = String::from("/home/ayoung/tftp/");
        let filename = match str::from_utf8(&data.get_ref().as_slice()[2..index]) {
            Ok(file_name) => file_name,
            Err(why) => panic!("couldn't read filename: {}",
                               Error::description(&why)),
        };
        full_path.push_str(filename);
        println!("filename: {}", filename);

        index += 1;

        let mode = match str::from_utf8(&data.get_ref().as_slice()[index..*amt]) {
            Ok(v) => v,
            Err(e) => panic!("Invalid UTF-8 sequence: {}", e),
        };
        println!("mode: {}", mode);
        println!("amount: {}", amt);
        println!("mode length: {}", mode.len());


        let file = match File::open(full_path){
            Err(err) => panic!("Can't open file: {}", err),
            Ok(file) => file,
        };

        let reader = BufReader::new(file);
        return FileStream{
            reader: reader,
        };
    }

    pub fn send_chunk(&mut self, chunk: &u16, connection: &Connection){
        println!("Sending chunk {}" , chunk);

        let chunk2: u64 = *chunk as u64;
        let offset: u64 = ((chunk2 - 1) * 512) as u64;
        match self.reader.seek(SeekFrom::Start(offset)) {
            Ok(amt) => println!("Sent {} bytes", amt),
            Err(err) => panic!("Write error: {}", err)
        }

        let mut buf=[0u8; 512];
        let bytes_read;
        let result = self.reader.read(&mut buf);
        match result {
            Ok(l) => bytes_read = l,
            Err(e) =>  {
                let mut message:Vec<u8> =Vec::new();
                message.push(0);
                message.push(5);
                message.extend(e.to_string().into_bytes());
                connection.send_response(message);

                return
            }

        }

        let content = buf.to_vec();
        let mut message:Vec<u8> =Vec::new();

        //message.write_u16::<BigEndian>(3).unwrap();
        //message.write_u16::<BigEndian>(*chunk).unwrap();

        message.push(0);
        message.push(3);
        message.write_u16::<BigEndian>(*chunk).unwrap();

        for i in 0..bytes_read{
            message.push(content[i]);
        }

        connection.send_response(message);
        println!("sending block :  {}", chunk);

    }
}




fn socket(listen_on: net::SocketAddr) -> net::UdpSocket {
    let attempt = net::UdpSocket::bind(listen_on);
    let socket;
    match attempt {
        Ok(sock) => {
            println!("Bound socket to {}", listen_on);
            socket = sock;
        },
        Err(err) => panic!("Could not bind: {}", err)
    }
    socket
}


fn handle_read_request(data: &mut Cursor<&Vec<u8>>, amt: &usize,connection: &Connection) ->FileStream {

    let chunk = 1;
    let mut stream = FileStream::new(data, &amt);
    stream.send_chunk(&chunk, connection);

    stream
}



fn read_message(socket: &net::UdpSocket) {
    let mut file_streams = HashMap::new();

    let mut buf: [u8; 100] = [0; 100];
    loop{
        let result = socket.recv_from(&mut buf);

        match result {
            Ok((amt, src)) => {
                let data = Vec::from(&buf[0..amt]);
                let connection = Connection{socket: socket, src: &src};
                let mut rdr = Cursor::new(&data);

                if amt < 2{
                    panic!("Not enough data in packet")
                }
                let opcode = rdr.read_u16::<BigEndian>().unwrap();

                match opcode {
                    1 => {
                        file_streams.insert(src, handle_read_request(
                            &mut rdr, &amt, &connection));
                    },
                    2 => println!("Write"),
                    3 => println!("Data"),
                    4 => {
                        let chunk = rdr.read_u16::<BigEndian>().unwrap() + 1;
                        file_streams.get_mut(&src).unwrap().send_chunk(&chunk, &connection);
                    },
                    5 => println!("ERROR"),
                    _ => println!("Illegal Op code"),
                }
            },
            Err(err) => panic!("Read error: {}", err)
        }
    }

}


pub fn send_response(socket: &net::UdpSocket,src:  &net::SocketAddr,   data: Vec<u8>) {
    let result = socket.send_to(&data, src);
    match result {
        Ok(amt) => println!("Sent {} bytes", amt),
        Err(err) => panic!("Write error: {}", err)
    }
}


pub fn listen(listen_on: net::SocketAddr) {
    let socket = socket(listen_on);
    read_message(&socket)
}

fn main(){


    let ip = net::Ipv4Addr::new(127, 0, 0, 1);
    let listen_addr = net::SocketAddrV4::new(ip, 8888);
    listen(net::SocketAddr::V4(listen_addr));
}
