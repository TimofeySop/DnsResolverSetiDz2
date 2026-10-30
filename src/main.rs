use std::io;
use std::thread;
use std::time::Duration;

fn main() {
    let mut input = String::new();

    io::stdin()
        .read_line(&mut input)
        .expect("Failed to read line!");

    let split: Vec<String> = input
        .trim()
        .split('.')
        .map(|x| x.to_owned())
        .collect();

    loop{
        for element in &split {
            println!("element: {}", element);
        }
        thread::sleep(Duration::from_secs(1));
    }

    
}
