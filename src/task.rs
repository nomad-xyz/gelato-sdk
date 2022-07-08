use futures_timer::Delay;
use futures_util::ready;
use pin_project::pin_project;

use ethers_core::types::H256;
use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
    time::Duration,
};

use crate::{
    rpc::{self, Check, CheckOrDate, Execution},
    GelatoClient,
};

/// Gelato Task error
#[derive(Debug, thiserror::Error)]
pub enum TaskError {
    /// reqwest
    #[error("{0}")]
    ReqwestError(#[from] reqwest::Error),
    /// cancelled by backend
    #[error("Cancelled by backend")]
    Cancelled {
        /// Cancellation message
        message: Option<String>,
        /// Cancellation reason
        reason: Option<String>,
    },
    /// Reverted
    #[error("Execution Reverted")]
    Reverted {
        /// execution
        execution: Execution,
        /// last check
        last_check: Box<Check>,
    },
    /// BlackListed by backend
    #[error("BlackListed by backend")]
    BlackListed {
        /// Cancellation message
        message: Option<String>,
        /// Cancellation reason
        reason: Option<String>,
    },
    /// Not found
    #[error("Dropped by backend")]
    NotFound,
    /// Too many retries
    #[error("Backend returned too many error responses")]
    TooManyRetries,
}

// convenience
type PinBoxFut<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// A pending Gelato task
///
/// Retries are decremented when the server returns "undefined", indicating a
/// potentially recoverable backend error. Unrecoverable backend errors (e.g.
/// deserialization errors or HTTP 500-series statuses are not retried.
#[pin_project(project = TaskProj)]
pub struct GelatoTask<'a, P> {
    /// Task Id
    id: H256,
    /// Client
    client: &'a GelatoClient,
    /// task state
    state: TaskState<'a>,
    /// retries
    retries: usize,
    /// delay between requests
    delay: Duration,
    /// request payload
    payload: P,
}

const DEFAULT_RETRIES: usize = 5;
const DEFAULT_DELAY: u64 = 15;

enum TaskState<'a> {
    // Initial delay to ensure the GettingTx loop doesn't immediately fail
    Delaying(Pin<Box<Delay>>),
    // Waiting for API response
    Requesting(PinBoxFut<'a, Result<Option<rpc::TransactionStatus>, reqwest::Error>>),
    // future is over
    Complete,
}

impl<'a, P> std::fmt::Debug for GelatoTask<'a, P> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Task").field("id", &self.id).finish()
    }
}

impl<'a, P> GelatoTask<'a, P> {
    /// Instantiate a Task
    pub fn new(id: H256, client: &'a GelatoClient, payload: P) -> Self {
        let delay = Duration::from_secs(DEFAULT_DELAY);
        Self {
            id,
            client,
            state: TaskState::Delaying(Box::pin(Delay::new(delay))),
            retries: DEFAULT_RETRIES,
            delay,
            payload,
        }
    }

    /// Set the number of retries. Retries are decremented when the server
    /// returns "undefined", indicating a potentially recoverable backend error.
    /// Unrecoverable backend errors (e.g. deserialization errors or HTTP
    /// 500-series statuses are not retried
    #[must_use]
    pub fn retries(mut self, retries: usize) -> Self {
        self.retries = retries;
        self
    }

    /// Sets the polling delay (the time between poll attempts)
    #[must_use]
    pub fn polling_interval<T: Into<Duration>>(mut self, duration: T) -> Self {
        self.delay = duration.into();

        if matches!(self.state, TaskState::Delaying(_)) {
            self.state = TaskState::Delaying(Box::pin(Delay::new(self.delay)))
        }

        self
    }
}

macro_rules! make_request {
    ($cx:ident, $this:ident) => {
        *$this.state = TaskState::Requesting(Box::pin($this.client.get_task_status(*$this.id)));
        $cx.waker().wake_by_ref();
        return Poll::Pending
    };
}

macro_rules! complete {
    ($this:ident) => {
        *$this.state = TaskState::Complete;
    };
}

macro_rules! delay_it {
    ($cx:ident, $this:ident) => {
        *$this.state = TaskState::Delaying(Box::pin(Delay::new(*$this.delay)));
        $cx.waker().wake_by_ref();
        return Poll::Pending
    };
}

impl<'a, P> Future for GelatoTask<'a, P> {
    type Output = Result<Execution, TaskError>;

    #[tracing::instrument(skip(self), fields(task_id = ?self.id, retries_remaining = self.retries))]
    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        let this: TaskProj<_> = self.project();

        let status_fut = match this.state {
            TaskState::Delaying(delay) => {
                // if the delay isn't elapsed, shortcut out
                ready!(delay.as_mut().poll(cx));
                // change state to requesting
                make_request!(cx, this);
            }
            // unpack the future from the state
            TaskState::Requesting(fut) => fut,
            // should never happen. indicates faulty program
            TaskState::Complete => panic!("polled completed task"),
        };

        // if the server hasn't responded, shortcut out
        let status = ready!(status_fut.as_mut().poll(cx));

        // if the server returned undefined, decrement retries. according to
        // gelato docs this is a backend error
        if let Ok(None) = status {
            tracing::warn!("Undefined status while polling task");
            if *this.retries == 0 {
                complete!(this);
                return Poll::Ready(Err(TaskError::TooManyRetries));
            }
            *this.retries -= 1;
        }

        // if reqwest returns a deser or server error, end the future
        if let Err(e) = status {
            tracing::error!(error = %e, "Reqwest error in pending tx");
            complete!(this);
            return Poll::Ready(Err(TaskError::ReqwestError(e)));
        }

        let rpc::TransactionStatus {
            last_check,
            execution,
            ..
        } = status.expect("checked").expect("checked");

        // if there's no last check, we poll again later
        if last_check.is_none() {
            delay_it!(cx, this);
        }

        // if the last check is a timestamp, we poll again later
        let last_check = last_check.expect("checked");
        let last_check = match last_check {
            CheckOrDate::Date(_) => {
                delay_it!(cx, this);
            }
            CheckOrDate::Check(last_check) => last_check,
        };

        match last_check.task_state {
            // execution is succesful. return the execution object
            // we assume that there is NO VALID CASE where the API returns
            // `ExecSuccess` but `execution` is undefined
            rpc::TaskState::ExecSuccess => {
                complete!(this);
                Poll::Ready(Ok(execution.expect("exists if status is sucess")))
            }
            // execution occurred but reverted
            // return an error
            rpc::TaskState::ExecReverted => {
                complete!(this);
                Poll::Ready(Err(TaskError::Reverted {
                    execution: execution.expect("exists if status is reverted"),
                    last_check,
                }))
            }
            // request was blacklisted by backend
            rpc::TaskState::Blacklisted => {
                complete!(this);
                Poll::Ready(Err(TaskError::BlackListed {
                    message: last_check.message,
                    reason: last_check.reason,
                }))
            }
            // request was cancelled by backend
            rpc::TaskState::Cancelled => {
                complete!(this);
                Poll::Ready(Err(TaskError::Cancelled {
                    message: last_check.message,
                    reason: last_check.reason,
                }))
            }
            // request not found by backend
            rpc::TaskState::NotFound => {
                complete!(this);
                Poll::Ready(Err(TaskError::NotFound))
            }
            // anything else is a continuation
            _ => {
                delay_it!(cx, this);
            }
        }
    }
}
