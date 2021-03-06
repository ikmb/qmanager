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

/**
 * clicommands.rs
 *
 * Contains various functions that create JSON requests out of CLI arguments,
 * parse the JSON response and provide a human-readable(-ish) console output.
 **/
use std::io::Result;

use serde_json;

use job_queue::*;
use protocol::{Request, Response};

/// Dumps a job vector to the console
fn print_jobs(header: &str, jobs: Vec<Job>) {
    println!("{}", header);
    for j in jobs {
        println!("{:?}", j);
    }
}

/// Sends a job submission request to the server and processes its result
///
/// # Arguments
///
/// * `client` - a HTTP(S) client object to be used for the connection
/// * `url` - the absolute URL that the client should use for posting the request
/// * `cmdline`- command line to be submitted for execution
/// * `dump_protocol` - a flag indicating that the JSON requests and responses are to be dumped
pub fn handle_submit(
    client: &reqwest::Client,
    url: reqwest::Url,
    cmdline: &str,
    dump_protocol: bool,
) -> Result<()> {
    // serialize the request into a JSON object
    let request_s = serde_json::to_string_pretty(&Request::SubmitJob(cmdline.to_string()))?;

    // write it to the server
    let mut response_req = client
        .post(url.clone())
        .body(request_s.clone())
        .send()
        .unwrap();
    if dump_protocol {
        println!("Sent: {} ", request_s);
    }

    // block for the server's response...
    let response_s = response_req.text().unwrap();
    if dump_protocol {
        println!("Received: {} ", response_s);
    }

    // ...and deserialize the response object from JSON
    let response = serde_json::from_str(&response_s)?;

    match response {
        Response::SubmitJob(id) => println!("Submitted as job #{}", id),
        Response::Error(s) => eprintln!("Could not submit job: {}", s),
        _ => panic!("Unexpected response: {:?}", response),
    }

    Ok(())
}

/// Requests a job to be removed from the queue
pub fn handle_remove(
    client: &reqwest::Client,
    url: reqwest::Url,
    jobid: u64,
    dump_protocol: bool,
) -> Result<Job> {
    let request_s = serde_json::to_string_pretty(&Request::RemoveJob(jobid))?;
    let mut response_req = client
        .post(url.clone())
        .body(request_s.clone())
        .send()
        .unwrap();
    if dump_protocol {
        println!("Sent: {} ", request_s);
    }

    let response_s = response_req.text().unwrap();
    if dump_protocol {
        println!("Received: {} ", response_s);
    }
    let response = serde_json::from_str(&response_s)?;

    match response {
        Response::GetJob(job) => Ok(job),
        Response::Error(s) => {
            eprintln!("Could not remove job: {}", s);
            Err(::std::io::Error::from(::std::io::ErrorKind::Other))
        }
        _ => panic!("Unexpected response: {:?}", response),
    }
}

/// Requests a running job to be terminated
pub fn handle_kill(
    client: &reqwest::Client,
    url: reqwest::Url,
    jobid: u64,
    dump_protocol: bool,
) -> Result<()> {
    let request_s = serde_json::to_string_pretty(&Request::KillJob(jobid))?;

    let mut response_req = client
        .post(url.clone())
        .body(request_s.clone())
        .send()
        .unwrap();
    if dump_protocol {
        println!("Sent: {} ", request_s);
    }

    let response_s = response_req.text().unwrap();
    if dump_protocol {
        println!("Received: {} ", response_s);
    }
    let response = serde_json::from_str(&response_s)?;

    match response {
        Response::Ok => Ok(()),
        Response::Error(s) => {
            eprintln!("Could not kill job: {}", s);
            Err(::std::io::Error::from(::std::io::ErrorKind::Other))
        }
        _ => panic!("Unexpected response: {:?}", response),
    }
}

/// Sets the current state of the queue.
/// Note that 'Stopped' cannot be set manually and will yield errors. You will have
/// to set 'Stopping' and let the queue itself to decide to go into 'Stopped' mode.
pub fn handle_set_queue_status(
    client: &reqwest::Client,
    url: reqwest::Url,
    new_state: QueueState,
    dump_protocol: bool,
) -> Result<()> {
    let request_s = serde_json::to_string_pretty(&Request::SetQueueState(new_state)).unwrap();
    let mut response_req = client
        .post(url.clone())
        .body(request_s.clone())
        .send()
        .unwrap();
    if dump_protocol {
        println!("Sent: {} ", request_s);
    }
    let response_s = response_req.text().unwrap();
    if dump_protocol {
        println!("Received: {} ", response_s);
    }
    let response = serde_json::from_str(&response_s)?;
    match response {
        Response::QueueState(s) => println!("Current queue status: {:?}", s),
        Response::Error(s) => eprintln!("Could not get queue status: {}", s),
        _ => panic!("Unexpected response: {:?}", response),
    };
    Ok(())
}

/// Removes jobs from the finished queue based on their age.
/// There is no direct JSON command to do this, so it requests
/// the job lists and removes them manually.
pub fn handle_cleanup(
    client: &reqwest::Client,
    url: reqwest::Url,
    max_age: humantime::Duration,
    dump_protocol: bool,
) -> Result<usize> {
    // Request list of finished jobs
    let request_s = serde_json::to_string_pretty(&Request::GetFinishedJobs).unwrap();
    let mut response_req = client
        .post(url.clone())
        .body(request_s.clone())
        .send()
        .unwrap();
    if dump_protocol {
        println!("Sent: {} ", request_s);
    }

    let response_s = response_req.text().unwrap();
    if dump_protocol {
        println!("Received: {} ", response_s);
    }

    // Get time stamp of oldest acceptable finished job
    let oldest_time = std::time::SystemTime::now() - *max_age;
    debug!(
        "It is now {:?}. Max age is {:?} and max job time stamp is {:?}.",
        std::time::SystemTime::now(),
        max_age,
        oldest_time
    );

    // Counter for removed jobs
    let mut jobs_removed = 0;

    // Find and remove expired jobs
    if let Response::GetJobs(jobs) = serde_json::from_str(&response_s)? {
        for job in &jobs {
            if let Some(t) = job.finished {
                if t < oldest_time {
                    match handle_remove(client, url.clone(), job.id, dump_protocol) {
                        Ok(_) => jobs_removed += 1,
                        Err(e) => println!("Could not remove job {}: {}", job.id, e),
                    }
                }
            }
        }
    }

    Ok(jobs_removed)
}

/// Requests the job queue state, the list of queued, running and finished jobs respectively
pub fn handle_queue_status(
    client: &reqwest::Client,
    url: reqwest::Url,
    dump_protocol: bool,
) -> Result<()> {
    // Request general queue state
    let mut request_s = serde_json::to_string_pretty(&Request::GetQueueState).unwrap();
    let mut response_req = client
        .post(url.clone())
        .body(request_s.clone())
        .send()
        .unwrap();
    if dump_protocol {
        println!("Sent: {} ", request_s);
    }
    let response_s = response_req.text().unwrap();
    if dump_protocol {
        println!("Received: {} ", response_s);
    }
    let response = serde_json::from_str(&response_s)?;
    match response {
        Response::QueueState(s) => println!("Current queue status: {:?}", s),
        Response::Error(s) => eprintln!("Could not get queue status: {}", s),
        _ => panic!("Unexpected response: {:?}", response),
    };

    // Request list of queued jobs (including running)
    request_s = serde_json::to_string_pretty(&Request::GetQueuedJobs).unwrap();
    response_req = client
        .post(url.clone())
        .body(request_s.clone())
        .send()
        .unwrap();
    if dump_protocol {
        println!("Sent: {} ", request_s);
    }

    let response_s = response_req.text().unwrap();
    if dump_protocol {
        println!("Received: {} ", response_s);
    }
    let mut response = serde_json::from_str(&response_s)?;

    match response {
        Response::GetJobs(jobs) => print_jobs("QUEUED JOBS", jobs),
        Response::Error(s) => {
            eprintln!("Could not get queued jobs: {}", s);
        }
        _ => {
            panic!("Unexpected response: {:?}", response);
        }
    }

    // Request list of finished jobs
    request_s = serde_json::to_string_pretty(&Request::GetFinishedJobs).unwrap();
    response_req = client
        .post(url.clone())
        .body(request_s.clone())
        .send()
        .unwrap();
    if dump_protocol {
        println!("Sent: {} ", request_s);
    }

    let response_s = response_req.text().unwrap();
    if dump_protocol {
        println!("Received: {} ", response_s);
    }
    response = serde_json::from_str(&response_s)?;
    match response {
        Response::GetJobs(jobs) => print_jobs("FINISHED JOBS", jobs),
        Response::Error(s) => {
            eprintln!("Could not get finished jobs: {}", s);
        }
        _ => {
            panic!("Unexpected response: {:?}", response);
        }
    }

    Ok(())
}
