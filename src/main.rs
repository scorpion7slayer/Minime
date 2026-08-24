mod localization;
mod preferences;

use std::{borrow::Cow, collections::HashSet, path::PathBuf, time::Duration};

use anyhow::Result;
use gpui::{
    Animation, AnimationExt as _, AnyElement, App, Application, AssetSource, Bounds, Context, Div,
    ExternalPaths, FocusHandle, FontWeight, ImageSource, IntoElement, KeyBinding, ObjectFit,
    Render, Resource, Rgba, SharedString, Stateful, StyledImage as _, Subscription,
    TitlebarOptions, Transformation, Window, WindowAppearance, WindowBackgroundAppearance,
    WindowBounds, WindowOptions, actions, div, ease_out_quint, img, prelude::*, px, relative, size,
    svg,
};
use minime::{
    CompressionEffort, CompressionOptions, CompressionResult, OutputFormat, ResultState,
    compress_batch, is_supported_path,
};
use rfd::FileDialog;

use crate::{
    localization::Language,
    preferences::{Preferences, ThemePreference},
};

#[derive(Debug, Clone, Copy)]
struct ThemeColor {
    light: u32,
    dark: u32,
}

impl ThemeColor {
    const fn new(light: u32, dark: u32) -> Self {
        Self { light, dark }
    }

    const fn resolve(self, dark_mode: bool) -> u32 {
        if dark_mode { self.dark } else { self.light }
    }
}

const INK: ThemeColor = ThemeColor::new(0x1a1917, 0xf3f1ec);
const MUTED: ThemeColor = ThemeColor::new(0x77736c, 0xaaa59b);
const APP_BG: ThemeColor = ThemeColor::new(0xf4f2ed, 0x151513);
const SURFACE: ThemeColor = ThemeColor::new(0xffffff, 0x20201d);
const CONTROL_BG: ThemeColor = ThemeColor::new(0xeeece7, 0x2a2925);
const HOVER_BG: ThemeColor = ThemeColor::new(0xf6f4f0, 0x302f2b);
const BORDER: ThemeColor = ThemeColor::new(0xe1ddd6, 0x3b3933);
const DIVIDER: ThemeColor = ThemeColor::new(0xeeeae4, 0x32312c);
const BLUE_WASH: ThemeColor = ThemeColor::new(0xe4eff6, 0x21323b);
const BLUE_INK: ThemeColor = ThemeColor::new(0x315f77, 0xa2cde1);
const GREEN_WASH: ThemeColor = ThemeColor::new(0xe8f0e7, 0x25362a);
const GREEN_INK: ThemeColor = ThemeColor::new(0x416a48, 0xa9d5b0);
const RED_WASH: ThemeColor = ThemeColor::new(0xf8e9e7, 0x3b2826);
const RED_INK: ThemeColor = ThemeColor::new(0x963f37, 0xf0a49b);
const PRIMARY_BG: ThemeColor = ThemeColor::new(0x1a1917, 0xf3f1ec);
const PRIMARY_FG: ThemeColor = ThemeColor::new(0xffffff, 0x1a1917);
const PRIMARY_HOVER: ThemeColor = ThemeColor::new(0x34312d, 0xdcd8cf);
const DISABLED_BG: ThemeColor = ThemeColor::new(0xaaa69f, 0x4e4b45);
const DISABLED_FG: ThemeColor = ThemeColor::new(0xffffff, 0xaaa59b);
const CHECK_BORDER: ThemeColor = ThemeColor::new(0xc8c3bb, 0x625f57);

const LOGO_ICON: &str = "minime.svg";
const LOGO_IMAGE: &str = "minime.png";
const PLUS_ICON: &str = "plus.svg";
const IMAGE_ICON: &str = "image.svg";
const CONVERT_ICON: &str = "convert.svg";
const FOLDER_ICON: &str = "folder.svg";
const CLOSE_ICON: &str = "close.svg";
const CHECK_ICON: &str = "check.svg";
const OPEN_ICON: &str = "arrow-up-right.svg";
const SETTINGS_ICON: &str = "settings.svg";
const COFFEE_ICON: &str = "coffee.svg";
const INFO_ICON: &str = "info.svg";

const SUPPORT_URL: &str = "https://buymeacoffee.com/scorpion7slayer";
const DEVELOPMENT_APP_ID: &str = "dev.minime.app";

fn application_id() -> &'static str {
    option_env!("MINIME_APP_ID").unwrap_or(DEVELOPMENT_APP_ID)
}

actions!(minime, [OpenFiles, CompressNow, ClearQueue]);

#[cfg(target_os = "macos")]
const MONO_FONT: &str = "SF Mono";
#[cfg(target_os = "windows")]
const MONO_FONT: &str = "Cascadia Mono";
#[cfg(all(not(target_os = "macos"), not(target_os = "windows")))]
const MONO_FONT: &str = "DejaVu Sans Mono";

struct Assets;

impl AssetSource for Assets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        let bytes: &'static [u8] = match path {
            LOGO_ICON => include_bytes!("../assets/minime.svg"),
            LOGO_IMAGE => include_bytes!("../assets/minime.png"),
            PLUS_ICON => include_bytes!("../assets/plus.svg"),
            IMAGE_ICON => include_bytes!("../assets/image.svg"),
            CONVERT_ICON => include_bytes!("../assets/convert.svg"),
            FOLDER_ICON => include_bytes!("../assets/folder.svg"),
            CLOSE_ICON => include_bytes!("../assets/close.svg"),
            CHECK_ICON => include_bytes!("../assets/check.svg"),
            OPEN_ICON => include_bytes!("../assets/arrow-up-right.svg"),
            SETTINGS_ICON => include_bytes!("../assets/settings.svg"),
            COFFEE_ICON => include_bytes!("../assets/coffee.svg"),
            INFO_ICON => include_bytes!("../assets/info.svg"),
            _ => return Ok(None),
        };
        Ok(Some(Cow::Borrowed(bytes)))
    }

    fn list(&self, _path: &str) -> Result<Vec<SharedString>> {
        Ok([
            LOGO_ICON,
            LOGO_IMAGE,
            PLUS_ICON,
            IMAGE_ICON,
            CONVERT_ICON,
            FOLDER_ICON,
            CLOSE_ICON,
            CHECK_ICON,
            OPEN_ICON,
            SETTINGS_ICON,
            COFFEE_ICON,
            INFO_ICON,
        ]
        .into_iter()
        .map(SharedString::from)
        .collect())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AppView {
    Introduction,
    Workspace,
    Settings,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PreviewVersion {
    Original,
    Optimized,
}

#[derive(Debug, Clone, Copy)]
enum PreferenceToggle {
    Preview,
    RevealAfterCompression,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ButtonMotion {
    HeaderAdd,
    EmptyChoose,
    QueueAdd,
    Destination,
    Compress,
    Preference(usize),
    IntroStart,
    Support,
}

impl ButtonMotion {
    fn id(self) -> String {
        match self {
            Self::HeaderAdd => "header-add".into(),
            Self::EmptyChoose => "empty-choose".into(),
            Self::QueueAdd => "queue-add".into(),
            Self::Destination => "destination".into(),
            Self::Compress => "compress".into(),
            Self::Preference(index) => format!("preference-{index}"),
            Self::IntroStart => "intro-start".into(),
            Self::Support => "support".into(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SelectionGroup {
    Preview,
    Language,
    Theme,
    Format,
    Effort,
}

impl SelectionGroup {
    const fn id(self) -> &'static str {
        match self {
            Self::Preview => "preview",
            Self::Language => "language",
            Self::Theme => "theme",
            Self::Format => "format",
            Self::Effort => "effort",
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct SelectionMotion {
    group: SelectionGroup,
    from: usize,
    to: usize,
    epoch: u64,
}

fn dark_mode_for(theme: ThemePreference, appearance: WindowAppearance) -> bool {
    match theme {
        ThemePreference::System => matches!(
            appearance,
            WindowAppearance::Dark | WindowAppearance::VibrantDark
        ),
        ThemePreference::Light => false,
        ThemePreference::Dark => true,
    }
}

struct MinimeApp {
    _appearance_subscription: Subscription,
    focus_handle: FocusHandle,
    files: Vec<PathBuf>,
    output_format: OutputFormat,
    output_dir: Option<PathBuf>,
    reject_larger: bool,
    effort: CompressionEffort,
    language: Language,
    theme: ThemePreference,
    dark_mode: bool,
    show_preview: bool,
    reveal_after_compression: bool,
    intro_seen: bool,
    view: AppView,
    selected_index: usize,
    preview_version: PreviewVersion,
    processing: bool,
    results: Vec<CompressionResult>,
    notice: Option<String>,
    button_motion: Option<ButtonMotion>,
    button_motion_epoch: u64,
    selection_motion: Option<SelectionMotion>,
    selection_motion_epoch: u64,
}

impl MinimeApp {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let focus_handle = cx.focus_handle();
        focus_handle.focus(window);
        let appearance_subscription =
            window.observe_window_appearance(|window, _| window.refresh());
        let preferences = Preferences::load();
        let dark_mode = dark_mode_for(preferences.theme, window.appearance());
        let view = if preferences.intro_seen {
            AppView::Workspace
        } else {
            AppView::Introduction
        };
        Self {
            _appearance_subscription: appearance_subscription,
            focus_handle,
            files: Vec::new(),
            output_format: preferences.output_format,
            output_dir: preferences.output_dir,
            reject_larger: preferences.reject_larger,
            effort: preferences.effort,
            language: preferences.language,
            theme: preferences.theme,
            dark_mode,
            show_preview: preferences.show_preview,
            reveal_after_compression: preferences.reveal_after_compression,
            intro_seen: preferences.intro_seen,
            view,
            selected_index: 0,
            preview_version: PreviewVersion::Original,
            processing: false,
            results: Vec::new(),
            notice: None,
            button_motion: None,
            button_motion_epoch: 0,
            selection_motion: None,
            selection_motion_epoch: 0,
        }
    }

    fn text<'a>(&self, french: &'a str, english: &'a str) -> &'a str {
        self.language.text(french, english)
    }

    fn persist_preferences(&self) {
        let preferences = Preferences {
            language: self.language,
            theme: self.theme,
            output_format: self.output_format,
            output_dir: self.output_dir.clone(),
            reject_larger: self.reject_larger,
            effort: self.effort,
            show_preview: self.show_preview,
            reveal_after_compression: self.reveal_after_compression,
            intro_seen: self.intro_seen,
        };
        if let Err(error) = preferences.save() {
            log::warn!("Unable to save Minime preferences: {error}");
        }
    }

    fn set_language(&mut self, language: Language, cx: &mut Context<Self>) {
        let from = Language::ALL
            .iter()
            .position(|candidate| *candidate == self.language)
            .unwrap_or(0);
        let to = Language::ALL
            .iter()
            .position(|candidate| *candidate == language)
            .unwrap_or(from);
        self.trigger_selection_motion(SelectionGroup::Language, from, to);
        self.language = language;
        self.persist_preferences();
        cx.notify();
    }

    fn set_theme(&mut self, theme: ThemePreference, cx: &mut Context<Self>) {
        let from = ThemePreference::ALL
            .iter()
            .position(|candidate| *candidate == self.theme)
            .unwrap_or(0);
        let to = ThemePreference::ALL
            .iter()
            .position(|candidate| *candidate == theme)
            .unwrap_or(from);
        self.trigger_selection_motion(SelectionGroup::Theme, from, to);
        self.theme = theme;
        self.persist_preferences();
        cx.notify();
    }

    fn trigger_button_motion(&mut self, motion: ButtonMotion, cx: &mut Context<Self>) {
        self.button_motion = Some(motion);
        self.button_motion_epoch = self.button_motion_epoch.wrapping_add(1).max(1);
        cx.notify();
    }

    fn trigger_selection_motion(&mut self, group: SelectionGroup, from: usize, to: usize) {
        if from == to {
            return;
        }
        self.selection_motion_epoch = self.selection_motion_epoch.wrapping_add(1).max(1);
        self.selection_motion = Some(SelectionMotion {
            group,
            from,
            to,
            epoch: self.selection_motion_epoch,
        });
    }

    fn finish_introduction(&mut self, cx: &mut Context<Self>) {
        self.intro_seen = true;
        self.view = AppView::Workspace;
        self.persist_preferences();
        cx.notify();
    }

    fn open_introduction(&mut self, cx: &mut Context<Self>) {
        self.view = AppView::Introduction;
        cx.notify();
    }

    fn open_settings(&mut self, cx: &mut Context<Self>) {
        if !self.processing {
            self.view = AppView::Settings;
            cx.notify();
        }
    }

    fn close_settings(&mut self, cx: &mut Context<Self>) {
        self.view = AppView::Workspace;
        cx.notify();
    }

    fn pick_files(&mut self, cx: &mut Context<Self>) {
        if self.processing || self.view != AppView::Workspace {
            return;
        }
        let dialog_title = self.text("Ajouter des images", "Add images");
        let filter_title = self.text("Formats acceptés", "Accepted formats");
        let files = FileDialog::new()
            .set_title(dialog_title)
            .add_filter(
                filter_title,
                &[
                    "png", "apng", "jpg", "jpeg", "jfif", "webp", "gif", "bmp", "tif", "tiff",
                    "tga", "dds", "qoi", "ico", "pnm", "ppm", "pgm", "pam", "pbm", "ff",
                ],
            )
            .pick_files();

        if let Some(files) = files {
            self.add_paths(files);
            cx.notify();
        }
    }

    fn add_paths(&mut self, paths: Vec<PathBuf>) {
        if self.processing {
            return;
        }
        let mut known = self.files.iter().cloned().collect::<HashSet<_>>();
        let mut added = 0;
        let mut rejected = 0;

        for path in paths {
            if is_supported_path(&path) && known.insert(path.clone()) {
                self.files.push(path);
                added += 1;
            } else {
                rejected += 1;
            }
        }

        if added > 0 {
            self.selected_index = self.files.len().saturating_sub(added);
            self.preview_version = PreviewVersion::Original;
        }
        self.results.clear();
        self.notice = match (added, rejected) {
            (0, 0) => None,
            (0, _) => Some(
                self.text(
                    "Minime n’a trouvé aucune image qu’il puisse ouvrir.",
                    "Minime couldn’t find an image it can open.",
                )
                .into(),
            ),
            (_, 0) => None,
            (_, _) => Some(if self.language == Language::French {
                if rejected == 1 {
                    "1 fichier n’a pas pu être ajouté.".into()
                } else {
                    format!("{rejected} fichiers n’ont pas pu être ajoutés.")
                }
            } else {
                if rejected == 1 {
                    "1 file couldn’t be added.".into()
                } else {
                    format!("{rejected} files couldn’t be added.")
                }
            }),
        };
    }

    fn choose_output_dir(&mut self, cx: &mut Context<Self>) {
        if self.processing {
            return;
        }
        let dialog_title = self.text("Enregistrer les résultats dans…", "Save results to…");
        if let Some(directory) = FileDialog::new().set_title(dialog_title).pick_folder() {
            self.output_dir = Some(directory);
            self.results.clear();
            self.persist_preferences();
            cx.notify();
        }
    }

    fn reset_output_dir(&mut self, cx: &mut Context<Self>) {
        if !self.processing {
            self.output_dir = None;
            self.results.clear();
            self.persist_preferences();
            cx.notify();
        }
    }

    fn remove_file(&mut self, index: usize, cx: &mut Context<Self>) {
        if !self.processing && index < self.files.len() {
            self.files.remove(index);
            self.selected_index = self.selected_index.min(self.files.len().saturating_sub(1));
            self.preview_version = PreviewVersion::Original;
            self.results.clear();
            self.notice = None;
            cx.notify();
        }
    }

    fn clear(&mut self, cx: &mut Context<Self>) {
        if self.processing || self.view != AppView::Workspace {
            return;
        }
        self.files.clear();
        self.selected_index = 0;
        self.preview_version = PreviewVersion::Original;
        self.results.clear();
        self.notice = None;
        cx.notify();
    }

    fn start_compression(&mut self, cx: &mut Context<Self>) {
        if self.processing || self.files.is_empty() || self.view != AppView::Workspace {
            return;
        }

        let paths = self.files.clone();
        let options = CompressionOptions {
            output_format: self.output_format,
            output_dir: self.output_dir.clone(),
            reject_larger: self.reject_larger,
            effort: self.effort,
        };

        self.processing = true;
        self.results.clear();
        self.notice = None;
        cx.notify();

        let task = cx
            .background_executor()
            .spawn(async move { compress_batch(paths, options) });
        cx.spawn(async move |this, cx| {
            let results = task.await;
            this.update(cx, |this, cx| {
                this.processing = false;
                let reveal_path = this
                    .reveal_after_compression
                    .then(|| results.iter().find_map(|result| result.output_path.clone()));
                this.results = results;
                if this
                    .results
                    .get(this.selected_index)
                    .and_then(|result| result.output_path.as_ref())
                    .is_some()
                {
                    this.preview_version = PreviewVersion::Optimized;
                }
                if let Some(Some(path)) = reveal_path {
                    cx.reveal_path(&path);
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    fn summary(&self) -> (u64, usize, usize, usize) {
        let saved = self.results.iter().map(|result| result.bytes_saved()).sum();
        let completed = self
            .results
            .iter()
            .filter(|result| result.state == ResultState::Saved)
            .count();
        let unchanged = self
            .results
            .iter()
            .filter(|result| result.state == ResultState::Unchanged)
            .count();
        let failed = self
            .results
            .iter()
            .filter(|result| result.state == ResultState::Failed)
            .count();
        (saved, completed, unchanged, failed)
    }

    fn source_bytes(&self) -> u64 {
        self.files
            .iter()
            .filter_map(|path| std::fs::metadata(path).ok())
            .map(|metadata| metadata.len())
            .sum()
    }

    fn rgb(&self) -> impl Fn(ThemeColor) -> Rgba + Copy + use<> {
        let dark_mode = self.dark_mode;
        move |color| gpui::rgb(color.resolve(dark_mode))
    }

    fn icon(&self, path: &'static str, icon_size: f32, color: ThemeColor) -> AnyElement {
        svg()
            .path(path)
            .size(px(icon_size))
            .text_color((self.rgb())(color))
            .into_any_element()
    }

    fn animate_button(&self, button: Stateful<Div>, motion: ButtonMotion) -> AnyElement {
        let animate = self.button_motion == Some(motion) && self.button_motion_epoch > 0;
        let epoch = if animate { self.button_motion_epoch } else { 0 };
        button
            .with_animation(
                SharedString::from(format!("button-press-{}-{epoch}", motion.id())),
                Animation::new(Duration::from_millis(150)).with_easing(ease_out_quint()),
                move |button, delta| {
                    if animate {
                        button.opacity(0.82 + delta * 0.18)
                    } else {
                        button
                    }
                },
            )
            .into_any_element()
    }

    fn animated_checkbox(&self, checked: bool, motion: ButtonMotion) -> AnyElement {
        let rgb = self.rgb();
        let animate = self.button_motion == Some(motion) && self.button_motion_epoch > 0;
        let epoch = if animate { self.button_motion_epoch } else { 0 };
        let check =
            checked.then(|| self.animated_button_icon(CHECK_ICON, 12.0, PRIMARY_FG, motion));

        div()
            .size(px(18.0))
            .flex_none()
            .flex()
            .items_center()
            .justify_center()
            .rounded_sm()
            .border_1()
            .border_color(if checked {
                rgb(PRIMARY_BG)
            } else {
                rgb(CHECK_BORDER)
            })
            .bg(if checked {
                rgb(PRIMARY_BG)
            } else {
                rgb(SURFACE)
            })
            .children(check)
            .with_animation(
                SharedString::from(format!("checkbox-{}-{epoch}", motion.id())),
                Animation::new(Duration::from_millis(170)).with_easing(ease_out_quint()),
                move |checkbox, delta| {
                    if animate {
                        checkbox.opacity(0.72 + delta * 0.28)
                    } else {
                        checkbox
                    }
                },
            )
            .into_any_element()
    }

    fn animated_button_icon(
        &self,
        path: &'static str,
        icon_size: f32,
        color: ThemeColor,
        motion: ButtonMotion,
    ) -> AnyElement {
        let animate = self.button_motion == Some(motion) && self.button_motion_epoch > 0;
        let epoch = if animate { self.button_motion_epoch } else { 0 };
        svg()
            .path(path)
            .size(px(icon_size))
            .text_color((self.rgb())(color))
            .with_animation(
                SharedString::from(format!("button-icon-{}-{epoch}", motion.id())),
                Animation::new(Duration::from_millis(240)).with_easing(ease_out_quint()),
                move |icon, delta| {
                    if animate {
                        let scale = 0.25 + delta * 0.75;
                        icon.opacity(delta)
                            .with_transformation(Transformation::scale(size(scale, scale)))
                    } else {
                        icon
                    }
                },
            )
            .into_any_element()
    }

    fn selection_indicator(
        &self,
        group: SelectionGroup,
        current_index: usize,
        item_count: usize,
        bordered: bool,
    ) -> AnyElement {
        let rgb = self.rgb();
        let motion = self
            .selection_motion
            .filter(|motion| motion.group == group && motion.to == current_index);
        let epoch = motion.map_or(0, |motion| motion.epoch);
        let from = motion.map_or(current_index, |motion| motion.from) as f32;
        let to = current_index as f32;
        let divisor = item_count.max(1) as f32;

        div()
            .absolute()
            .top_0()
            .bottom_0()
            .w(relative(1.0 / divisor))
            .left(relative(to / divisor))
            .rounded_sm()
            .bg(rgb(SURFACE))
            .when(bordered, |this| this.border_1().border_color(rgb(BORDER)))
            .with_animation(
                SharedString::from(format!("selection-{}-{epoch}", group.id())),
                Animation::new(Duration::from_millis(190)).with_easing(ease_out_quint()),
                move |indicator, delta| {
                    if motion.is_some() {
                        let position = from + (to - from) * delta;
                        indicator
                            .left(relative(position / divisor))
                            .opacity(0.78 + delta * 0.22)
                    } else {
                        indicator
                    }
                },
            )
            .into_any_element()
    }

    fn logo_source() -> ImageSource {
        ImageSource::Resource(Resource::Embedded(LOGO_IMAGE.into()))
    }

    fn render_header(&self, cx: &mut Context<Self>) -> AnyElement {
        let rgb = self.rgb();
        let busy = self.processing;
        div()
            .h(px(48.0))
            .flex()
            .items_center()
            .justify_between()
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .child(img(Self::logo_source()).size_9())
                    .child(
                        div()
                            .child(
                                div()
                                    .text_base()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .child("Minime"),
                            )
                            .child(div().mt(px(1.0)).text_xs().text_color(rgb(MUTED)).child(
                                self.text(
                                    "Alléger ou convertir, en local",
                                    "Shrink or convert, right here",
                                ),
                            )),
                    ),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .when(!self.files.is_empty(), |this| {
                        this.child(
                            div()
                                .id("clear-queue")
                                .size_10()
                                .flex()
                                .items_center()
                                .justify_center()
                                .rounded_md()
                                .text_color(rgb(MUTED))
                                .when(!busy, |this| {
                                    this.cursor_pointer()
                                        .hover(|style| style.bg(rgb(HOVER_BG)))
                                        .active(|style| style.opacity(0.82))
                                        .on_click(cx.listener(|this, _, _, cx| this.clear(cx)))
                                })
                                .child(self.icon(CLOSE_ICON, 18.0, MUTED)),
                        )
                    })
                    .child(
                        self.animate_button(
                            div()
                                .id("support")
                                .h_10()
                                .px_3()
                                .flex()
                                .items_center()
                                .gap_2()
                                .rounded_md()
                                .text_sm()
                                .font_weight(FontWeight::MEDIUM)
                                .cursor_pointer()
                                .hover(|style| style.bg(rgb(HOVER_BG)))
                                .active(|style| style.opacity(0.82))
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.trigger_button_motion(ButtonMotion::Support, cx);
                                    cx.open_url(SUPPORT_URL);
                                }))
                                .child(self.animated_button_icon(
                                    COFFEE_ICON,
                                    16.0,
                                    INK,
                                    ButtonMotion::Support,
                                ))
                                .child(self.text("Soutenir", "Support")),
                            ButtonMotion::Support,
                        ),
                    )
                    .child(
                        div()
                            .id("open-settings")
                            .size_10()
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded_md()
                            .text_color(rgb(MUTED))
                            .when(!busy, |this| {
                                this.cursor_pointer()
                                    .hover(|style| style.bg(rgb(HOVER_BG)).text_color(rgb(INK)))
                                    .active(|style| style.opacity(0.82))
                                    .on_click(cx.listener(|this, _, _, cx| this.open_settings(cx)))
                            })
                            .child(self.icon(SETTINGS_ICON, 18.0, MUTED)),
                    )
                    .child(
                        self.animate_button(
                            div()
                                .id("header-add")
                                .h_10()
                                .px_3()
                                .flex()
                                .items_center()
                                .gap_2()
                                .rounded_md()
                                .border_1()
                                .border_color(rgb(BORDER))
                                .bg(rgb(SURFACE))
                                .text_sm()
                                .font_weight(FontWeight::MEDIUM)
                                .when(!busy, |this| {
                                    this.cursor_pointer()
                                        .hover(|style| style.bg(rgb(HOVER_BG)))
                                        .active(|style| style.opacity(0.82))
                                        .on_click(cx.listener(|this, _, _, cx| {
                                            this.trigger_button_motion(ButtonMotion::HeaderAdd, cx);
                                            this.pick_files(cx);
                                        }))
                                })
                                .child(self.animated_button_icon(
                                    PLUS_ICON,
                                    16.0,
                                    INK,
                                    ButtonMotion::HeaderAdd,
                                ))
                                .child(self.text("Ajouter", "Add")),
                            ButtonMotion::HeaderAdd,
                        ),
                    ),
            )
            .into_any_element()
    }

    fn render_empty(&self, cx: &mut Context<Self>) -> AnyElement {
        let rgb = self.rgb();
        let mark = img(Self::logo_source()).size(px(58.0)).with_animation(
            "empty-mark",
            Animation::new(Duration::from_millis(360)).with_easing(ease_out_quint()),
            |mark, delta| mark.opacity(delta),
        );
        let choose_button = self.animate_button(
            div()
                .id("empty-choose")
                .mt_5()
                .h_10()
                .px_3()
                .flex()
                .items_center()
                .gap_2()
                .rounded_md()
                .bg(rgb(PRIMARY_BG))
                .text_color(rgb(PRIMARY_FG))
                .text_sm()
                .font_weight(FontWeight::SEMIBOLD)
                .cursor_pointer()
                .hover(|style| style.bg(rgb(PRIMARY_HOVER)))
                .active(|style| style.opacity(0.82))
                .on_click(cx.listener(|this, _, _, cx| {
                    this.trigger_button_motion(ButtonMotion::EmptyChoose, cx);
                    this.pick_files(cx);
                }))
                .child(self.animated_button_icon(
                    PLUS_ICON,
                    16.0,
                    PRIMARY_FG,
                    ButtonMotion::EmptyChoose,
                ))
                .child(self.text("Parcourir…", "Browse…")),
            ButtonMotion::EmptyChoose,
        );

        div()
            .size_full()
            .min_h(px(260.0))
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .px_6()
            .child(mark)
            .child(
                div()
                    .mt_4()
                    .text_lg()
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(self.text("Glissez vos images ici", "Drop images here"))
                    .with_animations(
                        "empty-title-enter",
                        vec![
                            Animation::new(Duration::from_millis(70)),
                            Animation::new(Duration::from_millis(260))
                                .with_easing(ease_out_quint()),
                        ],
                        |title, animation, delta| {
                            if animation == 0 {
                                title.opacity(0.0)
                            } else {
                                title.opacity(delta)
                            }
                        },
                    ),
            )
            .child(
                div()
                    .mt_1()
                    .text_sm()
                    .text_color(rgb(MUTED))
                    .text_center()
                    .line_height(relative(1.45))
                    .child(self.text(
                        "Allégez-les ou changez leur format. Rien ne quitte votre appareil.",
                        "Shrink them or switch formats. Nothing leaves your device.",
                    ))
                    .with_animations(
                        "empty-copy-enter",
                        vec![
                            Animation::new(Duration::from_millis(130)),
                            Animation::new(Duration::from_millis(260))
                                .with_easing(ease_out_quint()),
                        ],
                        |copy, animation, delta| {
                            if animation == 0 {
                                copy.opacity(0.0)
                            } else {
                                copy.opacity(delta)
                            }
                        },
                    ),
            )
            .child(
                div()
                    .flex()
                    .justify_center()
                    .child(choose_button)
                    .with_animations(
                        "empty-button-enter",
                        vec![
                            Animation::new(Duration::from_millis(190)),
                            Animation::new(Duration::from_millis(260))
                                .with_easing(ease_out_quint()),
                        ],
                        |button, animation, delta| {
                            if animation == 0 {
                                button.opacity(0.0)
                            } else {
                                button.opacity(delta)
                            }
                        },
                    ),
            )
            .child(
                div()
                    .mt_3()
                    .font_family(MONO_FONT)
                    .text_xs()
                    .text_color(rgb(MUTED))
                    .child(match (self.language, cfg!(target_os = "macos")) {
                        (Language::French, true) => "ou ⌘O",
                        (Language::French, false) => "ou Ctrl+O",
                        (Language::English, true) => "or ⌘O",
                        (Language::English, false) => "or Ctrl+O",
                    }),
            )
            .into_any_element()
    }

    fn render_list_header(&self, cx: &mut Context<Self>) -> AnyElement {
        let rgb = self.rgb();
        let (saved, completed, unchanged, failed) = self.summary();
        let has_results = !self.results.is_empty();
        let converting = self.output_format != OutputFormat::Auto;
        let format_label = self.output_format.label();
        let headline = if has_results {
            if failed > 0 {
                if self.language == Language::French {
                    let done = if completed == 1 {
                        "1 fichier terminé".into()
                    } else {
                        format!("{completed} fichiers terminés")
                    };
                    let errors = if failed == 1 {
                        "1 erreur".into()
                    } else {
                        format!("{failed} erreurs")
                    };
                    format!("{done} · {errors}")
                } else {
                    let done = if completed == 1 {
                        "1 file finished".into()
                    } else {
                        format!("{completed} files finished")
                    };
                    let errors = if failed == 1 {
                        "1 error".into()
                    } else {
                        format!("{failed} errors")
                    };
                    format!("{done} · {errors}")
                }
            } else if converting {
                if completed == 0 {
                    self.text("Aucune conversion enregistrée", "No converted files saved")
                        .into()
                } else if self.language == Language::French {
                    if completed == 1 {
                        format!("1 fichier converti en {format_label}")
                    } else {
                        format!("{completed} fichiers convertis en {format_label}")
                    }
                } else if completed == 1 {
                    format!("1 file converted to {format_label}")
                } else {
                    format!("{completed} files converted to {format_label}")
                }
            } else if saved > 0 {
                if self.language == Language::French {
                    format!("{} gagnés", self.language.format_bytes(saved))
                } else {
                    format!("{} saved", self.language.format_bytes(saved))
                }
            } else {
                self.text("Rien de plus léger trouvé", "Nothing smaller found")
                    .into()
            }
        } else {
            let count = self.files.len();
            let images = if count == 1 {
                self.text("1 image", "1 image").into()
            } else {
                format!("{count} images")
            };
            format!(
                "{images} · {}",
                self.language.format_bytes(self.source_bytes())
            )
        };
        let detail = if self.processing {
            if converting {
                if self.language == Language::French {
                    format!("Conversion en {format_label}…")
                } else {
                    format!("Converting to {format_label}…")
                }
            } else {
                self.text(
                    "Minime cherche une version plus légère…",
                    "Looking for a smaller version…",
                )
                .into()
            }
        } else if has_results {
            if unchanged == 0 && failed == 0 {
                self.text("Tout est prêt", "All done").into()
            } else if self.language == Language::French {
                let stored = if completed == 1 {
                    "1 fichier enregistré".into()
                } else {
                    format!("{completed} fichiers enregistrés")
                };
                let kept = if unchanged == 1 {
                    "1 original gardé".into()
                } else {
                    format!("{unchanged} originaux gardés")
                };
                format!("{stored} · {kept}")
            } else {
                let stored = if completed == 1 {
                    "1 file saved".into()
                } else {
                    format!("{completed} files saved")
                };
                let kept = if unchanged == 1 {
                    "1 original kept".into()
                } else {
                    format!("{unchanged} originals kept")
                };
                format!("{stored} · {kept}")
            }
        } else if converting {
            if self.language == Language::French {
                format!("Prêtes pour une conversion en {format_label}")
            } else {
                format!("Ready to convert to {format_label}")
            }
        } else {
            self.text("Prêtes à être allégées", "Ready to shrink")
                .into()
        };
        let action_icon = if self.processing {
            self.icon(IMAGE_ICON, 19.0, BLUE_INK)
        } else {
            self.animated_button_icon(PLUS_ICON, 18.0, MUTED, ButtonMotion::QueueAdd)
        };

        div()
            .h(px(54.0))
            .px_4()
            .flex()
            .items_center()
            .justify_between()
            .border_b_1()
            .border_color(rgb(DIVIDER))
            .child(
                div()
                    .min_w_0()
                    .child(
                        div()
                            .text_sm()
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(headline),
                    )
                    .child(
                        div()
                            .mt(px(1.0))
                            .text_xs()
                            .text_color(rgb(MUTED))
                            .child(detail),
                    ),
            )
            .child(
                self.animate_button(
                    div()
                        .id("list-add")
                        .size_10()
                        .flex()
                        .items_center()
                        .justify_center()
                        .rounded_md()
                        .text_color(rgb(MUTED))
                        .when(!self.processing, |this| {
                            this.cursor_pointer()
                                .hover(|style| style.bg(rgb(HOVER_BG)).text_color(rgb(INK)))
                                .active(|style| style.opacity(0.82))
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.trigger_button_motion(ButtonMotion::QueueAdd, cx);
                                    this.pick_files(cx);
                                }))
                        })
                        .child(action_icon),
                    ButtonMotion::QueueAdd,
                ),
            )
            .into_any_element()
    }

    fn render_file_row(
        &self,
        path: &std::path::Path,
        index: usize,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let rgb = self.rgb();
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("Image")
            .to_string();
        let extension = path
            .extension()
            .and_then(|extension| extension.to_str())
            .unwrap_or("IMG")
            .to_ascii_uppercase();
        let parent = path
            .parent()
            .and_then(|parent| parent.file_name())
            .and_then(|name| name.to_str())
            .unwrap_or("—")
            .to_string();
        let file_size = std::fs::metadata(path)
            .map(|metadata| self.language.format_bytes(metadata.len()))
            .unwrap_or_else(|_| "—".into());
        let selected = self.selected_index == index;

        div()
            .id(("file", index))
            .h(px(58.0))
            .px_3()
            .flex()
            .items_center()
            .gap_3()
            .border_b_1()
            .border_color(rgb(DIVIDER))
            .when(selected, |this| this.bg(rgb(CONTROL_BG)))
            .hover(|style| style.bg(rgb(HOVER_BG)))
            .when(!self.processing, |this| {
                this.cursor_pointer()
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.selected_index = index;
                        this.preview_version = PreviewVersion::Original;
                        cx.notify();
                    }))
            })
            .child(
                div()
                    .size_9()
                    .flex_none()
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded_md()
                    .bg(rgb(BLUE_WASH))
                    .font_family(MONO_FONT)
                    .text_color(rgb(BLUE_INK))
                    .text_xs()
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(extension),
            )
            .child(
                div()
                    .min_w_0()
                    .flex_1()
                    .child(
                        div()
                            .w_full()
                            .text_sm()
                            .font_weight(FontWeight::MEDIUM)
                            .text_ellipsis()
                            .child(name),
                    )
                    .child(
                        div()
                            .mt(px(1.0))
                            .text_xs()
                            .text_color(rgb(MUTED))
                            .text_ellipsis()
                            .child(parent),
                    ),
            )
            .child(
                div()
                    .flex_none()
                    .font_family(MONO_FONT)
                    .text_xs()
                    .text_color(rgb(MUTED))
                    .child(file_size),
            )
            .child(
                div()
                    .id(("remove", index))
                    .size_10()
                    .flex_none()
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded_md()
                    .text_color(rgb(MUTED))
                    .when(!self.processing, |this| {
                        this.cursor_pointer()
                            .hover(|style| style.bg(rgb(RED_WASH)).text_color(rgb(RED_INK)))
                            .active(|style| style.opacity(0.82))
                            .on_click(
                                cx.listener(move |this, _, _, cx| this.remove_file(index, cx)),
                            )
                    })
                    .child(self.icon(CLOSE_ICON, 17.0, MUTED)),
            )
            .with_animation(
                ("file-enter", index),
                Animation::new(Duration::from_millis(180)).with_easing(ease_out_quint()),
                |row, delta| row.opacity(delta),
            )
            .into_any_element()
    }

    fn render_result_row(
        &self,
        result: &CompressionResult,
        index: usize,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let rgb = self.rgb();
        let name = result
            .input_path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("Image")
            .to_string();
        let converting = self.output_format != OutputFormat::Auto;
        let (label, wash, color, icon_path) = match result.state {
            ResultState::Saved => (
                if converting {
                    self.text("Convertie", "Converted")
                } else {
                    self.text("Allégée", "Smaller")
                },
                GREEN_WASH,
                GREEN_INK,
                CHECK_ICON,
            ),
            ResultState::Unchanged => (
                if converting {
                    self.text("Ignorée", "Skipped")
                } else {
                    self.text("Original gardé", "Original kept")
                },
                BLUE_WASH,
                BLUE_INK,
                CHECK_ICON,
            ),
            ResultState::Failed => (self.text("Erreur", "Failed"), RED_WASH, RED_INK, CLOSE_ICON),
        };
        let details = match result.state {
            ResultState::Saved if converting => format!(
                "{} → {} · {}",
                self.language.format_bytes(result.original_bytes),
                self.language.format_bytes(result.output_bytes),
                result.output_format.map(OutputFormat::label).unwrap_or("—")
            ),
            ResultState::Saved => format!(
                "{} → {} · {:.1}% {}",
                self.language.format_bytes(result.original_bytes),
                self.language.format_bytes(result.output_bytes),
                result.savings_percent(),
                self.text("plus légère", "smaller")
            ),
            ResultState::Unchanged => {
                if converting {
                    self.text(
                        "La conversion aurait créé un fichier plus lourd.",
                        "The converted file would have been larger.",
                    )
                    .into()
                } else {
                    self.text(
                        "Minime n’a pas trouvé de version plus légère.",
                        "Minime couldn’t find a smaller version.",
                    )
                    .into()
                }
            }
            _ => self.language.engine_error(&result.message),
        };
        let output_path = result.output_path.clone();
        let selected = self.selected_index == index;
        let status_icon = svg()
            .path(icon_path)
            .size_4()
            .text_color(rgb(color))
            .with_animation(
                ("status-icon", index),
                Animation::new(Duration::from_millis(260)).with_easing(ease_out_quint()),
                |icon, delta| {
                    let scale = 0.25 + delta * 0.75;
                    icon.opacity(delta)
                        .with_transformation(Transformation::scale(size(scale, scale)))
                },
            );

        div()
            .id(("result", index))
            .h(px(58.0))
            .px_3()
            .flex()
            .items_center()
            .gap_3()
            .border_b_1()
            .border_color(rgb(DIVIDER))
            .when(selected, |this| this.bg(rgb(CONTROL_BG)))
            .cursor_pointer()
            .hover(|style| style.bg(rgb(HOVER_BG)))
            .active(|style| style.opacity(0.82))
            .on_click(cx.listener(move |this, _, _, cx| {
                this.selected_index = index;
                this.preview_version = if this
                    .results
                    .get(index)
                    .and_then(|result| result.output_path.as_ref())
                    .is_some()
                {
                    PreviewVersion::Optimized
                } else {
                    PreviewVersion::Original
                };
                cx.notify();
            }))
            .child(
                div()
                    .size_9()
                    .flex_none()
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded_md()
                    .bg(rgb(wash))
                    .child(status_icon),
            )
            .child(
                div()
                    .min_w_0()
                    .flex_1()
                    .child(
                        div()
                            .w_full()
                            .text_sm()
                            .font_weight(FontWeight::MEDIUM)
                            .text_ellipsis()
                            .child(name),
                    )
                    .child(
                        div()
                            .mt(px(1.0))
                            .text_xs()
                            .text_color(if result.state == ResultState::Failed {
                                rgb(RED_INK)
                            } else {
                                rgb(MUTED)
                            })
                            .text_ellipsis()
                            .child(details),
                    ),
            )
            .child(
                div()
                    .flex_none()
                    .text_color(rgb(color))
                    .text_xs()
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(label),
            )
            .when_some(output_path, |this, path| {
                this.child(
                    div()
                        .id(("reveal-result", index))
                        .size_10()
                        .flex_none()
                        .flex()
                        .items_center()
                        .justify_center()
                        .rounded_md()
                        .hover(|style| style.bg(rgb(SURFACE)))
                        .on_click(move |_, _, cx| cx.reveal_path(&path))
                        .child(self.icon(OPEN_ICON, 17.0, MUTED)),
                )
            })
            .with_animation(
                ("result-enter", index),
                Animation::new(Duration::from_millis(220)).with_easing(ease_out_quint()),
                |row, delta| row.opacity(delta),
            )
            .into_any_element()
    }

    fn render_inspector(&self, cx: &mut Context<Self>) -> AnyElement {
        let rgb = self.rgb();
        let source_path = self
            .files
            .get(self.selected_index)
            .cloned()
            .unwrap_or_default();
        let result = self.results.get(self.selected_index);
        let output_path = result.and_then(|result| result.output_path.clone());
        let showing_output = self.preview_version == PreviewVersion::Optimized
            && output_path.as_ref().is_some_and(|path| path.is_file());
        let display_path = if showing_output {
            output_path.clone().unwrap_or_else(|| source_path.clone())
        } else {
            source_path.clone()
        };
        let file_name = source_path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("Image")
            .to_string();
        let extension = display_path
            .extension()
            .and_then(|extension| extension.to_str())
            .unwrap_or("—")
            .to_ascii_uppercase();
        let dimensions = image::image_dimensions(&display_path)
            .map(|(width, height)| format!("{width} × {height}"))
            .unwrap_or_else(|_| "—".into());
        let file_size = std::fs::metadata(&display_path)
            .map(|metadata| self.language.format_bytes(metadata.len()))
            .unwrap_or_else(|_| "—".into());
        let fallback = self
            .text(
                "Aperçu indisponible pour ce fichier",
                "Preview unavailable for this file",
            )
            .to_string();

        let original_selected = !showing_output;
        let optimized_available = output_path.is_some();
        let preview_index = usize::from(showing_output);

        div()
            .w(px(288.0))
            .h_full()
            .flex_none()
            .flex()
            .flex_col()
            .border_l_1()
            .border_color(rgb(DIVIDER))
            .child(
                div()
                    .h(px(54.0))
                    .px_3()
                    .flex()
                    .items_center()
                    .border_b_1()
                    .border_color(rgb(DIVIDER))
                    .child(
                        div()
                            .min_w_0()
                            .child(
                                div()
                                    .text_sm()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .child(self.text("Aperçu", "Preview")),
                            )
                            .child(
                                div()
                                    .mt(px(1.0))
                                    .text_xs()
                                    .text_color(rgb(MUTED))
                                    .text_ellipsis()
                                    .child(file_name),
                            ),
                    ),
            )
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .p_3()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(
                        div()
                            .h(px(104.0))
                            .flex_none()
                            .w_full()
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded_lg()
                            .bg(rgb(APP_BG))
                            .overflow_hidden()
                            .child(
                                img(display_path)
                                    .size(px(104.0))
                                    .object_fit(ObjectFit::Contain)
                                    .with_loading(move || {
                                        div()
                                            .size_full()
                                            .flex()
                                            .items_center()
                                            .justify_center()
                                            .child(div().size_4().rounded_full().bg(rgb(BORDER)))
                                            .into_any_element()
                                    })
                                    .with_fallback(move || {
                                        div()
                                            .size_full()
                                            .px_4()
                                            .flex()
                                            .items_center()
                                            .justify_center()
                                            .text_center()
                                            .text_xs()
                                            .text_color(rgb(MUTED))
                                            .child(fallback.clone())
                                            .into_any_element()
                                    }),
                            ),
                    )
                    .child(
                        div()
                            .h_9()
                            .p(px(3.0))
                            .rounded_md()
                            .bg(rgb(CONTROL_BG))
                            .child(
                                div()
                                    .relative()
                                    .size_full()
                                    .flex()
                                    .child(self.selection_indicator(
                                        SelectionGroup::Preview,
                                        preview_index,
                                        2,
                                        false,
                                    ))
                                    .child(
                                        div()
                                            .id("preview-original")
                                            .relative()
                                            .h_full()
                                            .flex_1()
                                            .flex()
                                            .items_center()
                                            .justify_center()
                                            .rounded_sm()
                                            .text_xs()
                                            .font_weight(if original_selected {
                                                FontWeight::SEMIBOLD
                                            } else {
                                                FontWeight::MEDIUM
                                            })
                                            .text_color(if original_selected {
                                                rgb(INK)
                                            } else {
                                                rgb(MUTED)
                                            })
                                            .cursor_pointer()
                                            .hover(|style| style.text_color(rgb(INK)))
                                            .active(|style| style.opacity(0.82))
                                            .on_click(cx.listener(|this, _, _, cx| {
                                                let from = usize::from(
                                                    this.preview_version
                                                        == PreviewVersion::Optimized,
                                                );
                                                this.trigger_selection_motion(
                                                    SelectionGroup::Preview,
                                                    from,
                                                    0,
                                                );
                                                this.preview_version = PreviewVersion::Original;
                                                cx.notify();
                                            }))
                                            .child(self.text("Original", "Original")),
                                    )
                                    .child(
                                        div()
                                            .id("preview-optimized")
                                            .relative()
                                            .h_full()
                                            .flex_1()
                                            .flex()
                                            .items_center()
                                            .justify_center()
                                            .rounded_sm()
                                            .text_xs()
                                            .font_weight(if showing_output {
                                                FontWeight::SEMIBOLD
                                            } else {
                                                FontWeight::MEDIUM
                                            })
                                            .text_color(if optimized_available {
                                                if showing_output { rgb(INK) } else { rgb(MUTED) }
                                            } else {
                                                rgb(DISABLED_FG)
                                            })
                                            .when(optimized_available, |this| {
                                                this.cursor_pointer()
                                                    .hover(|style| style.text_color(rgb(INK)))
                                                    .active(|style| style.opacity(0.82))
                                                    .on_click(cx.listener(|this, _, _, cx| {
                                                        let from = usize::from(
                                                            this.preview_version
                                                                == PreviewVersion::Optimized,
                                                        );
                                                        this.trigger_selection_motion(
                                                            SelectionGroup::Preview,
                                                            from,
                                                            1,
                                                        );
                                                        this.preview_version =
                                                            PreviewVersion::Optimized;
                                                        cx.notify();
                                                    }))
                                            })
                                            .child(self.text("Minime", "Minime")),
                                    ),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .font_family(MONO_FONT)
                            .text_xs()
                            .text_color(rgb(MUTED))
                            .child(format!("{extension} · {dimensions}"))
                            .child(file_size),
                    ),
            )
            .into_any_element()
    }

    fn render_content(&self, narrow: bool, cx: &mut Context<Self>) -> AnyElement {
        let rgb = self.rgb();
        div()
            .id("content-panel")
            .flex_1()
            .min_h_0()
            .w_full()
            .rounded_xl()
            .border_1()
            .border_color(rgb(BORDER))
            .bg(rgb(SURFACE))
            .overflow_hidden()
            .drag_over::<ExternalPaths>(move |style, _, _, _| {
                style.bg(rgb(BLUE_WASH)).border_color(rgb(BLUE_INK))
            })
            .can_drop(|value, _, _| value.downcast_ref::<ExternalPaths>().is_some())
            .on_drop(cx.listener(|this, paths: &ExternalPaths, _, cx| {
                if !this.processing {
                    this.add_paths(paths.paths().to_vec());
                    cx.notify();
                }
            }))
            .when(self.files.is_empty(), |this| {
                this.child(self.render_empty(cx))
            })
            .when(!self.files.is_empty(), |this| {
                this.flex()
                    .child(
                        div()
                            .min_w_0()
                            .flex_1()
                            .h_full()
                            .flex()
                            .flex_col()
                            .child(self.render_list_header(cx))
                            .child(
                                div()
                                    .id("file-scroll")
                                    .flex_1()
                                    .min_h_0()
                                    .overflow_y_scroll()
                                    .children(if self.results.is_empty() {
                                        self.files
                                            .iter()
                                            .enumerate()
                                            .map(|(index, path)| {
                                                self.render_file_row(path, index, cx)
                                            })
                                            .collect::<Vec<_>>()
                                    } else {
                                        self.results
                                            .iter()
                                            .enumerate()
                                            .map(|(index, result)| {
                                                self.render_result_row(result, index, cx)
                                            })
                                            .collect::<Vec<_>>()
                                    }),
                            ),
                    )
                    .when(self.show_preview && !narrow, |this| {
                        this.child(self.render_inspector(cx))
                    })
            })
            .into_any_element()
    }

    fn render_format_picker(&self, cx: &mut Context<Self>) -> AnyElement {
        let rgb = self.rgb();
        let selected_index = OutputFormat::ALL
            .iter()
            .position(|format| *format == self.output_format)
            .unwrap_or(0);
        div()
            .h_10()
            .p(px(3.0))
            .rounded_md()
            .bg(rgb(CONTROL_BG))
            .child(
                div()
                    .relative()
                    .size_full()
                    .flex()
                    .items_center()
                    .child(self.selection_indicator(
                        SelectionGroup::Format,
                        selected_index,
                        OutputFormat::ALL.len(),
                        true,
                    ))
                    .children(
                        OutputFormat::ALL
                            .into_iter()
                            .enumerate()
                            .map(|(index, format)| {
                                let selected = self.output_format == format;
                                div()
                                    .id(("format", index))
                                    .relative()
                                    .h_full()
                                    .min_w(px(42.0))
                                    .px_2()
                                    .flex_1()
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .rounded_sm()
                                    .text_xs()
                                    .font_weight(if selected {
                                        FontWeight::SEMIBOLD
                                    } else {
                                        FontWeight::MEDIUM
                                    })
                                    .text_color(if selected { rgb(INK) } else { rgb(MUTED) })
                                    .when(!self.processing, |this| {
                                        this.cursor_pointer()
                                            .hover(|style| style.text_color(rgb(INK)))
                                            .active(|style| style.opacity(0.82))
                                            .on_click(cx.listener(move |this, _, _, cx| {
                                                let from = OutputFormat::ALL
                                                    .iter()
                                                    .position(|candidate| {
                                                        *candidate == this.output_format
                                                    })
                                                    .unwrap_or(0);
                                                this.trigger_selection_motion(
                                                    SelectionGroup::Format,
                                                    from,
                                                    index,
                                                );
                                                if this.output_format != format {
                                                    this.reject_larger =
                                                        format == OutputFormat::Auto;
                                                }
                                                this.output_format = format;
                                                this.results.clear();
                                                this.preview_version = PreviewVersion::Original;
                                                this.persist_preferences();
                                                cx.notify();
                                            }))
                                    })
                                    .child(format.label())
                            }),
                    ),
            )
            .into_any_element()
    }

    fn render_destination(&self, cx: &mut Context<Self>) -> AnyElement {
        let rgb = self.rgb();
        let label = self
            .output_dir
            .as_ref()
            .and_then(|path| path.file_name())
            .and_then(|name| name.to_str())
            .unwrap_or(self.text("À côté des originaux", "Beside originals"))
            .to_string();

        div()
            .flex()
            .items_center()
            .gap_1()
            .child(
                self.animate_button(
                    div()
                        .id("choose-destination")
                        .h_10()
                        .min_w_0()
                        .flex_1()
                        .px_3()
                        .flex()
                        .items_center()
                        .gap_2()
                        .rounded_md()
                        .border_1()
                        .border_color(rgb(BORDER))
                        .bg(rgb(SURFACE))
                        .when(!self.processing, |this| {
                            this.cursor_pointer()
                                .hover(|style| style.bg(rgb(HOVER_BG)))
                                .active(|style| style.opacity(0.82))
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.trigger_button_motion(ButtonMotion::Destination, cx);
                                    this.choose_output_dir(cx);
                                }))
                        })
                        .child(self.animated_button_icon(
                            FOLDER_ICON,
                            16.0,
                            MUTED,
                            ButtonMotion::Destination,
                        ))
                        .child(
                            div()
                                .min_w_0()
                                .text_xs()
                                .font_weight(FontWeight::MEDIUM)
                                .text_ellipsis()
                                .child(label),
                        ),
                    ButtonMotion::Destination,
                ),
            )
            .when(self.output_dir.is_some(), |this| {
                this.child(
                    div()
                        .id("reset-destination")
                        .size_10()
                        .flex_none()
                        .flex()
                        .items_center()
                        .justify_center()
                        .rounded_md()
                        .text_color(rgb(MUTED))
                        .when(!self.processing, |this| {
                            this.cursor_pointer()
                                .hover(|style| style.bg(rgb(HOVER_BG)).text_color(rgb(INK)))
                                .active(|style| style.opacity(0.82))
                                .on_click(cx.listener(|this, _, _, cx| this.reset_output_dir(cx)))
                        })
                        .child(self.icon(CLOSE_ICON, 16.0, MUTED)),
                )
            })
            .into_any_element()
    }

    fn render_compress_button(&self, cx: &mut Context<Self>) -> AnyElement {
        let rgb = self.rgb();
        let disabled = self.files.is_empty() || self.processing;
        let converting = self.output_format != OutputFormat::Auto;
        let format_label = self.output_format.label();
        let label = if self.processing {
            if converting {
                self.text("Conversion…", "Converting…").into()
            } else {
                self.text("Compression…", "Making smaller…").into()
            }
        } else if converting {
            if self.language == Language::French {
                format!("Convertir en {format_label}")
            } else {
                format!("Convert to {format_label}")
            }
        } else if self.files.is_empty() {
            self.text("Alléger", "Make smaller").into()
        } else if self.files.len() == 1 {
            self.text("Alléger l’image", "Make image smaller").into()
        } else {
            if self.language == Language::French {
                format!("Alléger {} images", self.files.len())
            } else {
                format!("Shrink {} images", self.files.len())
            }
        };
        let action_icon = if self.processing {
            self.icon(IMAGE_ICON, 17.0, PRIMARY_FG)
        } else {
            self.animated_button_icon(
                if converting { CONVERT_ICON } else { IMAGE_ICON },
                17.0,
                if disabled { DISABLED_FG } else { PRIMARY_FG },
                ButtonMotion::Compress,
            )
        };

        self.animate_button(
            div()
                .id("compress")
                .h_10()
                .px_3()
                .flex_none()
                .flex()
                .items_center()
                .justify_center()
                .gap_2()
                .rounded_md()
                .bg(if disabled {
                    rgb(DISABLED_BG)
                } else {
                    rgb(PRIMARY_BG)
                })
                .text_color(if disabled {
                    rgb(DISABLED_FG)
                } else {
                    rgb(PRIMARY_FG)
                })
                .text_sm()
                .font_weight(FontWeight::SEMIBOLD)
                .when(!disabled, |this| {
                    this.cursor_pointer()
                        .hover(|style| style.bg(rgb(PRIMARY_HOVER)))
                        .active(|style| style.opacity(0.82))
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.trigger_button_motion(ButtonMotion::Compress, cx);
                            this.start_compression(cx);
                        }))
                })
                .child(action_icon)
                .child(label),
            ButtonMotion::Compress,
        )
    }

    fn render_safety_toggle(&self, cx: &mut Context<Self>) -> AnyElement {
        let rgb = self.rgb();
        let motion = ButtonMotion::Preference(2);
        let label = if self.output_format == OutputFormat::Auto {
            self.text(
                "Garder l’original si Minime ne fait pas mieux",
                "Keep the original if Minime can’t make it smaller",
            )
        } else {
            self.text(
                "Ignorer la conversion si elle alourdit le fichier",
                "Skip the conversion if it makes the file larger",
            )
        };

        div()
            .id("reject-larger")
            .h_10()
            .flex()
            .items_center()
            .gap_2()
            .when(!self.processing, |this| {
                this.cursor_pointer()
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.trigger_button_motion(motion, cx);
                        this.reject_larger = !this.reject_larger;
                        this.results.clear();
                        this.persist_preferences();
                        cx.notify();
                    }))
            })
            .child(self.animated_checkbox(self.reject_larger, motion))
            .child(div().text_xs().text_color(rgb(MUTED)).child(label))
            .into_any_element()
    }

    fn render_controls(&self, narrow: bool, cx: &mut Context<Self>) -> AnyElement {
        let rgb = self.rgb();
        let selected_description =
            if matches!(self.output_format, OutputFormat::Auto | OutputFormat::Png) {
                format!(
                    "{} · {}",
                    self.language.format_description(self.output_format),
                    self.language.effort_label(self.effort)
                )
            } else if self.language == Language::French {
                format!("Conversion exacte vers {}", self.output_format.label())
            } else {
                format!("Pixel-exact conversion to {}", self.output_format.label())
            };
        let primary_row = div()
            .flex()
            .when(narrow, |this| this.flex_col())
            .items_end()
            .gap_2()
            .child(
                div()
                    .w(if narrow {
                        relative(1.0)
                    } else {
                        px(308.0).into()
                    })
                    .child(
                        div()
                            .mb_1()
                            .text_xs()
                            .text_color(rgb(MUTED))
                            .child(self.text("Résultat", "Result")),
                    )
                    .child(self.render_format_picker(cx)),
            )
            .child(
                div()
                    .min_w_0()
                    .flex_1()
                    .w(if narrow { relative(1.0) } else { relative(0.0) })
                    .child(
                        div()
                            .mb_1()
                            .text_xs()
                            .text_color(rgb(MUTED))
                            .child(self.text("Enregistrer dans", "Save to")),
                    )
                    .child(self.render_destination(cx)),
            )
            .child(
                div()
                    .w(if narrow {
                        relative(1.0)
                    } else {
                        px(160.0).into()
                    })
                    .child(div().mb_1().text_xs().text_color(rgb(SURFACE)).child(" "))
                    .child(self.render_compress_button(cx)),
            );

        div()
            .w_full()
            .p_2()
            .rounded_xl()
            .border_1()
            .border_color(rgb(BORDER))
            .bg(rgb(SURFACE))
            .child(primary_row)
            .child(
                div()
                    .mt_2()
                    .pt_2()
                    .border_t_1()
                    .border_color(rgb(DIVIDER))
                    .flex()
                    .when(narrow, |this| this.flex_col().items_start())
                    .when(!narrow, |this| this.items_center().justify_between())
                    .gap_1()
                    .child(self.render_safety_toggle(cx))
                    .child(
                        div()
                            .font_family(MONO_FONT)
                            .text_xs()
                            .text_color(rgb(MUTED))
                            .child(selected_description),
                    ),
            )
            .into_any_element()
    }

    fn render_language_picker(&self, cx: &mut Context<Self>) -> AnyElement {
        let rgb = self.rgb();
        let selected_index = Language::ALL
            .iter()
            .position(|language| *language == self.language)
            .unwrap_or(0);
        div()
            .h_10()
            .w(px(104.0))
            .p(px(3.0))
            .rounded_md()
            .bg(rgb(CONTROL_BG))
            .child(
                div()
                    .relative()
                    .size_full()
                    .flex()
                    .child(self.selection_indicator(
                        SelectionGroup::Language,
                        selected_index,
                        Language::ALL.len(),
                        false,
                    ))
                    .children(
                        Language::ALL
                            .into_iter()
                            .enumerate()
                            .map(|(index, language)| {
                                let selected = self.language == language;
                                div()
                                    .id(("language", index))
                                    .relative()
                                    .h_full()
                                    .flex_1()
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .rounded_sm()
                                    .font_family(MONO_FONT)
                                    .text_xs()
                                    .font_weight(if selected {
                                        FontWeight::SEMIBOLD
                                    } else {
                                        FontWeight::MEDIUM
                                    })
                                    .text_color(if selected { rgb(INK) } else { rgb(MUTED) })
                                    .cursor_pointer()
                                    .hover(|style| style.text_color(rgb(INK)))
                                    .active(|style| style.opacity(0.82))
                                    .on_click(cx.listener(move |this, _, _, cx| {
                                        this.set_language(language, cx)
                                    }))
                                    .child(language.label())
                            }),
                    ),
            )
            .into_any_element()
    }

    fn render_theme_picker(&self, cx: &mut Context<Self>) -> AnyElement {
        let rgb = self.rgb();
        let selected_index = ThemePreference::ALL
            .iter()
            .position(|theme| *theme == self.theme)
            .unwrap_or(0);
        div()
            .h_10()
            .p(px(3.0))
            .rounded_md()
            .bg(rgb(CONTROL_BG))
            .child(
                div()
                    .relative()
                    .size_full()
                    .flex()
                    .child(self.selection_indicator(
                        SelectionGroup::Theme,
                        selected_index,
                        ThemePreference::ALL.len(),
                        false,
                    ))
                    .children(ThemePreference::ALL.into_iter().enumerate().map(
                        |(index, theme)| {
                            let selected = self.theme == theme;
                            div()
                                .id(("theme", index))
                                .relative()
                                .h_full()
                                .flex_1()
                                .flex()
                                .items_center()
                                .justify_center()
                                .rounded_sm()
                                .text_xs()
                                .font_weight(if selected {
                                    FontWeight::SEMIBOLD
                                } else {
                                    FontWeight::MEDIUM
                                })
                                .text_color(if selected { rgb(INK) } else { rgb(MUTED) })
                                .cursor_pointer()
                                .hover(|style| style.text_color(rgb(INK)))
                                .active(|style| style.opacity(0.82))
                                .on_click(
                                    cx.listener(move |this, _, _, cx| this.set_theme(theme, cx)),
                                )
                                .child(theme.label(self.language))
                        },
                    )),
            )
            .into_any_element()
    }

    fn render_effort_picker(&self, cx: &mut Context<Self>) -> AnyElement {
        let rgb = self.rgb();
        let selected_index = CompressionEffort::ALL
            .iter()
            .position(|effort| *effort == self.effort)
            .unwrap_or(0);
        div()
            .h_10()
            .p(px(3.0))
            .rounded_md()
            .bg(rgb(CONTROL_BG))
            .child(
                div()
                    .relative()
                    .size_full()
                    .flex()
                    .child(self.selection_indicator(
                        SelectionGroup::Effort,
                        selected_index,
                        CompressionEffort::ALL.len(),
                        false,
                    ))
                    .children(CompressionEffort::ALL.into_iter().enumerate().map(
                        |(index, effort)| {
                            let selected = self.effort == effort;
                            div()
                                .id(("effort", index))
                                .relative()
                                .h_full()
                                .flex_1()
                                .flex()
                                .items_center()
                                .justify_center()
                                .rounded_sm()
                                .text_xs()
                                .font_weight(if selected {
                                    FontWeight::SEMIBOLD
                                } else {
                                    FontWeight::MEDIUM
                                })
                                .text_color(if selected { rgb(INK) } else { rgb(MUTED) })
                                .cursor_pointer()
                                .hover(|style| style.text_color(rgb(INK)))
                                .active(|style| style.opacity(0.82))
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    let from = CompressionEffort::ALL
                                        .iter()
                                        .position(|candidate| *candidate == this.effort)
                                        .unwrap_or(0);
                                    this.trigger_selection_motion(
                                        SelectionGroup::Effort,
                                        from,
                                        index,
                                    );
                                    this.effort = effort;
                                    this.results.clear();
                                    this.preview_version = PreviewVersion::Original;
                                    this.persist_preferences();
                                    cx.notify();
                                }))
                                .child(self.language.effort_label(effort))
                        },
                    )),
            )
            .into_any_element()
    }

    fn render_preference_toggle(
        &self,
        id: &'static str,
        label: &'static str,
        description: &'static str,
        checked: bool,
        preference: PreferenceToggle,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let rgb = self.rgb();
        let motion_index = match preference {
            PreferenceToggle::Preview => 0,
            PreferenceToggle::RevealAfterCompression => 1,
        };
        let motion = ButtonMotion::Preference(motion_index);
        div()
            .id(id)
            .h(px(50.0))
            .flex()
            .items_center()
            .gap_3()
            .cursor_pointer()
            .on_click(cx.listener(move |this, _, _, cx| {
                this.trigger_button_motion(motion, cx);
                match preference {
                    PreferenceToggle::Preview => this.show_preview = !this.show_preview,
                    PreferenceToggle::RevealAfterCompression => {
                        this.reveal_after_compression = !this.reveal_after_compression;
                    }
                }
                this.persist_preferences();
                cx.notify();
            }))
            .child(self.animated_checkbox(checked, motion))
            .child(
                div()
                    .min_w_0()
                    .child(div().text_sm().font_weight(FontWeight::MEDIUM).child(label))
                    .child(
                        div()
                            .mt(px(1.0))
                            .text_xs()
                            .text_color(rgb(MUTED))
                            .child(description),
                    ),
            )
            .into_any_element()
    }

    fn render_introduction(&self, cx: &mut Context<Self>) -> AnyElement {
        let rgb = self.rgb();
        div()
            .size_full()
            .p_3()
            .flex()
            .flex_col()
            .gap_3()
            .child(
                div()
                    .h(px(48.0))
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_3()
                            .child(img(Self::logo_source()).size_9())
                            .child(
                                div()
                                    .text_base()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .child("Minime"),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .when(self.intro_seen, |this| {
                                this.child(
                                    div()
                                        .id("close-introduction")
                                        .size_10()
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .rounded_md()
                                        .cursor_pointer()
                                        .hover(|style| style.bg(rgb(HOVER_BG)))
                                        .on_click(cx.listener(|this, _, _, cx| {
                                            this.finish_introduction(cx)
                                        }))
                                        .child(self.icon(CLOSE_ICON, 17.0, MUTED)),
                                )
                            })
                            .child(self.render_language_picker(cx)),
                    ),
            )
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .p_5()
                    .rounded_xl()
                    .border_1()
                    .border_color(rgb(BORDER))
                    .bg(rgb(SURFACE))
                    .flex()
                    .gap_5()
                    .child(
                        div()
                            .w(relative(0.46))
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .justify_between()
                            .child(
                                div()
                                    .child(
                                        div()
                                            .font_family(MONO_FONT)
                                            .text_xs()
                                            .text_color(rgb(BLUE_INK))
                                            .child(self.text(
                                                "MINIME, EN BREF",
                                                "A QUICK TOUR",
                                            )),
                                    )
                                    .child(
                                        div()
                                            .mt_3()
                                            .text_2xl()
                                            .font_weight(FontWeight::SEMIBOLD)
                                            .line_height(relative(1.08))
                                            .child(self.text(
                                                "Allégez une image ou changez son format.",
                                                "Make an image smaller or change its format.",
                                            )),
                                    )
                                    .child(
                                        div()
                                            .mt_3()
                                            .text_sm()
                                            .line_height(relative(1.55))
                                            .text_color(rgb(MUTED))
                                            .child(self.text(
                                                "Choisissez Auto pour gagner de la place, ou indiquez un format pour créer une nouvelle copie. Tout reste sur votre ordinateur, et les pixels sont vérifiés avant l’enregistrement.",
                                                "Choose Auto to save space, or pick a format to make a new copy. Everything stays on your computer, and the pixels are checked before the file is saved.",
                                            )),
                                    ),
                            )
                            .child(self.animate_button(
                                div()
                                    .id("start-minime")
                                    .h_10()
                                    .w(px(132.0))
                                    .px_3()
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .rounded_md()
                                    .bg(rgb(PRIMARY_BG))
                                    .text_color(rgb(PRIMARY_FG))
                                    .text_sm()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .cursor_pointer()
                                    .hover(|style| style.bg(rgb(PRIMARY_HOVER)))
                                    .active(|style| style.opacity(0.82))
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.trigger_button_motion(ButtonMotion::IntroStart, cx);
                                        this.finish_introduction(cx);
                                    }))
                                    .child(self.text("Ouvrir Minime", "Open Minime")),
                                ButtonMotion::IntroStart,
                            )),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .p_4()
                            .rounded_lg()
                            .bg(rgb(CONTROL_BG))
                            .flex()
                            .flex_col()
                            .justify_center()
                            .gap_4()
                            .children([
                                self.render_intro_step(
                                    "01",
                                    self.text("Ajoutez vos images", "Bring in your images"),
                                    self.text(
                                        "Glissez-les dans la fenêtre ou choisissez-les dans un dossier.",
                                        "Drop them into the window or choose them from a folder.",
                                    ),
                                    60,
                                ),
                                self.render_intro_step(
                                    "02",
                                    self.text(
                                        "Allégez ou convertissez",
                                        "Shrink or convert",
                                    ),
                                    self.text(
                                        "Auto cherche un fichier plus petit. Les autres choix convertissent vers le format indiqué.",
                                        "Auto looks for a smaller file. Every other choice converts to that format.",
                                    ),
                                    140,
                                ),
                                self.render_intro_step(
                                    "03",
                                    self.text("Récupérez le résultat", "Pick up the result"),
                                    self.text(
                                        "Comparez les deux versions, puis ouvrez le fichier terminé depuis la liste.",
                                        "Compare both versions, then open the finished file from the list.",
                                    ),
                                    220,
                                ),
                            ]),
                    ),
            )
            .into_any_element()
    }

    fn render_intro_step(
        &self,
        number: &'static str,
        title: &'static str,
        description: &'static str,
        delay_ms: u64,
    ) -> AnyElement {
        let rgb = self.rgb();
        div()
            .flex()
            .gap_3()
            .child(
                div()
                    .size_9()
                    .flex_none()
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded_md()
                    .bg(rgb(SURFACE))
                    .font_family(MONO_FONT)
                    .text_xs()
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(rgb(BLUE_INK))
                    .child(number),
            )
            .child(
                div()
                    .min_w_0()
                    .child(
                        div()
                            .text_sm()
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(title),
                    )
                    .child(
                        div()
                            .mt(px(2.0))
                            .text_xs()
                            .line_height(relative(1.45))
                            .text_color(rgb(MUTED))
                            .child(description),
                    ),
            )
            .with_animations(
                SharedString::from(format!("intro-step-{number}")),
                vec![
                    Animation::new(Duration::from_millis(delay_ms)),
                    Animation::new(Duration::from_millis(280)).with_easing(ease_out_quint()),
                ],
                |step, animation, delta| {
                    if animation == 0 {
                        step.opacity(0.0)
                    } else {
                        step.opacity(delta)
                    }
                },
            )
            .into_any_element()
    }

    fn render_settings(&self, cx: &mut Context<Self>) -> AnyElement {
        let rgb = self.rgb();
        div()
            .size_full()
            .p_3()
            .flex()
            .flex_col()
            .gap_3()
            .child(
                div()
                    .h(px(48.0))
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_3()
                            .child(
                                div()
                                    .id("close-settings")
                                    .size_10()
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .rounded_md()
                                    .cursor_pointer()
                                    .hover(|style| style.bg(rgb(HOVER_BG)))
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.close_settings(cx)
                                    }))
                                    .child(self.icon(CLOSE_ICON, 17.0, MUTED)),
                            )
                            .child(
                                div()
                                    .child(
                                        div()
                                            .text_base()
                                            .font_weight(FontWeight::SEMIBOLD)
                                            .child(self.text("Paramètres", "Settings")),
                                    )
                                    .child(
                                        div()
                                            .mt(px(1.0))
                                            .text_xs()
                                            .text_color(rgb(MUTED))
                                            .child(self.text(
                                                "Les changements sont enregistrés tout de suite",
                                                "Changes are saved as you make them",
                                            )),
                                    ),
                            ),
                    )
                    .child(self.render_language_picker(cx)),
            )
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .p_4()
                    .rounded_xl()
                    .border_1()
                    .border_color(rgb(BORDER))
                    .bg(rgb(SURFACE))
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(
                        div()
                            .child(
                                div()
                                    .mb_1()
                                    .text_xs()
                                    .text_color(rgb(MUTED))
                                    .child(self.text("Apparence", "Appearance")),
                            )
                            .child(self.render_theme_picker(cx)),
                    )
                    .child(
                        div()
                            .flex()
                            .gap_3()
                            .child(
                                div()
                                    .w(px(300.0))
                                    .child(
                                        div()
                                            .mb_1()
                                            .text_xs()
                                            .text_color(rgb(MUTED))
                                            .child(self.text(
                                                "Format au démarrage",
                                                "Starting format",
                                            )),
                                    )
                                    .child(self.render_format_picker(cx)),
                            )
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .child(
                                        div()
                                            .mb_1()
                                            .text_xs()
                                            .text_color(rgb(MUTED))
                                            .child(self.text(
                                                "Temps consacré au PNG",
                                                "PNG effort",
                                            )),
                                    )
                                    .child(self.render_effort_picker(cx)),
                            ),
                    )
                    .child(
                        div()
                            .pt_2()
                            .border_t_1()
                            .border_color(rgb(DIVIDER))
                            .flex()
                            .gap_5()
                            .child(
                                div()
                                    .flex_1()
                                    .child(self.render_preference_toggle(
                                        "settings-preview",
                                        self.text("Afficher l’aperçu", "Show preview"),
                                        self.text(
                                            "Voir l’image sélectionnée à côté de la liste.",
                                            "See the selected image beside the list.",
                                        ),
                                        self.show_preview,
                                        PreferenceToggle::Preview,
                                        cx,
                                    ))
                                    .child(self.render_safety_toggle(cx)),
                            )
                            .child(
                                div()
                                    .flex_1()
                                    .child(self.render_preference_toggle(
                                        "settings-reveal",
                                        self.text(
                                            "Montrer le fichier terminé",
                                            "Show the finished file",
                                        ),
                                        self.text(
                                            "Ouvrir son dossier dès que Minime a terminé.",
                                            "Open its folder as soon as Minime is done.",
                                        ),
                                        self.reveal_after_compression,
                                        PreferenceToggle::RevealAfterCompression,
                                        cx,
                                    ))
                                    .child(
                                        div()
                                            .id("show-introduction")
                                            .h_10()
                                            .flex()
                                            .items_center()
                                            .gap_2()
                                            .text_sm()
                                            .text_color(rgb(MUTED))
                                            .cursor_pointer()
                                            .hover(|style| style.text_color(rgb(INK)))
                                            .on_click(cx.listener(|this, _, _, cx| {
                                                this.open_introduction(cx)
                                            }))
                                            .child(self.icon(INFO_ICON, 16.0, MUTED))
                                            .child(self.text(
                                                "Revoir le démarrage",
                                                "See the quick tour",
                                            )),
                                    ),
                            ),
                    )
                    .child(
                        div()
                            .mt_auto()
                            .pt_3()
                            .border_t_1()
                            .border_color(rgb(DIVIDER))
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(
                                div()
                                    .child(
                                        div()
                                            .text_sm()
                                            .font_weight(FontWeight::SEMIBOLD)
                                            .child(self.text(
                                                "Un café pour la suite ?",
                                                "Want to help Minime along?",
                                            )),
                                    )
                                    .child(
                                        div()
                                            .mt(px(1.0))
                                            .text_xs()
                                            .text_color(rgb(MUTED))
                                            .child(self.text(
                                                "Ça aide à garder l’application simple et indépendante.",
                                                "It helps keep the app simple and independent.",
                                            )),
                                    ),
                            )
                            .child(self.animate_button(
                                div()
                                    .id("buy-me-a-coffee")
                                    .h_10()
                                    .px_3()
                                    .flex()
                                    .items_center()
                                    .gap_2()
                                    .rounded_md()
                                    .bg(rgb(PRIMARY_BG))
                                    .text_color(rgb(PRIMARY_FG))
                                    .text_sm()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .cursor_pointer()
                                    .hover(|style| style.bg(rgb(PRIMARY_HOVER)))
                                    .active(|style| style.opacity(0.82))
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.trigger_button_motion(ButtonMotion::Support, cx);
                                        cx.open_url(SUPPORT_URL);
                                    }))
                                    .child(self.animated_button_icon(
                                        COFFEE_ICON,
                                        16.0,
                                        PRIMARY_FG,
                                        ButtonMotion::Support,
                                    ))
                                    .child("Buy me a coffee"),
                                ButtonMotion::Support,
                            )),
                    ),
            )
            .with_animation(
                "settings-enter",
                Animation::new(Duration::from_millis(220)).with_easing(ease_out_quint()),
                |settings, delta| settings.opacity(delta),
            )
            .into_any_element()
    }

    fn render_workspace(&self, narrow: bool, cx: &mut Context<Self>) -> AnyElement {
        let rgb = self.rgb();
        div()
            .size_full()
            .p_3()
            .flex()
            .flex_col()
            .gap_3()
            .child(self.render_header(cx))
            .child(self.render_content(narrow, cx))
            .child(self.render_controls(narrow, cx))
            .when_some(self.notice.clone(), |this, notice| {
                this.child(
                    div()
                        .px_3()
                        .py_2()
                        .rounded_md()
                        .bg(rgb(RED_WASH))
                        .text_color(rgb(RED_INK))
                        .text_xs()
                        .child(notice)
                        .with_animation(
                            "notice-enter",
                            Animation::new(Duration::from_millis(180))
                                .with_easing(ease_out_quint()),
                            |notice, delta| notice.opacity(delta),
                        ),
                )
            })
            .with_animation(
                "app-enter",
                Animation::new(Duration::from_millis(280)).with_easing(ease_out_quint()),
                |app, delta| app.opacity(delta),
            )
            .into_any_element()
    }
}

impl Render for MinimeApp {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.dark_mode = dark_mode_for(self.theme, window.appearance());
        let rgb = self.rgb();
        let narrow = f32::from(window.viewport_size().width) < 700.0;
        let page = match self.view {
            AppView::Introduction => self.render_introduction(cx),
            AppView::Workspace => self.render_workspace(narrow, cx),
            AppView::Settings => self.render_settings(cx),
        };

        div()
            .id("app")
            .key_context("Minime")
            .track_focus(&self.focus_handle)
            .on_action(cx.listener(|this, _: &OpenFiles, _, cx| this.pick_files(cx)))
            .on_action(cx.listener(|this, _: &CompressNow, _, cx| this.start_compression(cx)))
            .on_action(cx.listener(|this, _: &ClearQueue, _, cx| this.clear(cx)))
            .size_full()
            .overflow_hidden()
            .bg(rgb(APP_BG))
            .text_color(rgb(INK))
            .child(page)
    }
}

#[cfg(target_os = "macos")]
fn install_dock_icon() {
    use objc2::{AnyThread as _, MainThreadMarker};
    use objc2_app_kit::{NSApplication, NSImage};
    use objc2_foundation::NSData;

    let Some(main_thread) = MainThreadMarker::new() else {
        return;
    };
    let data = NSData::with_bytes(include_bytes!("../assets/minime.png"));
    let Some(image) = NSImage::initWithData(NSImage::alloc(), &data) else {
        log::warn!("Unable to decode the Minime dock icon");
        return;
    };
    let application = NSApplication::sharedApplication(main_thread);

    // SAFETY: `image` is a valid, non-null NSImage retained for the duration of this call.
    unsafe { application.setApplicationIconImage(Some(&image)) };
}

fn main() {
    env_logger::init();

    Application::new().with_assets(Assets).run(|cx: &mut App| {
        #[cfg(target_os = "macos")]
        install_dock_icon();

        cx.on_window_closed(|cx| {
            if cx.windows().is_empty() {
                cx.quit();
            }
        })
        .detach();

        let bounds = Bounds::centered(None, size(px(760.0), px(500.0)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                titlebar: Some(TitlebarOptions {
                    title: Some(SharedString::from("Minime")),
                    appears_transparent: false,
                    ..Default::default()
                }),
                window_background: WindowBackgroundAppearance::Opaque,
                window_min_size: Some(size(px(620.0), px(500.0))),
                app_id: Some(application_id().into()),
                ..Default::default()
            },
            |window, cx| cx.new(|cx| MinimeApp::new(window, cx)),
        )
        .expect("Impossible d’ouvrir la fenêtre Minime");

        #[cfg(target_os = "macos")]
        cx.bind_keys([
            KeyBinding::new("cmd-o", OpenFiles, None),
            KeyBinding::new("cmd-enter", CompressNow, None),
            KeyBinding::new("cmd-shift-k", ClearQueue, None),
        ]);
        #[cfg(not(target_os = "macos"))]
        cx.bind_keys([
            KeyBinding::new("ctrl-o", OpenFiles, None),
            KeyBinding::new("ctrl-enter", CompressNow, None),
            KeyBinding::new("ctrl-shift-k", ClearQueue, None),
        ]);
        cx.activate(true);
    });
}
