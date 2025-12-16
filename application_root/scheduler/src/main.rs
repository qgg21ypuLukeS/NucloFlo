// Standard library imports
use std::path::PathBuf;
use std::sync::Arc;
use std::env;
use tokio::fs;
use tokio::process::Command;

// -----------------------------
// Structs
// -----------------------------
struct Job {
    id: u32,
    name: String,
    schedule: std::time::Duration,
    state: JobState,
    input_path: PathBuf,
    database: String,
    output_path: PathBuf,
    program: BlastType,
}

struct BlastExecutionRequest {
    job_id: u64,
    blast_type: BlastType,
    input: BlastInput,
    parameters: BlastParameters,
}

struct RustProcessEngine;
struct SmallDummyEngine;
struct LargeDummyEngine;
struct PythonBlastEngine;

struct BlastParameters;

struct Scheduler {
    queue: Vec<Job>,
    join_handle: Vec<tokio::task::JoinHandle<()>>,
    small_engine: Arc<dyn BlastEngine + Send + Sync>,
    large_engine: Arc<dyn BlastEngine + Send + Sync>,
    python_engine: Arc<dyn BlastEngine + Send + Sync>,
}

struct BlastResult {
    job_id: u64,
    status: ResultStatus,
    output: ResultOutput,
}

// -----------------------------
// Enums
// -----------------------------

enum JobState { 
    Queued, 
    Running, 
    Completed 
}

#[derive(Debug, Clone)]
enum BlastType { 
    BlastN, 
    BlastP, 
    BlastX, 
    TBlastN,
    TBlastX 
}

impl BlastType {
    fn to_string(&self) -> &str {
        match self {
            BlastType::BlastN => "blastn",
            BlastType::BlastP => "blastp",
            BlastType::BlastX => "blastx",
            BlastType::TBlastN => "tblastn",
            BlastType::TBlastX => "tblastx",
        }
    }
}

#[derive(Debug)]
enum BlastInput { 
    FilePath(PathBuf), 
    RawBytes(Vec<u8>) 
}

enum ResultStatus { 
    Success,
    Failed 
}

#[derive(Debug)]
enum ResultOutput { 
    FilePath(PathBuf) 
}

#[derive(Debug)]
enum BlastEngineError {
    InvalidInput(String),
    UnsupportedFormat,
    DatabaseUnavailable,
    ExecutionFailed(String),
    Timeout,
}

// -----------------------------
// Traits 
// -----------------------------
#[async_trait::async_trait]
trait BlastEngine {
    async fn execute(&self, request: BlastExecutionRequest) -> Result<BlastResult, BlastEngineError>;
    fn name(&self) -> &'static str;
}

// -----------------------------
// PYTHON BLAST ENGINE (NEW!)
// -----------------------------
#[async_trait::async_trait]
impl BlastEngine for PythonBlastEngine {
    fn name(&self) -> &'static str { "Python BLAST Engine" }

    async fn execute(&self, request: BlastExecutionRequest) -> Result<BlastResult, BlastEngineError> {
        println!("🐍 Python engine executing job {}", request.job_id);

        let input_path = match request.input {
            BlastInput::FilePath(ref path) => path,
            _ => return Err(BlastEngineError::InvalidInput(
                "Python engine requires file input".to_string()
            )),
        };

        if !input_path.exists() {
            return Err(BlastEngineError::InvalidInput(
                format!("Input file does not exist: {:?}", input_path)
            ));
        }

        // Get app root and build paths
        let exe_path = env::current_exe()
            .map_err(|e| BlastEngineError::ExecutionFailed(format!("Cannot get exe path: {}", e)))?;
        let app_root = exe_path.parent()
            .and_then(|p| p.parent())
            .and_then(|p| p.parent())
            .ok_or(BlastEngineError::ExecutionFailed("Cannot determine app root".to_string()))?;
        
        let output_dir = app_root.join("outputs");
        fs::create_dir_all(&output_dir).await
            .map_err(|e| BlastEngineError::ExecutionFailed(format!("Cannot create output dir: {}", e)))?;
        
        let output_path = output_dir.join(format!("python_blast_{}.xml", request.job_id));

        println!("📄 Input: {:?}", input_path);
        println!("💾 Output: {:?}", output_path);

        // Find the Python Flask server (assuming it's in application_root/python_engine/)
        let python_dir = app_root.join("python_engine");
        
        // Use curl to call the Flask API
        let blast_type = request.blast_type.to_string();
        
        let output = Command::new("curl")
            .arg("-X")
            .arg("POST")
            .arg("-F")
            .arg(format!("file=@{}", input_path.display()))
            .arg("-F")
            .arg(format!("blastType={}", blast_type))
            .arg("http://127.0.0.1:5001/run_blast")
            .arg("-o")
            .arg(&output_path)
            .output()
            .await
            .map_err(|e| BlastEngineError::ExecutionFailed(
                format!("Failed to call Python API: {}", e)
            ))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(BlastEngineError::ExecutionFailed(
                format!("Python API call failed: {}", stderr)
            ));
        }

        println!("✅ Python BLAST completed successfully");

        Ok(BlastResult {
            job_id: request.job_id,
            status: ResultStatus::Success,
            output: ResultOutput::FilePath(output_path),
        })
    }
}

// -----------------------------
// RUST PROCESS ENGINE
// -----------------------------
#[async_trait::async_trait]
impl BlastEngine for RustProcessEngine {
    fn name(&self) -> &'static str { "RUST engine" }

    async fn execute(&self, request: BlastExecutionRequest) -> Result<BlastResult, BlastEngineError> {
        println!("🦀 RUST engine executing job {}", request.job_id);

        let input_path = match request.input {
            BlastInput::FilePath(ref path) => path,
            _ => return Err(BlastEngineError::InvalidInput(
                "RUST engine requires file input".to_string()
            )),
        };

        // Get app root and build output path
        let exe_path = env::current_exe()
            .map_err(|e| BlastEngineError::ExecutionFailed(format!("Cannot get exe path: {}", e)))?;
        let app_root = exe_path.parent()
            .and_then(|p| p.parent())
            .and_then(|p| p.parent())
            .ok_or(BlastEngineError::ExecutionFailed("Cannot determine app root".to_string()))?;
        
        let output_dir = app_root.join("outputs");
        fs::create_dir_all(&output_dir).await
            .map_err(|e| BlastEngineError::ExecutionFailed(format!("Cannot create output dir: {}", e)))?;
        
        let output_path = output_dir.join(format!("rust_engine_{}.txt", request.job_id));
        let engine_dir = app_root.join("engines").join("rust_engine");

        let output = Command::new("cargo")
            .args(["run", "--quiet", "--"])
            .arg(request.job_id.to_string())
            .arg(input_path)
            .current_dir(&engine_dir)
            .output()
            .await
            .map_err(|e| BlastEngineError::ExecutionFailed(format!("Spawn failed: {}", e)))?;

        println!("--- Engine stdout ---\n{}", String::from_utf8_lossy(&output.stdout));
        println!("--- Engine stderr ---\n{}", String::from_utf8_lossy(&output.stderr));

        if !output.status.success() {
            return Err(BlastEngineError::ExecutionFailed("Engine failed".to_string()));
        }

        fs::write(&output_path, &output.stdout)
            .await
            .map_err(|e| BlastEngineError::ExecutionFailed(format!("Write failed: {}", e)))?;

        Ok(BlastResult {
            job_id: request.job_id,
            status: ResultStatus::Success,
            output: ResultOutput::FilePath(output_path),
        })
    }
}

// -----------------------------
// DUMMY ENGINES
// -----------------------------
#[async_trait::async_trait]
impl BlastEngine for SmallDummyEngine {
    fn name(&self) -> &'static str { "SmallDummyEngine" }

    async fn execute(&self, request: BlastExecutionRequest) -> Result<BlastResult, BlastEngineError> {
        println!("🔧 SMALL engine executing job {}", request.job_id);

        let exe_path = env::current_exe()
            .map_err(|e| BlastEngineError::ExecutionFailed(e.to_string()))?;
        let app_root = exe_path.parent()
            .and_then(|p| p.parent())
            .and_then(|p| p.parent())
            .ok_or(BlastEngineError::ExecutionFailed("Cannot determine app root".to_string()))?;
        
        let output_dir = app_root.join("outputs");
        fs::create_dir_all(&output_dir).await
            .map_err(|e| BlastEngineError::ExecutionFailed(e.to_string()))?;
        
        let output_path = output_dir.join(format!("small_result_{}.txt", request.job_id));

        fs::write(&output_path, format!("Dummy BLAST result\nJob ID: {}\n", request.job_id))
            .await
            .map_err(|e| BlastEngineError::ExecutionFailed(e.to_string()))?;

        Ok(BlastResult {
            job_id: request.job_id,
            status: ResultStatus::Success,
            output: ResultOutput::FilePath(output_path),
        })
    }
}

#[async_trait::async_trait]
impl BlastEngine for LargeDummyEngine {
    fn name(&self) -> &'static str { "LargeDummyEngine" }

    async fn execute(&self, request: BlastExecutionRequest) -> Result<BlastResult, BlastEngineError> {
        println!("🔧 LARGE engine executing job {}", request.job_id);

        let exe_path = env::current_exe()
            .map_err(|e| BlastEngineError::ExecutionFailed(e.to_string()))?;
        let app_root = exe_path.parent()
            .and_then(|p| p.parent())
            .and_then(|p| p.parent())
            .ok_or(BlastEngineError::ExecutionFailed("Cannot determine app root".to_string()))?;
        
        let output_dir = app_root.join("outputs");
        fs::create_dir_all(&output_dir).await
            .map_err(|e| BlastEngineError::ExecutionFailed(e.to_string()))?;
        
        let output_path = output_dir.join(format!("large_result_{}.txt", request.job_id));

        fs::write(&output_path, format!("Dummy BLAST result\nJob ID: {}\n", request.job_id))
            .await
            .map_err(|e| BlastEngineError::ExecutionFailed(e.to_string()))?;

        Ok(BlastResult {
            job_id: request.job_id,
            status: ResultStatus::Success,
            output: ResultOutput::FilePath(output_path),
        })
    }
}

// -----------------------------
// SCHEDULER IMPLEMENTATION
// -----------------------------

impl Scheduler {
    fn new(jobs: Vec<Job>) -> Self {
        Self {
            queue: jobs,
            join_handle: vec![],
            small_engine: Arc::new(SmallDummyEngine),
            large_engine: Arc::new(RustProcessEngine),
            python_engine: Arc::new(PythonBlastEngine),
        }
    }

    async fn run(mut self) {
        println!("Scheduler started");

        while let Some(job) = self.queue.pop() {
            println!("Dispatching job {}", job.id);

            let request = BlastExecutionRequest {
                job_id: job.id as u64,
                blast_type: job.program.clone(),
                input: BlastInput::FilePath(job.input_path.clone()),
                parameters: BlastParameters,
            };

            // Use Python engine for all jobs
            let engine = Arc::clone(&self.python_engine);

            println!("Job {} assigned to engine: {}", job.id, engine.name());

            let handle = tokio::spawn(async move {
                match engine.execute(request).await {
                    Ok(result) => println!("Job {} completed successfully. Output: {:?}", result.job_id, result.output),
                    Err(err) => println!("Job {} failed: {:?}", job.id, err),
                }
            });

            self.join_handle.push(handle);
        }

        println!("Scheduler finished dispatching jobs");

        for handle in self.join_handle {
            let _ = handle.await;
        }

        println!("All jobs completed");
    }
}

// -----------------------------
// MAIN ENTRY
// -----------------------------
#[tokio::main]
async fn main() {
    // Get input file path from command line argument (from Electron UI)
    let args: Vec<String> = env::args().collect();
    
    let input_path = if args.len() > 1 {
        PathBuf::from(&args[1])
    } else {
        eprintln!("Error: No input file provided");
        eprintln!("Usage: scheduler <path_to_fasta_file>");
        std::process::exit(1);
    };

    // Verify input file exists
    if !input_path.exists() {
        eprintln!("Error: Input file does not exist: {:?}", input_path);
        std::process::exit(1);
    }

    println!("Received input file: {:?}", input_path);

    // Create job from the provided input path
    // UI provides: input_path
    // Scheduler fills in: id, name, schedule, program, database, state, output_path
    let jobs = vec![
        Job {
            id: 1,
            name: format!("BLAST Job for {}", input_path.file_name().unwrap().to_string_lossy()),
            schedule: std::time::Duration::from_secs(0),
            program: BlastType::BlastN,  // Default to BlastN
            database: "nt".to_string(),   // Default to nucleotide database
            state: JobState::Queued,
            input_path,
            output_path: PathBuf::new(),  // Will be set by engine
        }
    ];

    let scheduler = Scheduler::new(jobs);
    scheduler.run().await;
}