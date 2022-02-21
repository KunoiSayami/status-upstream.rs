/*
 ** Copyright (C) 2021-2022 KunoiSayami
 **
 ** This program is free software: you can redistribute it and/or modify
 ** it under the terms of the GNU Affero General Public License as published by
 ** the Free Software Foundation, either version 3 of the License, or
 ** any later version.
 **
 ** This program is distributed in the hope that it will be useful,
 ** but WITHOUT ANY WARRANTY; without even the implied warranty of
 ** MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
 ** GNU Affero General Public License for more details.
 **
 ** You should have received a copy of the GNU Affero General Public License
 ** along with this program. If not, see <https://www.gnu.org/licenses/>.
 */
use anyhow::anyhow;
use serde_derive::{Deserialize, Serialize};

type VersionType = u64;
const DEADLINE: u64 = 600;

pub fn get_current_timestamp() -> u64 {
    let start = std::time::SystemTime::now();
    let since_the_epoch = start
        .duration_since(std::time::UNIX_EPOCH)
        .expect("Time went backwards");
    since_the_epoch.as_secs()
}

mod errors {
    use super::{PreReadCacheData, VersionType, CURRENT_VERSION};
    use std::error::Error;
    use std::fmt::{Debug, Display, Formatter};

    pub struct VersionNotMatchError {
        current_version: VersionType,
    }

    impl From<&PreReadCacheData> for VersionNotMatchError {
        fn from(data: &PreReadCacheData) -> Self {
            Self {
                current_version: data.version(),
            }
        }
    }

    impl Debug for VersionNotMatchError {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            write!(
                f,
                "VersionNotMatchError {{ current version: {}, except version: {} }}",
                self.current_version, CURRENT_VERSION
            )
        }
    }

    impl Display for VersionNotMatchError {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            write!(
                f,
                "Version not match, except {} but {} found",
                CURRENT_VERSION, self.current_version
            )
        }
    }

    impl Error for VersionNotMatchError {}

    #[derive(Debug, Clone, Default)]
    pub struct OutdatedError {}

    impl Display for OutdatedError {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            write!(f, "OutdatedError")
        }
    }

    impl Error for OutdatedError {}

    impl OutdatedError {
        pub fn new() -> Self {
            Self {}
        }
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct PreReadCacheData {
    version: VersionType,
    timestamp: u64,
}

impl PreReadCacheData {
    pub fn version(&self) -> VersionType {
        self.version
    }
    pub fn timestamp(&self) -> u64 {
        self.timestamp
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CacheData {
    version: VersionType,
    timestamp: u64,
    data: Vec<ComponentCache>,
}

impl CacheData {
    pub fn data(&self) -> &Vec<ComponentCache> {
        &self.data
    }
    /*pub fn version(&self) -> VersionType {
        self.version
    }
    pub fn timestamp(&self) -> u64 {
        self.timestamp
    }*/

    pub fn from_configure(config: &Configure) -> Self {
        let v = config
            .services()
            .clone()
            .into_iter()
            .map(|x| ComponentCache::from(&x))
            .collect::<Vec<ComponentCache>>();
        Self {
            version: CURRENT_VERSION,
            timestamp: get_current_timestamp(),
            data: v,
        }
    }
}

mod v2 {
    use super::{Deserialize, VersionType};
    use crate::connlib::ServiceWrapper;
    use serde_derive::Serialize;

    pub const VERSION: VersionType = 2;

    #[derive(Deserialize, Serialize, Debug, Clone)]
    pub struct ComponentCache {
        id: String,
        last_status: String,
    }

    impl ComponentCache {
        pub fn id(&self) -> &str {
            &self.id
        }
        pub fn last_status(&self) -> &str {
            &self.last_status
        }
    }

    impl From<&ServiceWrapper> for ComponentCache {
        fn from(service: &ServiceWrapper) -> Self {
            Self {
                id: service.report_uuid().to_string(),
                last_status: service.last_status().to_string(),
            }
        }
    }
}

use crate::Configure;
pub use current::ComponentCache;
pub use current::VERSION as CURRENT_VERSION;
pub use errors::OutdatedError;
pub use errors::VersionNotMatchError;
use v2 as current;

pub async fn read_cache(path: &str) -> anyhow::Result<CacheData> {
    let content = tokio::fs::read_to_string(&path).await?;
    let result = serde_json::from_str::<PreReadCacheData>(content.as_str());
    if let Err(ref e) = result {
        return Err(anyhow!("Got error while decode {:?}, {:?}", path, e));
    }
    let result = result.unwrap();
    if !result.version().eq(&CURRENT_VERSION) {
        return Err(anyhow::Error::from(VersionNotMatchError::from(&result)));
    }
    if get_current_timestamp() - result.timestamp() > DEADLINE {
        return Err(anyhow::Error::from(OutdatedError::new()));
    }
    let result = serde_json::from_str(content.as_str());
    if let Err(ref e) = result {
        return Err(anyhow!(
            "Got error while decode full data {:?}, {:?}",
            path,
            e
        ));
    }
    Ok(result.unwrap())
}
