use std::path::Path;

use cosmic::{
    desktop::{DesktopEntryData, fde::{DesktopEntry, IconSource}},
    widget::{Id, icon::Named, image::Handle},
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DesktopAction {
    pub name: String,
    pub exec: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
/// Represents an application entry in the Cosmic Classic Menu.
pub struct ApplicationEntry {
    pub name: String,
    pub generic_name: Option<String>,
    pub id: String,
    /// Desktop file id as defined by the desktop entry specification,
    /// e.g. `wine-Programs-Foo-Bar.desktop` for `applications/wine/Programs/Foo/Bar.desktop`.
    pub desktop_file_id: String,
    pub icon: Option<IconHandle>,
    pub comment: Option<String>,
    // WMDE: searched as well - our own apps carry the brand only in Keywords.
    pub keywords: Vec<String>,
    pub exec: Option<String>,
    pub category: Vec<String>,
    pub is_terminal: bool,
    pub item_id: Id,
    pub desktop_actions: Vec<DesktopAction>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum IconHandle {
    SvgHandle(cosmic::widget::svg::Handle),
    RasterHandle(cosmic::widget::image::Handle),
}

impl From<DesktopEntryData> for ApplicationEntry {
    fn from(app: DesktopEntryData) -> ApplicationEntry {
        ApplicationEntry {
            comment: get_comment(&app),
            keywords: get_keywords(&app),
            is_terminal: get_is_terminal(&app),
            generic_name: get_generic_name(&app),
            desktop_file_id: get_desktop_file_id(app.path.as_deref()),
            id: app.id,
            name: app.name,
            icon: match app.icon {
                IconSource::Name(name) => Some(
                    cosmic::widget::icon::from_name(name.as_str())
                        .size(64)
                        .fallback(Some(cosmic::widget::icon::IconFallback::Names(vec![
                            "application-default".into(),
                            "application-x-executable".into(),
                        ])))
                        .prefer_svg(true)
                        .into(),
                ),
                IconSource::Path(path) => {
                    Some(cosmic::widget::icon(cosmic::widget::icon::from_path(path.clone())).into())
                }
            },
            exec: app.exec,
            category: app.categories,
            item_id: Id::unique(),
            desktop_actions: app.desktop_actions.into_iter().map(From::from).collect(),
        }
    }
}

impl From<cosmic::widget::Icon> for IconHandle {
    fn from(icon: cosmic::widget::Icon) -> IconHandle {
        if let Some(icon_handle) = icon.into_svg_handle() {
            IconHandle::SvgHandle(cosmic::widget::svg::Handle::from(icon_handle))
        } else {
            IconHandle::default()
        }
    }
}

impl From<Named> for IconHandle {
    fn from(named: Named) -> IconHandle {
        if let Some(handle) = named.clone().icon().into_svg_handle() {
            IconHandle::SvgHandle(handle)
        } else if let Some(path) = named.path() {
            IconHandle::RasterHandle(Handle::from_path(path))
        } else {
            // Same fallback as Default: never panic when the theme lookup fails.
            IconHandle::RasterHandle(Handle::from_rgba(1, 1, vec![0u8, 0, 0, 0]))
        }
    }
}

impl From<cosmic::desktop::DesktopAction> for DesktopAction {
    fn from(value: cosmic::desktop::DesktopAction) -> Self {
        Self {
            exec: value.exec,
            name: value.name,
        }
    }
}

impl Default for IconHandle {
    fn default() -> Self {
        let named = cosmic::widget::icon::from_name("application-x-executable").size(32);

        // The icon may resolve to either an SVG or a raster (PNG) file depending on
        // the active icon theme. Never force it into an SVG handle: doing so panics
        // whenever the theme only provides a raster variant at this size.
        if let Some(handle) = named.clone().icon().into_svg_handle() {
            IconHandle::SvgHandle(handle)
        } else if let Some(path) = named.path() {
            IconHandle::RasterHandle(Handle::from_path(path))
        } else {
            // Ultimate fallback: a 1x1 transparent pixel. Always valid, never panics,
            // even if no icon at all could be resolved.
            IconHandle::RasterHandle(Handle::from_rgba(1, 1, vec![0u8, 0, 0, 0]))
        }
    }
}

fn get_comment(app: &DesktopEntryData) -> Option<String> {
    if let Some(path) = &app.path {
        let locale = std::env::var("LANG")
            .ok()
            .and_then(|l| l.split(".").next().map(str::to_string));
        let desktop_entry = DesktopEntry::from_path(path, Some(locale.as_slice()));

        if let Ok(entry) = desktop_entry {
            return Some(
                entry
                    .comment(locale.as_slice())
                    .unwrap_or_default()
                    .into_owned(),
            );
        }
    }

    None
}

// WMDE: the localized Keywords, or the plain ones when the entry has none for the locale.
fn get_keywords(app: &DesktopEntryData) -> Vec<String> {
    if let Some(path) = &app.path {
        let locale = std::env::var("LANG")
            .ok()
            .and_then(|l| l.split(".").next().map(str::to_string));
        let desktop_entry = DesktopEntry::from_path(path, Some(locale.as_slice()));

        if let Ok(entry) = desktop_entry {
            return entry
                .keywords(locale.as_slice())
                .unwrap_or_default()
                .into_iter()
                .filter(|keyword| !keyword.is_empty())
                .map(|keyword| keyword.into_owned())
                .collect();
        }
    }

    Vec::new()
}

fn get_is_terminal(app: &DesktopEntryData) -> bool {
    if let Some(path) = &app.path {
        let locale = std::env::var("LANG")
            .ok()
            .and_then(|l| l.split(".").next().map(str::to_string));
        let desktop_entry = DesktopEntry::from_path(path, Some(locale.as_slice()));

        if let Ok(entry) = desktop_entry {
            return entry.terminal();
        }
    }

    false
}

fn get_generic_name(app: &DesktopEntryData) -> Option<String> {
    if let Some(path) = &app.path {
        let locale = [std::env::var("LANG")
            .ok()
            .and_then(|l| l.split(".").next().map(str::to_string))
            .unwrap_or_else(|| "en_US".to_string())];
        let desktop_entry = DesktopEntry::from_path(path, Some(locale.as_slice()));

        if let Ok(entry) = desktop_entry {
            return entry.generic_name(&locale).map(|name| name.into_owned());
        }
    }

    None
}

fn get_desktop_file_id(path: Option<&Path>) -> String {
    let Some(path) = path else {
        return String::new();
    };

    let components: Vec<_> = path
        .components()
        .map(|c| c.as_os_str().to_string_lossy())
        .collect();

    // the id is the path relative to the `applications` directory with `/` replaced by `-`
    match components.iter().rposition(|c| c == "applications") {
        Some(index) if index + 1 < components.len() => components[index + 1..].join("-"),
        _ => components.last().map(|c| c.to_string()).unwrap_or_default(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn desktop_file_id_of_nested_entry() {
        assert_eq!(
            get_desktop_file_id(Some(Path::new(
                "/home/user/.local/share/applications/wine/Programs/Foo/Bar.desktop"
            ))),
            "wine-Programs-Foo-Bar.desktop"
        );
    }

    #[test]
    fn desktop_file_id_of_top_level_entry() {
        assert_eq!(
            get_desktop_file_id(Some(Path::new(
                "/usr/share/applications/com.cisco.secureclient.gui.desktop"
            ))),
            "com.cisco.secureclient.gui.desktop"
        );
    }
}
