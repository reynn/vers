//! An Asset is a downloadable file for a release.

use crate::archive::ArchiveType;
use url::Url;

#[derive(Debug, Clone)]
/// TODO: write docs
pub enum Asset {
    /// TODO: write docs
    Archive {
        /// TODO: write docs
        file_name: String,
        /// TODO: write docs
        link: Url,
        /// TODO: write docs
        archive_type: Option<ArchiveType>,
    },
    /// TODO: write docs
    Binary {
        /// TODO: write docs
        file_name: String,
        /// TODO: write docs
        link: Url,
    },
    /// TODO: write docs
    CheckSum {
        /// TODO: write docs
        file_name: String,
        /// TODO: write docs
        link: Url,
        /// TODO: write docs
        check_sum_type: Option<CheckSumType>,
    },
}

#[derive(Debug, Clone)]
/// TODO: write docs
pub enum CheckSumType {
    /// TODO: write docs
    Md5,
    /// TODO: write docs
    Sha1,
    /// TODO: write docs
    Sha256,
}
