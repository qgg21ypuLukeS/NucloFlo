// The code below is the precursor to the job scheduler application. It demonstrates basic Rust concepts such as functions, parameters, return values, and printing to the console.

use std::time::Duration;
//Struct to represent a Job in the scheduler
struct Job {
    id: u32,
    name: String,
    schedule: Duration,
    state: JobState,
}

struct Scheduler {
    queue: Vec<Job>,
    join_handle: Vec<tokio::task::JoinHandle<()>>,
}

use std::path::PathBuf;

struct BlastExecutionRequest {
    job_id: u64,
    blast_type: BlastType,
    input: BlastInput,
    parameters: BlastParameters,
}

struct BlastParameters;

//enum to represent different Job states and other Enum variants
enum JobState {
    Queued,
    Running,
    Completed,
}

enum BlastType {
    BlastN,
    BlastP,
    BlastX,
    TBlastN,
    TBlastX,
}

enum BlastInput {
    FilePath(PathBuf),
    RawBytes(Vec<u8>),
}

enum BlastEngineError {
    InvalidInput(String),
    UnsupportedFormat,
    DatabaseUnavailable,
    ExecutionFailed(String),
    Timeout,
    EngineCrashed,
}

//trait to define behavior for Blast Engines plus enums and structs used in the trait
#[async_trait::async_trait]
trait BlastEngine {
    async fn execute(
        &self,
        request: BlastExecutionRequest,
    ) -> Result<BlastResult, BlastEngineError>;
}


enum ResultStatus {
    Success,
    Failed,
}

enum ResultOutput {
    FilePath(PathBuf),
}

struct BlastResult {
    job_id: u64,
    status: ResultStatus,
    output: ResultOutput,
}

// Implementing methods for the Scheduler struct

impl Scheduler {

    // Constructor for Scheduler.
    // Takes ownership of a vector of Job structs and stores it internally.
    // After this call, the Scheduler is the sole owner of the job queue.
    fn new(jobs: Vec<Job>) -> Self {
    Scheduler {
        queue: jobs,
        join_handle: Vec::new(),
    }
}

    // The main scheduler loop.
    //
    // `async` allows us to use `.await` inside this function.
    // `mut self` means:
    //   - This method takes ownership of the Scheduler
    //   - The Scheduler is allowed to mutate its internal state
    //
    // We take ownership of `self` because once a scheduler starts running,
    // we don't expect to use it elsewhere — it "consumes" itself.
    async fn run(mut self) {

        // Log that the scheduler has started
        println!("Scheduler started");

        // Loop while there are still jobs in the queue.
        //
        // `self.queue.pop()`:
        //   - Removes the last Job from the vector
        //   - Returns `Some(job)` if a job exists
        //   - Returns `None` if the queue is empty
        //
        // `while let Some(job) = ...` keeps looping
        // until the queue is empty.
        //
        // IMPORTANT: `pop()` MOVES the Job out of the queue.
        // The Scheduler no longer owns this Job after this line.
        while let Some(job) = self.queue.pop() {

            // Log that the scheduler is dispatching this job
            println!("Dispatching job {}", job.id);

            // Spawn a new asynchronous task to run the job.
            //
            // `tokio::spawn` schedules the task to run concurrently
            // on the Tokio runtime.
            //
            // `async move` is CRITICAL:
            //   - `async` creates an asynchronous future
            //   - `move` transfers ownership of captured variables
            //     (in this case, `job`) into the task
            //
            // This ensures the job can run independently of the scheduler
            // without borrowing or lifetime issues.
            let mut job = job;
            job.state = JobState::Running;
            let join_handle = tokio::spawn(async move {
                // The worker task begins execution here
                println!("Job {} started", job.id);

                // Simulate work by sleeping asynchronously.
                //
                // This does NOT block the thread.
                // While this job is "working":
                //   - Other jobs can run
                //   - The scheduler can continue dispatching
                //   - The runtime remains responsive

                tokio::time::sleep(job.schedule).await;
                job.state = JobState::Completed;
                // When this task ends, the following happens automatically:
                
                println!("Job {} finished", job.id);

                // When this async block ends:
                //   - The task completes
                //   - The Job is dropped
                //   - All resources are cleaned up safely
            }); // Store the join_handle IMMEDIATELY
            self.join_handle.push(join_handle);
        }

        // This line executes once ALL jobs have been dispatched.
        //
        // IMPORTANT:
        //   - This does NOT mean jobs are finished
        //   - It only means the scheduler has handed them off
        //
        // Worker tasks may still be running at this point.
        println!("Scheduler finished dispatching jobs");
        for handle in self.join_handle {
            let _ = handle.await;
        }

        
        println!("All jobs completed");
    }
}


// Entry point of the application
#[tokio::main]
async fn main() {
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

    let scheduler = Scheduler::new(jobs);
    scheduler.run().await;
}