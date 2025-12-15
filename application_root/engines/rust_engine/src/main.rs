use std::env;
use std::fs;

fn main() {
    let args: Vec<String> = env::args().collect();
    let job_id = &args[1];
    let input_path = &args[2];

    // Read input file (absolute path)
    let contents = fs::read_to_string(input_path)
        .expect(&format!("Failed to read input: {}", input_path));

    println!("RUST engine executing job {} with input length {}", job_id, contents.len());
}
