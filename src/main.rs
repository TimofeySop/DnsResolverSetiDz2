//use std::io;
//use std::thread;
//use std::time::Duration;
use std::{io::ErrorKind::QuotaExceeded, net::UdpSocket};

fn main() -> std::io::Result<()> {
    {
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

        //let mut next_octet = 12;
        for i in 0..qdcount {
            println!("Query asked a question {}", i);
        }
        
        let mut question: Vec<Vec<u8>>;
        let mut next: usize;
        (question, next) = parse_questions(buf, qdcount);
        for label in question {
            for symbol in label {
                print!("{}", symbol as char);
            }

            print!(".");
        }

        socket.send_to(buf, &src)?;
    }
    Ok(())
}

//ADD ACTUAL PARSING OF FLAGS AND CHANGE RETURN TYPE TO CORRESPOND TO THE RECORD!!!!!!!!!
fn parse_questions(source: &[u8], qdcount: u16, ) -> (Vec<Vec<u8>>, usize){ //end point
    //panic if not IN for QCLASS
    //panic if not A for QTYPE (maybe add AAAA support later)
    if qdcount > 1 {
        panic!("dont support multiquestion queries");
    }

    let mut oct_index: usize = 12;

    let mut result: Vec<Vec<u8>> = Vec::new();

    for i in 0..qdcount {
        let domain: Vec<Vec<u8>>;
        (domain, oct_index) = parse_domain(source, oct_index);

        result = domain;
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
    //let octet_index = usize::from(source[size_byte_start] & 0b00_111111);
    let octet_index = usize::from(
        u16::from_be_bytes([source[size_byte_start] & 0b00_111111, 
                            source[size_byte_start + 1]])
    );
    let next = size_byte_start + 2;

    let mut domain_part: Vec<Vec<u8>>;

    (domain_part, _) = parse_domain(source, octet_index);

    (domain_part, next)
}

fn parse_answers(source: &[u8], ancount: u16){

}

fn parse_ns(source: &[u8], nscount: u16){

}

fn parse_additional(source: &[u8], arcount: u16){

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



        //write label parser make a enum for either pointer or a normal label?
        // ^ write label pointer parser. PASS THE WHOLE RECORD. RECURSIVELY UNWRAP POINTERS

        //question parser function goes here

        //rr parsing goes here
        //answer
        //nscount resource record
        //additional data


        //understand difference bwtween answer, nameserver field, authority
        //, additional data completely.
        //A: nameserver is authority section (referral)
        //qdcount is question
        //ancount is final answer
        //arcount is shit to look up 
        //authority contatins domain names
        //additional section writes what it is

        //write recursive resolver which looks for answer fields.
        //write multiply thingie, cache answers, maybe support AAAA
        //DONE