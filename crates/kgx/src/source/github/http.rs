/// HTTP-based GitHub API client using reqwest.
///
/// Future implementation: direct REST calls without shelling out to
/// `gh`. Requires `reqwest` and `tokio` dependencies.
pub struct HttpGitHubSource {
    _private: (),
}
