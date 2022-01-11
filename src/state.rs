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

use std::fs::{self, File};
use std::io::Result;
use std::path::PathBuf;

use job_queue::*;

/// Job IDs are incremented before they are assigned to jobs. Setting the
/// default last job id to zero makes the first submitted job to get
/// job id 1 assigned.
const DEFAULT_STATE_LAST_ID: u64 = 0;

/// Configuration of the program state object
pub struct State {
    state_file: PathBuf,
}

impl State {
    /// Configure the program state object
    fn load(p: PathBuf) -> State {
        State { state_file: p }
    }

    /// Configures the program state or uses defaults if the state file is not available
    pub fn from(p: PathBuf) -> State {
        if !p.exists() {
            warn!(
                "Cannot open state file {}. Using defaults.",
                p.to_str().unwrap()
            );
            State { state_file: p }
        } else {
            debug!("Loading program state from {}", p.to_str().unwrap());
            State::load(p)
        }
    }

    /// Loads the job queue from the configured program state
    pub fn load_queue(&self) -> JobQueue {
        let s = fs::read_to_string(&self.state_file).unwrap_or("".to_owned());
        let o = serde_json::from_str(&s);
        if let Ok(q) = o {
            q
        } else {
            warn!("Could not parse JobQueue from state file, returning default queue");
            JobQueue::new(DEFAULT_STATE_LAST_ID)
        }
    }

    /// Stores the given job queue into the configured program state
    pub fn save(&self, q: &JobQueue) -> Result<()> {
        let f = File::create(&self.state_file);
        if let Err(e) = f {
            error!(
                "Cannot create or open state file {}: {:?}",
                self.state_file.to_str().unwrap(),
                e
            );
            Err(e)
        } else {
            let mut f = f.unwrap();
            serde_json::to_writer_pretty(&mut f, q)?;
            debug!("State file {} updated.", self.state_file.to_str().unwrap());
            Ok(())
        }
    }
}
