//! An Asset is a downloadable file for a release that should be downloaded

use crate::archive::ArchiveType;
use url::Url;

#[derive(Debug, Clone)]
pub enum Asset {
    Archive {
        file_name: String,
        link: Url,
        archive_type: Option<ArchiveType>,
    },
    Binary {
        file_name: String,
        link: Url,
    },
    CheckSum {
        file_name: String,
        link: Url,
        check_sum_type: Option<CheckSumType>,
    },
}

#[derive(Debug, Clone)]
pub enum CheckSumType {
    Md5,
    Sha1,
    Sha256,
}
