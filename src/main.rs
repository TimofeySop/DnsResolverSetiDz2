//use std::io;
//use std::thread;
//use std::time::Duration;
use std::net::UdpSocket;

fn main() -> std::io::Result<()> {
    {
        let socket = UdpSocket::bind("127.0.0.1:5300")?;
        println!("Listening on 127.0.0.1:5300");

        let mut buf = [0u8; 1024];
        let (amt, src) = socket.recv_from(&mut buf)?;
        println!("Received {} bytes from {}", amt, src);

        let buf = &mut buf[..amt];
        //let id = std::str::from_utf8(&buf[..2])?;
        let id = u16::from_be_bytes([buf[0], buf[1]]);
        //verify that its a query (later)
        //verify that its a standart query
        //aa, tc no check for query
        // rd send on my own later
        //ignore z
        //panic if rcode is not 0, make a mask later
        // if qdcount != 1 send rcode 4(not implemented) in rcode
        // ic cant find, send rcode 3(name error)

        //ancount ignore for query, check for loop
        //nscount check on the last step when recieved AA answer
        //arcount search for actual ips
        //question octet groups is stuff inbetween . in qname
        //qtype????????????????????????????????????????????????????????
        //qclass only take IN
        //check out resource record and message compression

        println!("id of the requst: {}", id);
        socket.send_to(buf, &src)?;
    }
    Ok(())
}

fn get_bits_slice(source: &u8, start: i32, end: i32) -> u8 {
    if !(0 <= start && start < end && end <= 7) {
        panic!("invalid bit positions!")
    }
    let mut mask: u8 = 255;
    mask = mask << start;
    mask = mask >> start;
    mask = mask >> ( 7 - end);
    mask = mask << (7 - end);

    source.clone() & mask
}