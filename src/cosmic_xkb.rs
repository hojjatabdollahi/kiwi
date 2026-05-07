//! Minimal access to COSMIC Comp's XKB configuration.

use cosmic_config::cosmic_config_derive::CosmicConfigEntry;
use cosmic_config::CosmicConfigEntry;
use serde::{Deserialize, Serialize};

pub const COSMIC_COMP_APP_ID: &str = "com.system76.CosmicComp";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct XkbConfig {
    pub rules: String,
    pub model: String,
    pub layout: String,
    pub variant: String,
    pub options: Option<String>,
}

impl Default for XkbConfig {
    fn default() -> Self {
        Self {
            rules: String::new(),
            model: String::new(),
            layout: String::new(),
            variant: String::new(),
            options: None,
        }
    }
}

impl XkbConfig {
    /// COSMIC's input-source applet keeps the active source first.
    pub fn active_source(&self) -> XkbSource<'_> {
        let layout = self.layout.split(',').next().unwrap_or_default();
        let variant = self.variant.split(',').next().unwrap_or_default();

        XkbSource {
            rules: self.rules.as_str(),
            model: self.model.as_str(),
            layout,
            variant,
            options: self.options.as_deref(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct XkbSource<'a> {
    pub rules: &'a str,
    pub model: &'a str,
    pub layout: &'a str,
    pub variant: &'a str,
    pub options: Option<&'a str>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, CosmicConfigEntry)]
#[version = 1]
pub struct CosmicCompConfig {
    pub xkb_config: XkbConfig,
}

pub fn load_current_config() -> XkbConfig {
    let Ok(handler) = cosmic_config::Config::new(COSMIC_COMP_APP_ID, CosmicCompConfig::VERSION)
    else {
        log::warn!("Failed to create COSMIC Comp config handler; using default XKB config");
        return XkbConfig::default();
    };

    match CosmicCompConfig::get_entry(&handler) {
        Ok(config) => config.xkb_config,
        Err((errors, config)) => {
            log::warn!("Errors loading COSMIC Comp XKB config: {:?}", errors);
            config.xkb_config
        }
    }
}

#[cfg(test)]
mod tests {
    use super::XkbConfig;

    #[test]
    fn active_source_uses_first_layout_and_variant() {
        let config = XkbConfig {
            layout: "us,ir,us".to_string(),
            variant: "dvorak,,".to_string(),
            ..XkbConfig::default()
        };

        let source = config.active_source();
        assert_eq!(source.layout, "us");
        assert_eq!(source.variant, "dvorak");
    }

    #[test]
    fn active_source_handles_empty_variant_slots() {
        let config = XkbConfig {
            layout: "us,ir,us".to_string(),
            variant: ",,dvorak".to_string(),
            ..XkbConfig::default()
        };

        let source = config.active_source();
        assert_eq!(source.layout, "us");
        assert_eq!(source.variant, "");
    }
}
