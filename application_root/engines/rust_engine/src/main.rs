use std::fs;
use std::path::PathBuf;
use std::env;


fn main() {
    // Scheduler will pass job_id as first argument
    let job_id = env::args()
        .nth(1)
        .expect("job_id not provided");

    // Dummy "work"
    println!("Rust engine processed job {}", job_id);
}