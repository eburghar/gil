use std::fmt::{self, Display, Formatter};

use chrono::{DateTime, Utc};
use serde::Deserialize;

use super::ObjectId;

#[derive(Deserialize, Debug, Clone)]
pub struct PipelineId(u64);

impl PipelineId {
	/// The value of the id.
	pub const fn value(&self) -> u64 {
		self.0
	}
}

impl Display for PipelineId {
	fn fmt(&self, f: &mut Formatter) -> fmt::Result {
		write!(f, "{}", self.0)
	}
}

/// States for commit statuses.
#[derive(Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusState {
	/// The check was created.
	#[serde(rename = "created")]
	Created,
	/// The check is waiting for some other resource.
	#[serde(rename = "waiting_for_resource")]
	WaitingForResource,
	/// The check is currently being prepared.
	#[serde(rename = "preparing")]
	Preparing,
	/// The check is queued.
	#[serde(rename = "pending")]
	Pending,
	/// The check is currently running.
	#[serde(rename = "running")]
	Running,
	/// The check succeeded.
	#[serde(rename = "success")]
	Success,
	/// The check failed.
	#[serde(rename = "failed")]
	Failed,
	/// The check was canceled.
	#[serde(rename = "canceled")]
	Canceled,
	/// The check was skipped.
	#[serde(rename = "skipped")]
	Skipped,
	/// The check is waiting for manual action.
	#[serde(rename = "manual")]
	Manual,
	/// The check is scheduled to run at some point in time.
	#[serde(rename = "scheduled")]
	Scheduled,
}

#[derive(Deserialize, Debug, Clone)]
pub struct JobId(u64);

impl JobId {
	/// The value of the id.
	pub const fn value(&self) -> u64 {
		self.0
	}
}

impl Display for JobId {
	fn fmt(&self, f: &mut Formatter) -> fmt::Result {
		write!(f, "{}", self.0)
	}
}

/// Information about a job in Gitlab CI.
#[derive(Deserialize, Debug, Clone)]
pub struct Job {
	/// The ID of the job.
	pub id: JobId,
	/// The name of the job.
	pub name: String,
	/// The status of the job.
	pub status: StatusState,
	pub stage: String,
	/// The URL to the job page.
	pub web_url: String,
	/// When the job was created or marked as pending.
	pub created_at: DateTime<Utc>,
	/// When the job was started.
	pub started_at: Option<DateTime<Utc>>,
	/// When the job completed.
	pub finished_at: Option<DateTime<Utc>>,
}

/// More information about a pipeline in Gitlab CI.
#[derive(Deserialize, Debug, Clone)]
pub struct Pipeline {
	/// The ID of the pipeline.
	pub id: PipelineId,
	/// The name of the reference that was tested.
	#[serde(rename = "ref")]
	pub ref_: Option<String>,
	/// The status of the pipeline.
	pub status: StatusState,
	/// The object ID that was tested.
	pub sha: ObjectId,
	/// When the pipeline was created.
	pub created_at: Option<DateTime<Utc>>,
	/// The URL to the pipeline page.
	pub web_url: String,
}
