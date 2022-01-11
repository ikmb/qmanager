/**
 * Copyright (c) 2021 Jan Christian Kaessens
 * 
 * Permission is hereby granted, free of charge, to any person obtaining a copy
 * of this software and associated documentation files (the "Software"), to deal
 * in the Software without restriction, including without limitation the rights
 * to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
 * copies of the Software, and to permit persons to whom the Software is
 * furnished to do so, subject to the following conditions:
 * 
 * The above copyright notice and this permission notice shall be included in all
 * copies or substantial portions of the Software.
 * 
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 * IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 * FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
 * AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 * LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
 * OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
 * SOFTWARE.
 **/

use job_queue::{Job, QueueState};

/// A request by the client for the server. May be answered by
#[derive(Serialize, Deserialize, Debug)]
pub enum Request {
    /// Submit a job with the given command-line string (contains an appkey)
    /// Triggers a SubmitJob or Error response
    SubmitJob(String),

    /// Remove the job with the given ID with `Queued` or `Finished` job.
    /// Triggers a GetJob or an Error response
    RemoveJob(u64),

    /// Send SIGTERM to the job with the given ID (not PID)
    /// Triggers an Ok or Error response
    KillJob(u64),

    /// Request a list of queued jobs, including the currently running
    /// Triggers a GetJobs response
    GetQueuedJobs,

    /// Request a list of terminated jobs
    /// Triggers a GetJobs response
    GetFinishedJobs,

    /// Set the queue state
    /// Triggers a QueueState or Error response
    SetQueueState(QueueState),

    /// Request the current queue state
    /// Triggers a QueueState response
    GetQueueState,
}

/// A response from the server to the client
#[derive(Serialize, Deserialize, Debug)]
pub enum Response {
    /// The job has been submitted with the given ID
    SubmitJob(u64),

    /// A list of jobs
    GetJobs(Vec<Job>),

    /// A single job
    GetJob(Job),

    /// The request could not be handled (error message given)
    Error(String),

    /// The current queue state
    QueueState(QueueState),

    /// The request was successfully handled and no return value is given
    Ok,
}
