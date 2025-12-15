// Standard library imports
use std::path::PathBuf;
use std::sync::Arc;
use tokio::fs;
use tokio::process::Command; // Async process execution

// -----------------------------
// JOB MODEL
// -----------------------------
struct Job {
    id: u32,                 // Unique job identifier
    name: String,            // UI-facing name
    schedule: std::time::Duration, // Placeholder for future scheduling logic
    state: JobState,         // Job lifecycle state (queued/running/completed)
    input_path: PathBuf,     // Absolute path to input file
}

enum JobState { Queued, Running, Completed }

// -----------------------------
// BLAST EXECUTION REQUEST
// -----------------------------
struct BlastExecutionRequest {
    job_id: u64,
    blast_type: BlastType,
    input: BlastInput,
    parameters: BlastParameters,
}

struct BlastParameters;

#[derive(Debug)]
enum BlastType { BlastN, BlastP, BlastX, TBlastN, TBlastX }

enum BlastInput { FilePath(PathBuf), RawBytes(Vec<u8>) }

#[derive(Debug)]
enum BlastEngineError {
    InvalidInput(()),
    UnsupportedFormat,
    DatabaseUnavailable,
    ExecutionFailed(()),
    Timeout,
}

// -----------------------------
// ENGINE TRAIT
// -----------------------------
#[async_trait::async_trait]
trait BlastEngine {
    async fn execute(&self, request: BlastExecutionRequest) -> Result<BlastResult, BlastEngineError>;
    fn name(&self) -> &'static str;
}

// -----------------------------
// DUMMY ENGINES
// -----------------------------
struct SmallDummyEngine;
struct LargeDummyEngine;
struct RustProcessEngine;

#[async_trait::async_trait]
impl BlastEngine for RustProcessEngine {
    fn name(&self) -> &'static str { "RUST engine" }

    async fn execute(&self, request: BlastExecutionRequest) -> Result<BlastResult, BlastEngineError> {
        println!("RUST engine executing job {}", request.job_id);

        // Absolute input path
        let input_path = match request.input {
            BlastInput::FilePath(ref path) => path,
            _ => return Err(BlastEngineError::InvalidInput(())),
        };

        // Absolute path to output file
        let mut output_path = PathBuf::from("/home/lukesal/BioClick/NucloFlo/application_root/outputs");
        fs::create_dir_all(&output_path).await.unwrap(); // Ensure outputs folder exists
        output_path.push(format!("rust_engine_{}.txt", request.job_id));

        // Run the engine process (example)
        let output = Command::new("cargo")
            .args(["run", "--quiet", "--"])
            .arg(request.job_id.to_string())
            .arg(input_path) // pass absolute input
            .current_dir("/home/lukesal/BioClick/NucloFlo/application_root/engines/rust_engine")
            .output()
            .await
            .map_err(|_| BlastEngineError::ExecutionFailed(()))?;

        println!("--- Engine stdout ---\n{}", String::from_utf8_lossy(&output.stdout));
        println!("--- Engine stderr ---\n{}", String::from_utf8_lossy(&output.stderr));

        if !output.status.success() {
            return Err(BlastEngineError::ExecutionFailed(()));
        }

        fs::write(&output_path, &output.stdout)
            .await
            .map_err(|_| BlastEngineError::ExecutionFailed(()))?;

        Ok(BlastResult {
            job_id: request.job_id,
            status: ResultStatus::Success,
            output: ResultOutput::FilePath(output_path),
        })
    }
}

#[async_trait::async_trait]
impl BlastEngine for SmallDummyEngine {
    fn name(&self) -> &'static str { "SmallDummyEngine" }

    async fn execute(&self, request: BlastExecutionRequest) -> Result<BlastResult, BlastEngineError> {
        println!("SMALL engine executing job {}", request.job_id);

        let mut output_path = PathBuf::from("/home/lukesal/BioClick/NucloFlo/application_root/outputs");
        fs::create_dir_all(&output_path).await.unwrap();
        output_path.push(format!("small_result_{}.txt", request.job_id));

        fs::write(&output_path, format!("Dummy BLAST result\nJob ID: {}\n", request.job_id))
            .await
            .map_err(|_| BlastEngineError::ExecutionFailed(()))?;

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
        println!("LARGE engine executing job {}", request.job_id);

        let mut output_path = PathBuf::from("/home/lukesal/BioClick/NucloFlo/application_root/outputs");
        fs::create_dir_all(&output_path).await.unwrap();
        output_path.push(format!("large_result_{}.txt", request.job_id));

        fs::write(&output_path, format!("Dummy BLAST result\nJob ID: {}\n", request.job_id))
            .await
            .map_err(|_| BlastEngineError::ExecutionFailed(()))?;

        Ok(BlastResult {
            job_id: request.job_id,
            status: ResultStatus::Success,
            output: ResultOutput::FilePath(output_path),
        })
    }
}

// -----------------------------
// RESULTS
// -----------------------------
enum ResultStatus { Success, Failed }

#[derive(Debug)]
enum ResultOutput { FilePath(PathBuf) }

struct BlastResult {
    job_id: u64,
    status: ResultStatus,
    output: ResultOutput,
}

// -----------------------------
// SCHEDULER
// -----------------------------
struct Scheduler {
    queue: Vec<Job>,
    join_handle: Vec<tokio::task::JoinHandle<()>>,
    small_engine: Arc<dyn BlastEngine + Send + Sync>,
    large_engine: Arc<dyn BlastEngine + Send + Sync>,
}

impl Scheduler {
    fn new(jobs: Vec<Job>) -> Self {
        Self {
            queue: jobs,
            join_handle: vec![],
            small_engine: Arc::new(SmallDummyEngine),
            large_engine: Arc::new(RustProcessEngine),
        }
    }

    async fn run(mut self) {
        println!("Scheduler started");

        while let Some(job) = self.queue.pop() {
            println!("Dispatching job {}", job.id);

            let request = BlastExecutionRequest {
                job_id: job.id as u64,
                blast_type: BlastType::BlastN,
                input: BlastInput::FilePath(job.input_path.clone()),
                parameters: BlastParameters,
            };

            let engine = if job.id % 2 == 0 {
                Arc::clone(&self.large_engine)
            } else {
                Arc::clone(&self.small_engine)
            };

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
    let jobs = vec![
        Job {
            id: 2,
            name: "Test BLAST Job".to_string(),
            schedule: std::time::Duration::from_secs(0),
            state: JobState::Queued,
            input_path: PathBuf::from("/home/lukesal/BioClick/NucloFlo/application_root/inputs/job_1.fasta"),
        }
    ];

    let scheduler = Scheduler::new(jobs);
    scheduler.run().await;
}
