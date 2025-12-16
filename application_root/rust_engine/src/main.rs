// -----------------------------
// RUST ENGINE IMPLEMENTATION
// -----------------------------
use axum::{
    routing::get,
    Router,
};

use serde::Deserialize;
use axum::extract::Query;
use tokio::fs;


//---
//Struct and impl for RustProcessEngine here if needed
//---

#[derive(Deserialize)]
struct EngineInput {
    job_id: String,
    input_path: String,
}




//-----------------------------
// Handlers for the web server
//-----------------------------
#[tokio::main]
async fn main() {
    let app = Router::new()
        .route("/", get(root_handler))
        .route("/run_blast", get(run_blast));

        // 3. Start server
    let addr = "127.0.0.1:5002";
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}



async fn root_handler() -> &'static str {
    "Hello from Axum"
}

async fn run_blast(
    Query(params): Query<EngineInput>
) -> String {
    let contents = match fs::read_to_string(&params.input_path).await {
        Ok(data) => data,
        Err(err) => {
            return format!(
                "Failed to read input file '{}': {}",
                params.input_path,
                err
            );
        }
    };

    format!(
        "Job {}\nRead {} bytes from input file",
        params.job_id,
        contents.len()
    )
}
//fn main() {
//    let args: Vec<String> = env::args().collect();
//    let job_id = &args[1];
 //
 //    let input_path = &args[2];



//}





