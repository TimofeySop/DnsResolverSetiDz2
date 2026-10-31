use core::panic;
use std::net;
use std::net::UdpSocket;
use std::net::Ipv4Addr;
use std::net::Ipv6Addr;
use std::net::{SocketAddr, SocketAddrV4};
use rand::RngExt;

struct Message{
    header: Header,
    question: Vec<Question>,
    answer: Vec<ResourceRecord>,
    authority: Vec<ResourceRecord>,
    additional: Vec<ResourceRecord>
}

struct Header{
    id: u16,
    qr: u8,
    opcode: u8,
    aa: u8,
    tc: u8,
    rd: u8, 
    ra: u8,
    z: u8,
    rcode: u8,
    qcount: u16,
    ancount: u16,
    nscount: u16,
    arcount: u16
}

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

        let user_request = &mut buf[..amt];
        
        let initial_message = parse_message(user_request);

        //debug
        for label in &initial_message.question[0].domain {
            for symbol in label {
                print!("{}", *symbol as char);
            }

            print!(".");
        }

        //basically debug too
        let upstream_socket = UdpSocket::bind("0.0.0.0:0")?;

        let mut request = [0u8; 512];
        request[..user_request.len()].copy_from_slice(user_request);

        let mut new_id: u16 = rand::rng().random();
        request[0] = (new_id >> 8) as u8;
        request[1] = new_id as u8;

        request[2] &= !0b00000001;

        let root_addr = SocketAddr::V4(SocketAddrV4::new(root, 53));
        upstream_socket.send_to(&request[..user_request.len()], root_addr)?;


        loop{
            let mut response = [0u8; 1024];
            let (amt1, src1) = upstream_socket.recv_from(&mut response)?;
            let response = &mut response[..amt1];
            if response[0] != request[0] || response[1] != request[1]{
                panic!("Request id doesnt match response id!");
            }

            let message = parse_message(response);

            if !message.answer.is_empty() && message.answer[0].r_type == 5 {
                let new_domain: Vec<Vec<u8>>;

                match &message.answer[0].r_data {
                    RData::Cname(domain) => new_domain = domain.clone(),
                    _ => panic!("Expected CNAME data!")
                }

                let mut initial_message = parse_message(&request);
                initial_message.question[0].domain = new_domain;

                let message_built = build_message(&initial_message);

                upstream_socket.send_to(&message_built, root_addr)?;
                continue;
            }

            if message.answer.iter().any(|record| record.r_type == 1) {
                response[0] = user_request[0];
                response[1] = user_request[1];

                socket.send_to(response, src)?;
                break;
            }

            let ns_domain: Vec<Vec<u8>>;
            match &message.authority[0].r_data {
                RData::Ns(domain) => ns_domain = domain.clone(),
                _ => panic!("Expected NS record!")
            }

            let mut ns_ip: Option<Ipv4Addr> = None;

            for additional_record in &message.additional {
                if additional_record.name == ns_domain {
                    if let RData::A(ip) = &additional_record.r_data {
                        ns_ip = Some(*ip);
                        break;
                    }
                }
            }

            let ns_ip = ns_ip.expect("No glue record found!");

            let next_addr = SocketAddr::V4(SocketAddrV4::new(ns_ip, 53));

            upstream_socket.send_to(&request[..user_request.len()], next_addr)?;
        }

    }
    Ok(())
}

fn parse_questions(source: &[u8], qdcount: u16, ) -> (Vec<Question>, usize){
    if qdcount > 1 {
        panic!("Don't support multiquestion queries");
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
            result.extend(domain_part);
            return (result, octet_index);
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

fn parse_label(source: &[u8], size_byte_start: usize) -> (Vec<u8>, usize) {
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

fn parse_pointer(source: &[u8], size_byte_start: usize) -> (Vec<Vec<u8>>, usize) {
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
            //(domain, position) = parse_domain(source, position);
            let rdata_start = position;
            (domain, position) = parse_domain(source, position);

            if position - rdata_start != record_rd_length as usize {
                panic!("Invalid NS RDATA length!");
            }

            record_data = RData::Ns(domain)
        },
        5 => { //cname
            let domain: Vec<Vec<u8>>;
            //(domain, position) = parse_domain(source, position);
            let rdata_start = position;
            (domain, position) = parse_domain(source, position);

            if position - rdata_start != record_rd_length as usize {
                panic!("Invalid CNAME RDATA length!");
            }

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
        _ => {
            let len = record_rd_length as usize;
            let data = source[position..position + len].to_vec();
            position += len;
            record_data = RData::Other(data);
        }
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

fn parse_records(source: &[u8], count: u16, start: usize) -> (Vec<ResourceRecord>, usize) {
    let mut position = start;

    let mut result: Vec<ResourceRecord> = Vec::new();

    for i in 0..count {
        let record: ResourceRecord;
        (record, position) = parse_record(source, position);

        result.push(record);
    }

    (result, position)
}

fn parse_header(source: &[u8]) -> Header {
    let id = u16::from_be_bytes([source[0], source[1]]);

    let qr = (source[2] >> 7) & 0b0000000_1;
    let opcode = (source[2] >> 3) & 0b000_1111;
    let aa = (source[2] >> 2) & 0b0000000_1;
    let tc = (source[2] >> 1) & 0b0000000_1;
    let rd = source[2] & 0b0000000_1;
    println!("id of the requst: {}", id);
    println!("qr: {} opcode: {} aa: {} tc: {} rd: {}", qr, opcode, aa, tc, rd);

    let ra = (source[3] >> 7) & 0b0000000_1;
    let z = (source[3] >> 4) & 0b00000_111;
    let rcode = source[3] & 0b0000_1111;
    println!("ra: {} z: {} rcode: {}", ra, z, rcode);

    let qdcount = u16::from_be_bytes([source[4], source[5]]);
    let ancount = u16::from_be_bytes([source[6], source[7]]);
    let nscount = u16::from_be_bytes([source[8], source[9]]);
    let arcount = u16::from_be_bytes([source[10], source[11]]);

    return Header { id, qr, opcode, aa, tc, rd, ra, z, 
                    rcode, qcount: qdcount, ancount, nscount, arcount };
}

fn parse_message(source: &[u8]) -> Message{
    let header = parse_header(source);
    let question: Vec<Question>;
    let answer: Vec<ResourceRecord>;
    let authority: Vec<ResourceRecord>;
    let additional: Vec<ResourceRecord>;
    let mut position: usize;
    (question, position) = parse_questions(source, header.qcount);
    (answer, position) = parse_records(source, header.ancount, position);
    (authority, position) = parse_records(source, header.nscount, position);
    (additional, position) = parse_records(source, header.arcount, position);

    return Message { header, question, answer, authority, additional }
}

fn build_message(message: &Message) -> Vec<u8> {
    let mut result: Vec<u8> = Vec::new();
    result.extend_from_slice(&message.header.id.to_be_bytes());

    let flags1 =
        (message.header.qr << 7) |
        (message.header.opcode << 3) |
        (message.header.aa << 2) |
        (message.header.tc << 1) |
        message.header.rd;

    let flags2 =
        (message.header.ra << 7) |
        (message.header.z << 4) |
        message.header.rcode;

    result.push(flags1);
    result.push(flags2);

    result.extend_from_slice(&(message.question.len() as u16).to_be_bytes());
    result.extend_from_slice(&(message.answer.len() as u16).to_be_bytes());
    result.extend_from_slice(&(message.authority.len() as u16).to_be_bytes());
    result.extend_from_slice(&(message.additional.len() as u16).to_be_bytes());

    for question in &message.question {
        write_domain(&mut result, &question.domain);
        result.extend_from_slice(&question.q_type.to_be_bytes());
        result.extend_from_slice(&question.q_class.to_be_bytes());
    }

    for record in &message.answer {
        write_record(&mut result, record);
    }

    for record in &message.authority {
        write_record(&mut result, record);
    }

    for record in &message.additional {
        write_record(&mut result, record);
    }

    result
}

fn write_domain(result: &mut Vec<u8>, domain: &Vec<Vec<u8>>) {
    for label in domain {
        result.push(label.len() as u8);
        result.extend_from_slice(label);
    }

    result.push(0);
}

fn write_record(result: &mut Vec<u8>, record: &ResourceRecord) {
    write_domain(result, &record.name);

    result.extend_from_slice(&record.r_type.to_be_bytes());
    result.extend_from_slice(&record.r_class.to_be_bytes());
    result.extend_from_slice(&record.ttl.to_be_bytes());

    let mut rdata: Vec<u8> = Vec::new();

    match &record.r_data {
        RData::A(ip) => rdata.extend_from_slice(&ip.octets()),
        RData::AAAA(ip) => rdata.extend_from_slice(&ip.octets()),
        RData::Ns(domain) | RData::Cname(domain) => {
            write_domain(&mut rdata, domain);
        },
        RData::Other(data) => rdata.extend_from_slice(data),
    }

    result.extend_from_slice(&(rdata.len() as u16).to_be_bytes());
    result.extend_from_slice(&rdata);
}