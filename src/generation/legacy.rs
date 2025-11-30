use serde::{Deserialize, Serialize};

#[derive(PartialEq, Serialize, Deserialize, Debug)]
#[serde(deny_unknown_fields, default)]
pub struct Packages {
    pub pkgs: Vec<String>,
}

impl Default for Packages {
    fn default() -> Self {
        Self { pkgs: Vec::new() }
    }
}

#[derive(PartialEq, Serialize, Deserialize, Debug)]
#[serde(deny_unknown_fields, default)]
pub struct Script {
    pub name: String,
    pub pre: String,
    pub run: String,
}

impl Default for Script {
    fn default() -> Self {
        Self {
            name: String::new(),
            pre: String::new(),
            run: String::new(),
        }
    }
}

#[derive(PartialEq, Serialize, Deserialize, Debug)]
#[serde(deny_unknown_fields, default)]
pub struct Service {
    pub name: String,
    pub enabled: bool,
}

impl Default for Service {
    fn default() -> Self {
        Self {
            name: String::new(),
            enabled: false,
        }
    }
}

#[derive(PartialEq, Serialize, Deserialize, Debug)]
#[serde(deny_unknown_fields, default)]
pub struct Hook {
    pub pre: String,
    pub post: String,
}

impl Default for Hook {
    fn default() -> Self {
        Self {
            pre: String::new(),
            post: String::new(),
        }
    }
}

pub mod legacy_1 {
    use serde::{Deserialize, Serialize};

    #[derive(PartialEq, Serialize, Deserialize, Debug, Default)]
    #[serde(deny_unknown_fields, default)]
    pub struct Generation {
        pub imports: Vec<String>,
        pub pkgs: Vec<String>,
        pub flatpaks: Vec<String>,
        pub crates: Vec<String>,
    }

    impl crate::generation::Migrate<crate::generation::Generation> for Generation {
        fn migrate(self) -> crate::generation::Generation {
            use std::collections::HashMap;
            
            let mut managers: HashMap<String, crate::generation::Items> = HashMap::new();
            managers.insert("system".to_string(), crate::generation::Items { items: self.pkgs });
            managers.insert("flatpak".to_string(), crate::generation::Items { items: self.flatpaks });
            managers.insert("cargo".to_string(), crate::generation::Items { items: self.crates });

            crate::generation::Generation {
                imports: self.imports,
                managers,
            }
        }
    }
}

pub mod legacy_2 {
    use serde::{Deserialize, Serialize};

    #[derive(PartialEq, Serialize, Deserialize, Debug)]
    #[serde(deny_unknown_fields, default)]
    pub struct Generation {
        pub imports: Vec<String>,
        pub packages: super::Packages,
        pub flatpak: super::Packages,
        pub crates: super::Packages,
        pub groups: Vec<String>,
        pub scripts: Vec<super::Script>,
        pub services: Vec<super::Service>,
        pub hooks: Vec<super::Hook>,
    }

    impl Default for Generation {
        fn default() -> Self {
            Self {
                imports: Vec::new(),
                packages: super::Packages { pkgs: Vec::new() },
                flatpak: super::Packages { pkgs: Vec::new() },
                crates: super::Packages { pkgs: Vec::new() },
                groups: Vec::new(),
                scripts: Vec::new(),
                services: Vec::new(),
                hooks: Vec::new(),
            }
        }
    }

    impl crate::generation::Migrate<crate::generation::Generation> for Generation {
        fn migrate(self) -> crate::generation::Generation {
            use std::collections::HashMap;
            
            let mut managers: HashMap<String, crate::generation::Items> = HashMap::new();
            managers.insert("system".to_string(), crate::generation::Items { items: self.packages.pkgs });
            managers.insert("flatpak".to_string(), crate::generation::Items { items: self.flatpak.pkgs });
            managers.insert("cargo".to_string(), crate::generation::Items { items: self.crates.pkgs });

            crate::generation::Generation {
                imports: self.imports,
                managers,
            }
        }
    }
}