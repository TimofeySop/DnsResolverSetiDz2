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

        let id = u16::from_be_bytes([buf[0], buf[1]]);

        let qr = (buf[2] >> 7) & 0b0000000_1;   //query if 0
        let opcode = (buf[2] >> 3) & 0b000_1111;
        let aa = (buf[2] >> 2) & 0b0000000_1;
        let tc = (buf[2] >> 1) & 0b0000000_1;
        let rd = buf[2] & 0b0000000_1;
        println!("id of the requst: {}", id);
        println!("qr: {} opcode: {} aa: {} tc: {} rd: {}", qr, opcode, aa, tc, rd);

        let ra = (buf[3] >> 7) & 0b0000000_1;
        let z = (buf[3] >> 4) & 0b00000_111; //ignore
        let rcode = buf[3] & 0b0000_1111;
        println!("ra: {} z: {} rcode: {}", ra, z, rcode);

        let qdcount = u16::from_be_bytes([buf[4], buf[5]]);
        let ancount = u16::from_be_bytes([buf[6], buf[7]]);
        let nscount = u16::from_be_bytes([buf[8], buf[9]]);
        let arcount = u16::from_be_bytes([buf[10], buf[11]]);
        println!("qdcount: {} ancount: {} nscount: {} arcount: {}"
                    , qdcount, ancount, nscount, arcount);  

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

        //where labels are used: name in resource record, 



        //write label parser
        //write label pointer parser. PASS THE WHOLE RECORD

        //question parser function goes here

        //rr parsing goes here
        //answer
        //nscount resource record
        //additional data


        socket.send_to(buf, &src)?;
    }
    Ok(())
}

//big or small endian?
fn get_bits_slice(source: u8, start: i32, end: i32) -> u8 {
    if !(0 <= start && start < end && end <= 7) {
        panic!("invalid bit positions!")
    }
    let mut mask: u8 = 255;
    mask = mask << start;
    mask = mask >> start;
    mask = mask >> ( 7 - end);
    mask = mask << (7 - end);

    (source.clone() & mask) >> (7 - end)
}