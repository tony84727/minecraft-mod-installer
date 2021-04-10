use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct FormatSpecific {
    #[serde(rename = "ignoreProject")]
    pub ignore_project: Vec<i32>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct Install {
    #[serde(rename = "modpackUrl")]
    pub modpack_url: String,
    #[serde(rename = "formatSpecific")]
    pub format_specific: FormatSpecific,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct ServerSetupConfig {
    pub install: Install,
}

#[cfg(test)]
mod tests {
    use crate::config::{FormatSpecific, Install, ServerSetupConfig};

    #[test]
    fn test_deserialize() {
        let config_file = r#"install:
  # version of minecraft, needs the exact version
  mcVersion: 1.16.5
  modpackUrl: https://media.forgecdn.net/files/3249/360/All+the+Mods+6-1.5.6.zip

  # This is used to specify in which format the modpack is distributed, the server launcher has to handle each individually if their format differs
  # current supported formats:
  # - curseforge or curse
  # - curseid
  # - zip or zipfile
  modpackFormat: curse

  formatSpecific:
    ignoreProject:
      - 263420
      - 317780
      - 232131
      - 231275
  baseInstallPath: ~
  installForge: yes
"#;
        let config = serde_yaml::from_str(config_file).unwrap();
        let expected_config = ServerSetupConfig {
            install: Install {
                modpack_url: "https://media.forgecdn.net/files/3249/360/All+the+Mods+6-1.5.6.zip"
                    .to_string(),
                format_specific: FormatSpecific {
                    ignore_project: vec![263420, 317780, 232131, 231275],
                },
            },
        };
        assert_eq!(expected_config, config);
    }
}
