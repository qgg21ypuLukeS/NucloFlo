//todo 
// add intelligent result deleting to save disk space
// add job prioritization



// Standard library imports
use std::time::Duration;
use std::path::PathBuf;
use std::sync::Arc;

// -----------------------------
// JOB MODEL
// -----------------------------

// Represents a unit of work submitted to the scheduler
struct Job {
    id: u32,                 // Unique identifier for the job
    name: String,            // Human-readable name (UI-facing)
    schedule: Duration,      // Placeholder for scheduling logic (used earlier)
    state: JobState,         // Current lifecycle state of the job
}

// Tracks where a job is in its lifecycle
enum JobState {
    Queued,
    Running,
    Completed,
}

// -----------------------------
// SCHEDULER
// -----------------------------

struct Scheduler {
    // Queue of pending jobs
    queue: Vec<Job>,

    // Handles to all spawned async tasks so we can await completion
    join_handle: Vec<tokio::task::JoinHandle<()>>,

    // Shared engine instance (trait object wrapped in Arc)
    //
    // Arc:
    //  - allows safe sharing across async tasks
    // dyn BlastEngine:
    //  - allows Python, Rust, Dummy engines interchangeably
    small_engine: Arc<dyn BlastEngine + Send + Sync>,
    large_engine: Arc<dyn BlastEngine + Send + Sync>,
}

// -----------------------------
// BLAST EXECUTION REQUEST
// -----------------------------

// This is the *exact payload* sent from the scheduler to the engine
struct BlastExecutionRequest {
    job_id: u64,             // Used to associate results back to jobs
    blast_type: BlastType,   // What kind of BLAST to run
    input: BlastInput,       // Where the sequence data comes from
    parameters: BlastParameters, // Placeholder for future BLAST flags
}

// Empty for now — grows later without breaking interfaces
struct BlastParameters;

// BLAST variants supported by the system
#[derive(Debug)]
enum BlastType {
    BlastN,
    BlastP,
    BlastX,
    TBlastN,
    TBlastX,
}

// Input source for BLAST
enum BlastInput {
    FilePath(PathBuf),   // Most common case (UI selects file)
    RawBytes(Vec<u8>),   // Future: pasted sequences
}

// -----------------------------
// ENGINE ERROR MODEL
// -----------------------------

// Structured errors returned by engines
#[derive(Debug)]
enum BlastEngineError {
    InvalidInput(String),
    UnsupportedFormat,
    DatabaseUnavailable,
    ExecutionFailed(String),
    Timeout,
    EngineCrashed,
}

// -----------------------------
// ENGINE TRAIT (CONTRACT)
// -----------------------------

// This defines what *every* BLAST engine must do
#[async_trait::async_trait]
trait BlastEngine {
    async fn execute(
        &self,
        request: BlastExecutionRequest,
    ) -> Result<BlastResult, BlastEngineError>;
    fn name(&self) -> &'static str;
}

// -----------------------------
// DUMMY ENGINE (TEST ENGINE)
// -----------------------------

// Stateless dummy engine used to validate architecture
struct SmallDummyEngine;
struct LargeDummyEngine;

#[async_trait::async_trait]
impl BlastEngine for SmallDummyEngine {
    fn name(&self) -> &'static str {
        "SmallDummyEngine"
    }

    async fn execute(
        &self,
        request: BlastExecutionRequest,
    ) -> Result<BlastResult, BlastEngineError> {
        println!("SMALL engine executing job {}", request.job_id);
        tokio::time::sleep(Duration::from_secs(1)).await;

        Ok(BlastResult {
            job_id: request.job_id,
            status: ResultStatus::Success,
            output: ResultOutput::FilePath(
                PathBuf::from(format!("small_result_{}.txt", request.job_id))
            ),
        })
    }
}

#[async_trait::async_trait]
impl BlastEngine for LargeDummyEngine {
    fn name(&self) -> &'static str {
        "LargeDummyEngine"
    }

    async fn execute(
        &self,
        request: BlastExecutionRequest,
    ) -> Result<BlastResult, BlastEngineError> {
        println!("LARGE engine executing job {}", request.job_id);
        tokio::time::sleep(Duration::from_secs(3)).await;

        Ok(BlastResult {
            job_id: request.job_id,
            status: ResultStatus::Success,
            output: ResultOutput::FilePath(
                PathBuf::from(format!("large_result_{}.txt", request.job_id))
            ),
        })
    }
}


// -----------------------------
// RESULT MODEL
// -----------------------------

enum ResultStatus {
    Success,
    Failed,
}

// Result points to an output artifact, not raw data
#[derive(Debug)]
enum ResultOutput {
    FilePath(PathBuf),
}

struct BlastResult {
    job_id: u64,
    status: ResultStatus,
    output: ResultOutput,
}

// -----------------------------
// SCHEDULER IMPLEMENTATION
// -----------------------------

impl Scheduler {

    // Constructor — creates scheduler with a dummy engine
    fn new(jobs: Vec<Job>) -> Self {
    Scheduler {
        queue: jobs,
        join_handle: Vec::new(),
        small_engine: Arc::new(SmallDummyEngine),
        large_engine: Arc::new(LargeDummyEngine),
        }
    }

    // Main async scheduler loop
    async fn run(mut self) {

        println!("Scheduler started");

        // Pop jobs until queue is empty
        while let Some(job) = self.queue.pop() {

            println!("Dispatching job {}", job.id);

            // Convert Job -> BlastExecutionRequest
            let request = BlastExecutionRequest {
                job_id: job.id as u64,
                blast_type: BlastType::BlastN, // hardcoded for now
                input: BlastInput::FilePath(PathBuf::from("dummy.fasta")),
                parameters: BlastParameters,
            };

            // Clone Arc so this task owns its engine reference
            let input_size_bytes = if job.id % 2 == 0 {
                    2_000_000  // even jobs → LARGE
                } else {
                    100_000    // odd jobs → SMALL
                }; // placeholder

            let engine = if input_size_bytes < 1_000_000 {
                Arc::clone(&self.small_engine)
            } else {
                Arc::clone(&self.large_engine)
            };

            println!(
                "Job {} assigned to engine: {}",
                job.id,
                engine.name()
            );
            // Spawn async task for this job
            let handle = tokio::spawn(async move {

                match engine.execute(request).await {
                    Ok(result) => {
                        println!(
                            "Job {} completed successfully. Output: {:?}",
                            result.job_id, result.output
                        );
                    }
                    Err(err) => {
                        println!("Job {} failed: {:?}", job.id, err);
                    }
                }
            });

            // Store handle so we can await later
            self.join_handle.push(handle);
        }

        println!("Scheduler finished dispatching jobs");

        // Wait for all jobs to finish
        for handle in self.join_handle {
            let _ = handle.await;
        }

        println!("All jobs completed");
    }
}

// -----------------------------
// APPLICATION ENTRY POINT
// -----------------------------

#[tokio::main]
async fn main() {

    // Sample jobs (UI will create these later)
    let jobs = vec![
        Job {
            id: 1,
            name: "Job A".to_string(),
            schedule: Duration::from_secs(2),
            state: JobState::Queued,
        },
        Job {
            id: 2,
            name: "Job B".to_string(),
            schedule: Duration::from_secs(1),
            state: JobState::Queued,
        },
        Job {
            id: 3,
            name: "Job C".to_string(),
            schedule: Duration::from_secs(3),
            state: JobState::Queued,
        },
    ];

    // Create scheduler and run it
    let scheduler = Scheduler::new(jobs);
    scheduler.run().await;
}
