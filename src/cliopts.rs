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


use std::io::{ErrorKind, Result};
use std::path::PathBuf;

use config::Config;
use std::collections::HashMap;
use structopt::StructOpt;

/// Default port for use with both daemon and client code
pub const DEFAULT_PORT: u16 = 1337;

/// Default server hostname for the client to connect to. Can be any
/// resolvable address.
pub const DEFAULT_HOST: &str = "localhost";

/// Default program state file to be used by the daemon.
pub const DEFAULT_STATE: &str = "/var/lib/qmanager/qmanager.state";

#[derive(Debug, StructOpt)]
#[structopt(name=crate_name!(), version=crate_version!(), author=crate_authors!(), about=crate_description!())]
pub struct Opt {
    /// Set CA certificate
    #[structopt(long, parse(from_os_str))]
    pub ca: Option<PathBuf>,

    /// Use plain TCP instead of SSL/TLS
    #[structopt(long)]
    pub insecure: bool,

    /// For clients, the host name to connect to. For servers ignored (default: localhost)
    #[structopt(long, default_value = "")]
    pub host: String,

    /// For clients, the port to connect to. For servers, the port to listen on (default: 1337)
    #[structopt(long, default_value = "0")]
    pub port: u16,

    #[structopt(long)]
    /// Dump client requests and responses to stdout
    pub dump_json: bool,

    #[structopt(long, default_value = "")]
    /// The log level (default: Info, possible: Error, Warn, Info, Debug)
    pub loglevel: String,

    #[structopt(long, parse(from_os_str), default_value = "/etc/qmanager.conf")]
    /// Path to configuration file
    pub config: PathBuf,

    #[structopt(skip)]
    /// Application keys
    pub appkeys: HashMap<String, PathBuf>,

    #[structopt(subcommand)]
    pub cmd: OptCommand,

    #[structopt(long, parse(from_os_str))]
    pub state_file: Option<PathBuf>,
}

#[derive(Debug, StructOpt)]
pub enum OptCommand {
    /// Starts the qmanager daemon
    Daemon {
        /// Stays in foreground, does not detach. Pidfile argument is ignored
        #[structopt(long)]
        foreground: bool,

        /// Certificate file for SSL/TLS operation
        #[structopt(long, parse(from_os_str))]
        cert: Option<PathBuf>,

        /// Key for SSL/TLS certificate
        #[structopt(long, parse(from_os_str))]
        key: Option<PathBuf>,

        /// PID file location
        #[structopt(long, parse(from_os_str))]
        pidfile: Option<PathBuf>,

        /// Notify URL
        #[structopt(long)]
        notify_url: Option<String>,
    },

    /// Requests the queue to be stopped
    Stop {},

    /// Requests queue operations to be resumed
    Start {},

    /// Requests queue status
    Status {},

    /// Submits a job to the queue
    Submit {
        #[structopt(name = "CMDLINE", parse(from_str))]
        cmdline: String,
    },

    /// Removes a finished job from the queue
    Remove {
        /// Job ID to remove from the 'finished' queue
        #[structopt(long)]
        job_id: u64,
    },

    /// Asks a running job to terminate
    Kill {
        /// Job ID to terminate
        #[structopt(long)]
        job_id: u64,
    },

    /// Removes finished jobs from the queue based on timestamps
    Cleanup {
        /// Maximum age of a job's 'finished' timestamp, i.e. '8 days 3 seconds'
        #[structopt(long)]
        max_age: humantime::Duration,
    },
}

impl Opt {
    /// Merges a config file with the command-line options.
    /// CLI options generally take precedence over options imported from
    /// the config file.
    pub fn merge_config(&mut self, conf: Config) {
        // if --insecure is not present on the CL, check config for CA.
        // Certs and keys will be checked when destructuring the self.cmd.
        if !self.insecure {
            if self.ca.is_none() {
                self.ca = conf.get_str("ca").ok().map(PathBuf::from);
            }
            self.insecure |= conf.get_bool("insecure").unwrap_or(false);
        }

        // TCP port for connecting (client) or listening (daemon)
        self.port = if self.port == 0 {
            conf.get_int("port")
                .unwrap_or_else(|_| i64::from(DEFAULT_PORT)) as u16
        } else {
            self.port
        };

        // Host name for client connection (client only)
        if self.host.is_empty() {
            self.host = conf
                .get_str("host")
                .unwrap_or_else(|_| DEFAULT_HOST.to_string());
        }

        // "dump-json" debug flag
        if !self.dump_json {
            self.dump_json = conf.get_bool("dump-json").unwrap_or(false);
        }

        // state file location (daemon only)
        if self.state_file.is_none() {
            self.state_file = Some(PathBuf::from(
                conf.get_str("state-file")
                    .unwrap_or(DEFAULT_STATE.to_string()),
            ));
        }

        // daemon-specific opts
        if let OptCommand::Daemon {
            ref mut cert,
            ref mut key,
            ref mut pidfile,
            ref mut notify_url,
            ..
        } = &mut self.cmd
        {
            if cert.is_none() {
                *cert = conf.get_str("cert").ok().map(PathBuf::from);
            }

            if key.is_none() {
                *key = conf.get_str("key").ok().map(PathBuf::from);
            }

            if pidfile.is_none() {
                *pidfile = conf.get_str("pidfile").ok().map(PathBuf::from);
            }

            if notify_url.is_none() {
                *notify_url = conf.get_str("notify-url").ok();
            }
        }

        let appkeys = conf
            .get_table("appkeys")
            .expect("Could not load appkeys from config file!");
        for (k, v) in appkeys {
            self.appkeys.insert(k, PathBuf::from(v.into_str().unwrap()));
        }

        // set log level
        if self.loglevel.is_empty() {
            self.loglevel = conf
                .get_str("loglevel")
                .unwrap_or_else(|_| "Info".to_owned());
        }
    }

    /// Checks general validity of the option occurrences
    pub fn verify(&self) -> Result<()> {
        // it does not make sense to specify --insecure AND any SSL-related stuff
        if self.insecure {
            if self.ca.is_some() {
                eprintln!("You cannot specify both --insecure and --ca!");
                return Err(std::io::Error::from(ErrorKind::InvalidInput));
            }
            if let OptCommand::Daemon { cert, key, .. } = &self.cmd {
                if cert.is_some() || key.is_some() {
                    eprintln!(
                        "You cannot specify --insecure in combination with --cert and --key!"
                    );
                    return Err(std::io::Error::from(ErrorKind::InvalidInput));
                }
            }
        } else {
            if self.ca.is_none() {
                eprintln!("You need to specify either --ca or --insecure!");
                return Err(std::io::Error::from(ErrorKind::InvalidInput));
            }
            if let OptCommand::Daemon { cert, key, .. } = &self.cmd {
                if cert.is_none() || key.is_none() {
                    eprintln!(
                        "You cannot use daemon mode without specifying both --cert and --key!"
                    );
                    return Err(std::io::Error::from(ErrorKind::InvalidInput));
                }
            }
        }

        // PathBuf validity is checked when the path is actually opened later, no need to check here.
        Ok(())
    }
}
