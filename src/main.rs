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

use byteorder::ReadBytesExt;
use byteorder::{BigEndian, WriteBytesExt};

use std::collections::HashMap;
use std::env;
use std::fs::File;
use std::io::BufReader;
use std::io::Cursor;
use std::io::prelude::*;
use std::io::SeekFrom;
use std::net;
use std::str;

struct Connection<'a> {
    src: &'a net::SocketAddr,
    socket: &'a net::UdpSocket,
}

impl<'a> Connection<'a> {
    pub fn send_response(&self, data: &[u8]) {
        let result = self.socket.send_to(data, &self.src);
        match result {
            Ok(_) => {}
            Err(err) => panic!("Write error: {}", err),
        }
    }
    pub fn send_error(&self, code: u16, err: &str) {
        let mut message = Vec::new();
        message.push((code >> 8) as u8);
        message.push(code as u8);
        message.extend(err.as_bytes());
        self.send_response(&message);
    }
}

struct FileStream {
    reader: BufReader<File>,
    chunks: u64,
    pos: u64,
    done: bool,
}

impl FileStream {
    pub fn new(data: &mut Cursor<&Vec<u8>>, amt: &usize) -> FileStream {
        let mut parts = data.get_ref().as_slice()[2..].split(|b| *b == b'\x00');
        let name = str::from_utf8(parts.next().unwrap()).unwrap();
        let mode = str::from_utf8(parts.next().unwrap()).unwrap();
        println!("Request name: {:?}", name);
        println!("Mode: {:?}", mode);

        let mut abs_path = env::current_dir().unwrap();
        abs_path.push(name);
        println!("Absolute path: {:?}", abs_path.display());

        let file = match File::open(abs_path) {
            Err(err) => panic!("Can't open file: {}", err),
            Ok(file) => file,
        };

        let len = file.metadata().unwrap().len();
        println!("Found file length: {}", len);
        let reader = BufReader::new(file);
        let chunks = if len % 512 == 0 {
            len / 512
        } else {
            len / 512 + 1
        };
        println!("Calculated length of {} chunks", chunks);
        return FileStream {
            reader: reader,
            chunks,
            pos: 0,
            done: false,
        };
    }

    pub fn send_chunk(&mut self, chunk: u64, connection: &Connection) {
        if chunk > self.chunks {
            let end = [0u8, 3, (chunk >> 8) as u8, chunk as u8];
            connection.send_response(&end);
            println!("Requested chunks past end -> sent empty DATA and set to done");
            self.done = true;
            return;
        }

        let offset = (chunk - 1) * 512;
        if offset != self.pos {
            println!("Seeking to offset {}", offset);
            match self.reader.seek(SeekFrom::Start(offset)) {
                Ok(new_pos) => println!("Seek to {}", new_pos),
                Err(err) => panic!("Seek error: {}", err),
            }
        }

        let mut buf = Vec::with_capacity(516);
        buf.write_u16::<BigEndian>(3).unwrap();
        buf.write_u16::<BigEndian>(chunk as u16).unwrap();
        buf.resize(516, 0);
        let read = match self.reader.read(&mut buf[4..]) {
            Ok(l) => l,
            Err(e) => {
                println!("Send read error {}", e);
                connection.send_error(0, &e.to_string());
                return;
            }
        };
        if read < 512 {
            println!("Sending incomplete block -> set to done");
            self.done = true;
        }

        self.pos = offset + (read as u64);
        connection.send_response(&buf[..4 + read]);
        if chunk % 1000 == 0 || chunk > self.chunks - 10 {
            println!("Sent block {} with {} bytes", chunk, read);
        }
    }
}

fn socket(listen_on: net::SocketAddr) -> net::UdpSocket {
    let attempt = net::UdpSocket::bind(listen_on);
    let socket;
    match attempt {
        Ok(sock) => {
            println!("Bound socket to {}", listen_on);
            socket = sock;
        }
        Err(err) => panic!("Could not bind: {}", err),
    }
    socket
}

fn read_message(socket: &net::UdpSocket) {
    let mut file_streams = HashMap::new();

    let mut buf: [u8; 100] = [0; 100];
    loop {
        let result = socket.recv_from(&mut buf);

        match result {
            Ok((amt, src)) => {
                let data = Vec::from(&buf[0..amt]);
                let connection = Connection {
                    socket: socket,
                    src: &src,
                };
                let mut rdr = Cursor::new(&data);

                if amt < 2 {
                    panic!("Not enough data in packet")
                }
                let opcode = rdr.read_u16::<BigEndian>().unwrap();

                match opcode {
                    1 => {
                        let mut stream = FileStream::new(&mut rdr, &amt);
                        stream.send_chunk(1, &connection);
                        file_streams.insert(src, stream);
                    }
                    2 => println!("Write"),
                    3 => println!("Data"),
                    4 => {
                        let stream = file_streams.get_mut(&src).unwrap();
                        if !stream.done {
                            let chunk = rdr.read_u16::<BigEndian>().unwrap() + 1;
                            stream.send_chunk(chunk as u64, &connection);
                        }
                    }
                    5 => println!("ERROR"),
                    _ => println!("Illegal Op code"),
                }
            }
            Err(err) => panic!("Read error: {}", err),
        }
    }
}

pub fn send_response(socket: &net::UdpSocket, src: &net::SocketAddr, data: &Vec<u8>) {
    let result = socket.send_to(&data, src);
    match result {
        Ok(amt) => println!("Sent {} bytes", amt),
        Err(err) => panic!("Write error: {}", err),
    }
}

pub fn listen(listen_on: net::SocketAddr) {
    let socket = socket(listen_on);
    read_message(&socket)
}

fn main() {
    let ip = net::Ipv4Addr::new(0, 0, 0, 0);
    let listen_addr = net::SocketAddrV4::new(ip, 69);
    listen(net::SocketAddr::V4(listen_addr));
}
