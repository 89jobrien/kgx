/// Local git repository source.
///
/// Future implementation: walk a cloned git repo on disk, parse
/// source files, README, and metadata into `ParsedDocument`s.
pub struct LocalGitSource {
    _private: (),
}
