// use crate::release::Release;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tool<'t> {
    pub name: &'t str,
    // pub current_release: Option<&'t Release>,
    // pub installed_releases: Option<Vec<&'t Release>>,
}
