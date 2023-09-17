use derive_builder::Builder;
use gitlab::api::common::NameOrId;
use gitlab::api::endpoint_prelude::*;

/// Query a single pipeline on a project.
#[derive(Debug, Builder)]
pub struct Archive<'a> {
    /// The project to query for pipeline.
    #[builder(setter(into))]
    project: NameOrId<'a>,
    #[allow(dead_code)]
    #[builder(setter(into))]
    sha: String,
}

impl<'a> Archive<'a> {
    /// Create a builder for the endpoint.
    pub fn builder() -> ArchiveBuilder<'a> {
        ArchiveBuilder::default()
    }
}

impl<'a> Endpoint for Archive<'a> {
    fn method(&self) -> Method {
        Method::GET
    }

    fn endpoint(&self) -> Cow<'static, str> {
        format!("projects/{}/repository/archive.tar.gz", self.project).into()
    }
}
