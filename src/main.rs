use core::panic;
use std::net;
use std::net::UdpSocket;
use std::net::Ipv4Addr;
use std::net::Ipv6Addr;
use std::net::{SocketAddr, SocketAddrV4};
use rand::RngExt;

struct Question{
    domain: Vec<Vec<u8>>,
    q_type: u16,
    q_class: u16
}

struct ResourceRecord {
    name: Vec<Vec<u8>>,
    r_type: u16,
    r_class: u16,
    ttl: u32,
    rd_length: u16,
    r_data: RData
}

enum RData{
    Ns(Vec<Vec<u8>>),
    Cname(Vec<Vec<u8>>),
    A(net::Ipv4Addr),
    AAAA(net::Ipv6Addr),
    Other(Vec<u8>)
}

fn main() -> std::io::Result<()> {
    {
        let root = Ipv4Addr::new(192, 58, 128, 30);

        let socket = UdpSocket::bind("127.0.0.1:5300")?;
        println!("Listening on 127.0.0.1:5300");

        let mut buf = [0u8; 1024];
        let (amt, src) = socket.recv_from(&mut buf)?;
        println!("Received {} bytes from {}", amt, src);

        let buf = &mut buf[..amt];


        //HEADER PARSING START
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
        //HEADER PARSING END

        for i in 0..qdcount {
            println!("Query asked a question {}", i);
        }
        
        let mut question: Vec<Question>;
        let mut next: usize;
        (question, next) = parse_questions(buf, qdcount);
        

        //debug
        for label in &question[0].domain {
            for symbol in label {
                print!("{}", *symbol as char);
            }

            print!(".");
        }

        //basically debug too
        let upstream_socket = UdpSocket::bind("0.0.0.0:0")?;

        let mut request = [0u8; 512];
        request[..buf.len()].copy_from_slice(buf);

        let new_id: u16 = rand::rng().random();
        request[0] = (new_id >> 8) as u8;
        request[1] = new_id as u8;

        request[2] &= !0b00000001;

        let root_addr = SocketAddr::V4(SocketAddrV4::new(root, 53));
        upstream_socket.send_to(&request[..buf.len()], root_addr)?;

        let mut response = [0u8; 1024];
        let (amt1, src1) = upstream_socket.recv_from(&mut response)?;
        let response = &mut response[..amt1];
        if response[0] != request[0] || response[1] != request[1]{
            panic!("request id doesnt match response id!");
        }

        response[0] = buf[0];
        response[1] = buf[1];

        socket.send_to(response, &src)?;
    }
    Ok(())
}

//ADD ACTUAL PARSING OF FLAGS AND CHANGE RETURN TYPE TO CORRESPOND TO THE RECORD!!!!!!!!!
fn parse_questions(source: &[u8], qdcount: u16, ) -> (Vec<Question>, usize){ //end point
    //panic if not IN for QCLASS
    //panic if not A for QTYPE (maybe add AAAA support later)
    if qdcount > 1 {
        panic!("dont support multiquestion queries");
    }

    let mut oct_index: usize = 12;

    let mut result: Vec<Question> = Vec::new();

    for i in 0..qdcount {
        let domain: Vec<Vec<u8>>;
        (domain, oct_index) = parse_domain(source, oct_index);

        let questionType = u16::from_be_bytes([source[oct_index], source[oct_index + 1]]);
        let questionClass = u16::from_be_bytes([source[oct_index + 2], source[oct_index + 3]]);
        oct_index += 4;

        //result = domain;
        let question = Question{
            domain: domain,
            q_type: questionType,
            q_class: questionClass
        };

        result.push(question);
    }

    (result, oct_index)
}

fn parse_domain(source: &[u8], start: usize) -> (Vec<Vec<u8>>, usize){
    let mut result: Vec<Vec<u8>> = Vec::new();
    let mut octet_index = start;
    
    while source[octet_index] != 0 {
        let octet = source[octet_index];

        if (octet & 0b11_000000) == 0b11_000000 {
            let mut domain_part: Vec<Vec<u8>>;
            (domain_part, octet_index) = parse_pointer(source, octet_index);
            for label in domain_part{
                result.push(label);
            }
        } else if (octet & 0b11_000000) == 0 {
            let mut label: Vec<u8>;
            (label, octet_index) = parse_label(source, octet_index);
            result.push(label);
        } else {
            panic!("Expected label length of a pointer! Found 01 or 10 at {}", octet_index);
        }
    }
    
    (result, octet_index + 1)
}

fn parse_label(source: &[u8], size_byte_start: usize) -> (Vec<u8>, usize) { //next index + label
    let mut curr_byte: usize = 1;
    let octets_count = usize::from(source[size_byte_start] & 0b00_111111);

    let mut label: Vec<u8> = Vec::new();

    while curr_byte <= octets_count {
        let octet = source[size_byte_start + curr_byte];

        label.push(octet);

        curr_byte += 1;
    }

    (label, size_byte_start + curr_byte)
}

fn parse_pointer(source: &[u8], size_byte_start: usize) -> (Vec<Vec<u8>>, usize) { //next index + labels
    let octet_index = usize::from(
        u16::from_be_bytes([source[size_byte_start] & 0b00_111111, 
                            source[size_byte_start + 1]])
    );
    let next = size_byte_start + 2;

    let mut domain_part: Vec<Vec<u8>>;

    (domain_part, _) = parse_domain(source, octet_index);

    (domain_part, next)
}

fn parse_record(source: &[u8], start: usize) -> (ResourceRecord, usize){
    let mut position: usize = start;

    let name: Vec<Vec<u8>>;
    (name, position) = parse_domain(source, position);

    let record_type = u16::from_be_bytes([source[position], source[position + 1]]);
    let record_class = u16::from_be_bytes([source[position + 2], source[position + 3]]);
    let record_ttl = u32::from_be_bytes([source[position + 4], source[position + 5], 
                                            source[position + 6], source[position + 7]]);
    let record_rd_length = u16::from_be_bytes([source[position + 8], source[position + 9]]);
    position += 10;

    let mut record_data: RData;

    match record_type {
        1 => { //A
            if record_rd_length != 4 {
                panic!("Invalid A record length!");
            }

            let octets: [u8; 4] = source[position..position + 4]
                .try_into()
                .unwrap();

            record_data = RData::A(Ipv4Addr::from(octets));

            position += 4;
        },
        2 => { //ns
            let domain: Vec<Vec<u8>>;
            (domain, position) = parse_domain(source, position);
            record_data = RData::Ns(domain)
        },
        5 => { //cname
            let domain: Vec<Vec<u8>>;
            (domain, position) = parse_domain(source, position);
            record_data = RData::Cname(domain)
        },
        28 => { //AAAA
            if record_rd_length != 16 {
                panic!("Invalid A record length!");
            }

            let octets: [u8; 16] = source[position..position + 16]
                .try_into()
                .unwrap();

            record_data = RData::AAAA(Ipv6Addr::from(octets));

            position += 16;
        }
        _ => panic!("Unexpected record type!"),
    }

    let result = ResourceRecord{
        name: name,
        r_type: record_type,
        r_class: record_class,
        ttl: record_ttl,
        rd_length: record_rd_length,
        r_data: record_data
    };

    (result, position)
}

fn parse_answers(source: &[u8], ancount: u16){

}

fn parse_ns(source: &[u8], nscount: u16){

}

fn parse_additional(source: &[u8], arcount: u16){

}
