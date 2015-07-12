use std::thread;
use std::net;
use std::str;

fn socket(listen_on: net::SocketAddr) -> net::UdpSocket {
    let attempt = net::UdpSocket::bind(listen_on);
    let mut socket;
    match attempt {
        Ok(sock) => {
            println!("Bound socket to {}", listen_on);
            socket = sock;
        },
        Err(err) => panic!("Could not bind: {}", err)
    }
    socket
}


fn handle_read_request(data: &[u8; 100] )  {
    println!("Read Request");

    let ignored: u8 = data[0];
    let opcode: u8 = data[1];

    let mut index = 2;
    
    for x in 2..100 {
        if (data[x] == 0){
            index = x;
            break;
        }
    }
    
    let filename = match str::from_utf8(&data[2..index]) {
        Ok(v) => v,
        Err(e) => panic!("Invalid UTF-8 sequence: {}", e),
    };
    println!("filename: {}", filename);

    let mode = match str::from_utf8(&data[index..100]) {
        Ok(v) => v,
        Err(e) => panic!("Invalid UTF-8 sequence: {}", e),
    };
    println!("mode: {}", mode);
}



fn read_message(socket: net::UdpSocket) {
    let mut buf: [u8; 100] = [0; 100];
    println!("Reading data");
    let result = socket.recv_from(&mut buf);
    drop(socket);
    match result {
        Ok((amt, src)) => {
            println!("Received data from {}", src);
            println!("Amount is  {}", amt);

            if  (amt < 2){
                panic!("Note enough data in packet")
            }
            let opcode = buf[1];
            match opcode {
                1 => handle_read_request(&buf),
                2 => println!("Write"),
                3 => println!("Data"),
                4 => println!("ACK"),
                5 => println!("ERROR"),
                _ => println!("Illegal Op code"),
            }
        },
        Err(err) => panic!("Read error: {}", err)
    }
}

pub fn send_message(send_addr: net::SocketAddr, target: net::SocketAddr, data: Vec<u8>) {
    let socket = socket(send_addr);
    println!("Sending data");
    let result = socket.send_to(&data, target);
    drop(socket);
    match result {
        Ok(amt) => println!("Sent {} bytes", amt),
        Err(err) => panic!("Write error: {}", err)
    }
}

pub fn listen(listen_on: net::SocketAddr) {
    let socket = socket(listen_on);
    thread::spawn(move || {
        read_message(socket)
    });
}



fn main(){
    let ip = net::Ipv4Addr::new(127, 0, 0, 1);
    let listen_addr = net::SocketAddrV4::new(ip, 8888);
    let ip = net::Ipv4Addr::new(127, 0, 0, 1);
    let listen_addr = net::SocketAddrV4::new(ip, 8888);
    let future = listen(net::SocketAddr::V4(listen_addr));

    loop{
        thread::sleep_ms(3000);
    }
    //let received = future.join().unwrap();
    //println!("Got {} bytes", received.len());
}
