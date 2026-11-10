//! The Android half of the backup document.
//!
//! A provider is reached through SAF, which has no synchronous API: a picker
//! waits for an activity result and a document is read through a content
//! resolver. Every provider call therefore runs on a worker thread and the UI
//! thread only merges what comes back, exactly like Jota's page does on Android.

use std::sync::mpsc::{Receiver, TryRecvError, channel};

use serde_json::Value;

/// What a provider answered about a document.
#[derive(Clone, Debug, Default)]
pub struct Meta {
    pub last_modified: i64,
    pub size: i64,
    pub name: String,
}

/// What the picker answered: the document's URI, its provider name and whether
/// it was just created.
#[derive(Clone, Debug, Default)]
pub struct Choice {
    pub uri: String,
    pub name: String,
    pub created: bool,
}

/// Why a document is being read.
#[derive(Clone, Copy)]
pub enum Purpose {
    Pass {
        trust: bool,
    },
    Reload,
    /// A file the user picked for the manual import, not the sync document.
    Manual,
}

#[derive(Clone, Copy)]
pub enum Pick {
    Folder,
    Document,
    Create,
}

/// One provider call, answered on the UI thread by `Saf::poll`.
pub enum Step {
    Infoed(Purpose, Result<Option<Meta>, String>),
    Staged(Purpose, Result<Meta, String>),
    Committed(Result<(), String>),
    Picked(Result<Choice, String>),
    Lost,
}

pub struct Saf {
    work: Option<Receiver<Step>>,
}

impl Saf {
    pub fn new() -> Self {
        Self { work: None }
    }

    pub fn busy(&self) -> bool {
        self.work.is_some()
    }

    /// The stat Jota's `settled` check makes: no stage, no merge.
    pub fn info(&mut self, uri: &str, purpose: Purpose) {
        let uri = uri.to_owned();
        self.run(move || Step::Infoed(purpose, worker::info(&uri)));
    }

    /// Copies the provider's document into `incoming.json` for the merge.
    pub fn stage(&mut self, uri: &str, purpose: Purpose) {
        let uri = uri.to_owned();
        self.run(move || Step::Staged(purpose, worker::stage(&uri)));
    }

    /// Writes `outgoing.json` back to the provider.
    pub fn commit(&mut self, uri: &str) {
        let uri = uri.to_owned();
        self.run(move || Step::Committed(worker::commit(&uri)));
    }

    pub fn pick(&mut self, pick: Pick, last: &str) {
        // The launch is posted to the activity's UI thread by Java: this thread
        // is GPUI's native event loop, and the framework refuses a picker
        // started from anything but the activity's own thread with
        // IllegalMonitorStateException. Everything after the launch stays on
        // the worker, where a provider read may block for as long as it likes.
        let last = last.to_owned();
        let started = worker::launch(pick, &last);
        self.run(move || Step::Picked(worker::pick(started)));
    }

    pub fn poll(&mut self) -> Option<Step> {
        let receiver = self.work.as_ref()?;
        match receiver.try_recv() {
            Ok(step) => {
                self.work = None;
                Some(step)
            }
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => {
                self.work = None;
                Some(Step::Lost)
            }
        }
    }

    fn run(&mut self, task: impl FnOnce() -> Step + Send + 'static) {
        let (sender, receiver) = channel();
        std::thread::spawn(move || {
            let _ = sender.send(task());
        });
        self.work = Some(receiver);
    }
}

mod worker {
    use super::*;

    pub fn info(uri: &str) -> Result<Option<Meta>, String> {
        let Some(answer) = call("info", Some(uri))? else {
            return Ok(None);
        };

        let value = parse(&answer)?;
        if value.get("lastModified").is_none() {
            return Ok(None);
        }

        Ok(Some(meta(&value)))
    }

    pub fn stage(uri: &str) -> Result<Meta, String> {
        let answer = call("stage", Some(uri))?.ok_or("Cannot read the backup file")?;
        Ok(meta(&parse(&answer)?))
    }

    pub fn commit(uri: &str) -> Result<(), String> {
        match call("commit", Some(uri))? {
            Some(error) if !error.is_empty() => Err(error),
            _ => Ok(()),
        }
    }

    /// Starts the picker through Java, which posts the launch to the activity's
    /// UI thread, and answers empty when the picker is on its way.
    pub fn launch(pick: Pick, last: &str) -> Result<Option<String>, String> {
        match pick {
            Pick::Folder => call("folder", None),
            Pick::Document => call("pick", Some(last)),
            Pick::Create => call("create", Some(last)),
        }
    }

    pub fn pick(started: Result<Option<String>, String>) -> Result<Choice, String> {
        let started = started?;

        // The picker opens now and answers when the user is done, so a non-empty
        // answer here is the launch itself failing.
        let answer = match started.filter(|answer| !answer.is_empty()) {
            Some(answer) => answer,
            None => wait()?,
        };

        let value = parse(&answer)?;
        if let Some(error) = value.get("error").and_then(Value::as_str) {
            return Err(error.to_owned());
        }

        let mut choice = Choice {
            uri: value
                .get("uri")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_owned(),
            name: value
                .get("displayName")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_owned(),
            created: value
                .get("created")
                .and_then(Value::as_bool)
                .unwrap_or(false),
        };

        // A URI that cannot be opened would be stored anyway and then fail on
        // every later pass, so an existing document is read once before it is
        // kept. A document the platform just created is empty and some
        // providers cannot open it yet; the first pass fills it.
        if !choice.uri.is_empty() && !choice.created {
            let meta = stage(&choice.uri)?;
            if !meta.name.is_empty() {
                choice.name = meta.name;
            }
        }

        Ok(choice)
    }

    /// Polls for the picker's answer. Blocking on it is not an option: the wait
    /// needs a monitor this platform does not give a native thread.
    fn wait() -> Result<String, String> {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(300);
        loop {
            if let Some(answer) = call("take", None)?
                && !answer.is_empty()
            {
                return Ok(answer);
            }

            if std::time::Instant::now() >= deadline {
                return Err("The picker did not answer".to_owned());
            }

            std::thread::sleep(std::time::Duration::from_millis(50));
        }
    }

    fn meta(value: &Value) -> Meta {
        Meta {
            last_modified: value
                .get("lastModified")
                .and_then(Value::as_i64)
                .unwrap_or_default(),
            size: value.get("size").and_then(Value::as_i64).unwrap_or(-1),
            name: value
                .get("displayName")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_owned(),
        }
    }

    fn parse(answer: &str) -> Result<Value, String> {
        serde_json::from_str(answer).map_err(|error| format!("sync answer: {error}"))
    }
}

const CLASS: &str = "com/jondo/jondo/SyncDocuments";

fn call(method: &str, argument: Option<&str>) -> Result<Option<String>, String> {
    match argument {
        Some(argument) => crate::shared::platform::android::java::call(
            CLASS,
            method,
            "(Landroid/app/Activity;Ljava/lang/String;)Ljava/lang/String;",
            &[argument],
        ),
        None => crate::shared::platform::android::java::call(
            CLASS,
            method,
            "(Landroid/app/Activity;)Ljava/lang/String;",
            &[],
        ),
    }
}
