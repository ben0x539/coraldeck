//include!(concat!(env!("OUT_DIR"), "/modimports.rs"));
pub use crate::error::Error;

use std::collections::BTreeMap;
use std::fmt;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use serde::{Deserialize, Deserializer, de::{MapAccess, Visitor, Error as _}};
use tokio::sync::mpsc::Receiver;

use async_trait::async_trait;

pub type DynModule = Box<dyn Module + Send>;
type DynModuleFuture<'a> = Pin<Box<dyn Future<Output = Result<DynModule, Error>>+'a>>;

automod::dir!("src/modules");

pub trait ModuleConfig: Send+Sync+fmt::Debug {
    fn name(&self) -> &'static str;
    fn build(&self) -> DynModuleFuture<'_>;
}

automod::with_mods! { "src/modules" MOD =>
    impl ModuleConfig for MOD::Config {
        fn name(&self) -> &'static str { stringify!(MOD) } // while we'e here...

        fn build(&self) -> DynModuleFuture<'_> {
            Box::pin(self.build())
        }
    }
}

type ConfigMap = BTreeMap<String, Arc<dyn ModuleConfig>>;

#[derive(Debug, Clone)]
pub struct Config {
    pub modules: ConfigMap,
}

// iterate by forwarding to BTreeMap
impl<'a> IntoIterator for &'a Config {
    type Item = <&'a ConfigMap as IntoIterator>::Item;
    type IntoIter = <&'a ConfigMap as IntoIterator>::IntoIter;
    fn into_iter(self) -> Self::IntoIter {
        (&self.modules).into_iter()
    }
}

impl<'de> Deserialize<'de> for Config {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where D: Deserializer<'de> {
        deserializer.deserialize_map(MyVisitor)
    }
}

struct MyVisitor;

impl<'de> Visitor<'de> for MyVisitor {
    type Value = Config;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "a map of module configurations")
    }

    fn visit_map<M>(self, mut access: M) -> Result<Self::Value, M::Error>
            where M: MapAccess<'de> {
        let mut config = Config { modules: BTreeMap::new() };
        while let Some(name) = access.next_key()? {
            automod::with_mods! { "src/modules" MOD =>
                if stringify!(MOD) == &name {
                    let value: MOD::Config = access.next_value()?;
                    let old = config.modules.insert(name, Arc::new(value));
                    if old.is_some() {
                        return Err(M::Error::custom(
                            format!("duplicate entry for {:?}",
                                stringify!(MOD))));
                    }
                    continue;
                }
            }
        }
        Ok(config)
    }
}

#[async_trait]
pub trait Module {
    fn name(&self) -> String;

    async fn trigger(&mut self, action: &str) -> Option<String>;

    async fn subscribe(&mut self) -> Receiver<(String, String)>;

    fn color(&self) -> (u8, u8, u8) {
        return (230, 100, 20);
    }
}
