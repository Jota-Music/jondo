use crate::shared::ui::views::design::theme;

use std::time::Duration;

use gpui::{Animation, AnimationExt, ElementId, IntoElement, Radians, Rgba, SharedString};

use crate::shared::i18n::t;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Icon {
    ArrowLeft,
    ArrowRight,
    ArrowUpDown,
    GripVertical,
    Library,
    ListMusic,
    Settings,
    Search,
    SearchBold,
    RefreshCw,
    X,
    ChevronUp,
    ChevronDown,
    Check,
    CheckBold,
    Users,
    Heart,
    HeartFill,
    ArrowUpFromLine,
    Play,
    Pause,
    PlayLine,
    PauseLine,
    ListPlus,
    ListChecks,
    Square,
    SquarePlus,
    SquareCheck,
    SkipBack,
    SkipForward,
    Shuffle,
    Repeat,
    Repeat1,
    Volume,
    Volume1,
    Volume2,
    VolumeX,
    EllipsisVertical,
    Plus,
    Trash2,
    Pencil,
    Edit2,
    Link2,
    Minus,
    Minimize2,
    UserRound,
    Info,
    Copy,
    Pin,
    Disc3,
    Music,
    List,
    LayoutGrid,
    CirclePlay,
    CircleAlert,
    TriangleAlert,
    Loader,
    Turntable,
    Wifi,
    WifiOff,
    Lock,
    LogIn,
    LogOut,
    PowerOff,
    BookmarkPlus,
    BookmarkCheck,
    Unplug,
    Spotify,
    YouTube,
    Languages,
    Headphones,
    Moon,
    ArchiveRestore,
    FileDown,
    Download,
    Upload,
    ExternalLink,
    Save,
    FolderSync,
    RotateCcw,
    Github,
    CloudDownload,
    CloudUpload,
    Smartphone,
    ShieldAlert,
    ChevronRight,
    Tv,
}

pub const ARROW_LEFT: Icon = Icon::ArrowLeft;
pub const ARROW_RIGHT: Icon = Icon::ArrowRight;
pub const ARROW_UP_DOWN: Icon = Icon::ArrowUpDown;
pub const GRIP_VERTICAL: Icon = Icon::GripVertical;
pub const LIBRARY: Icon = Icon::Library;
pub const LIST_MUSIC: Icon = Icon::ListMusic;
pub const SETTINGS: Icon = Icon::Settings;
pub const SEARCH: Icon = Icon::Search;
pub const SEARCH_BOLD: Icon = Icon::SearchBold;
pub const REFRESH_CW: Icon = Icon::RefreshCw;
pub const X: Icon = Icon::X;
pub const CHEVRON_UP: Icon = Icon::ChevronUp;
pub const CHEVRON_DOWN: Icon = Icon::ChevronDown;
pub const CHECK: Icon = Icon::Check;
pub const CHECK_BOLD: Icon = Icon::CheckBold;
pub const USERS: Icon = Icon::Users;
pub const HEART: Icon = Icon::Heart;
pub const HEART_FILL: Icon = Icon::HeartFill;
pub const ARROW_UP_FROM_LINE: Icon = Icon::ArrowUpFromLine;
pub const PLAY: Icon = Icon::Play;
pub const PAUSE: Icon = Icon::Pause;
pub const PLAY_LINE: Icon = Icon::PlayLine;
pub const PAUSE_LINE: Icon = Icon::PauseLine;
pub const LIST_PLUS: Icon = Icon::ListPlus;
pub const LIST_CHECKS: Icon = Icon::ListChecks;
pub const SQUARE: Icon = Icon::Square;
pub const SQUARE_PLUS: Icon = Icon::SquarePlus;
pub const SQUARE_CHECK: Icon = Icon::SquareCheck;
pub const SKIP_BACK: Icon = Icon::SkipBack;
pub const SKIP_FORWARD: Icon = Icon::SkipForward;
pub const SHUFFLE: Icon = Icon::Shuffle;
pub const REPEAT: Icon = Icon::Repeat;
pub const REPEAT_1: Icon = Icon::Repeat1;
pub const VOLUME: Icon = Icon::Volume;
pub const VOLUME_1: Icon = Icon::Volume1;
pub const VOLUME_2: Icon = Icon::Volume2;
pub const VOLUME_X: Icon = Icon::VolumeX;
pub const ELLIPSIS_VERTICAL: Icon = Icon::EllipsisVertical;
pub const PLUS: Icon = Icon::Plus;
pub const TRASH_2: Icon = Icon::Trash2;
pub const PENCIL: Icon = Icon::Pencil;
pub const EDIT_2: Icon = Icon::Edit2;
pub const LINK_2: Icon = Icon::Link2;
pub const MINUS: Icon = Icon::Minus;
pub const MINIMIZE_2: Icon = Icon::Minimize2;
pub const COPY: Icon = Icon::Copy;
pub const PIN: Icon = Icon::Pin;
pub const DISC_3: Icon = Icon::Disc3;
pub const MUSIC: Icon = Icon::Music;
pub const LIST: Icon = Icon::List;
pub const LAYOUT_GRID: Icon = Icon::LayoutGrid;
pub const CIRCLE_PLAY: Icon = Icon::CirclePlay;
pub const CIRCLE_ALERT: Icon = Icon::CircleAlert;
pub const TRIANGLE_ALERT: Icon = Icon::TriangleAlert;
pub const LOADER: Icon = Icon::Loader;
pub const TURNTABLE: Icon = Icon::Turntable;
pub const WIFI: Icon = Icon::Wifi;
pub const WIFI_OFF: Icon = Icon::WifiOff;
pub const LOCK: Icon = Icon::Lock;
pub const LOG_IN: Icon = Icon::LogIn;
pub const LOG_OUT: Icon = Icon::LogOut;
pub const POWER_OFF: Icon = Icon::PowerOff;
pub const BOOKMARK_PLUS: Icon = Icon::BookmarkPlus;
pub const BOOKMARK_CHECK: Icon = Icon::BookmarkCheck;
pub const UNPLUG: Icon = Icon::Unplug;
pub const USER_ROUND: Icon = Icon::UserRound;
pub const TV: Icon = Icon::Tv;

impl Icon {
    pub fn name(self) -> &'static str {
        match self {
            Self::ArrowLeft => "arrow-left",
            Self::ArrowRight => "arrow-right",
            Self::ArrowUpDown => "arrow-up-down",
            Self::GripVertical => "grip-vertical",
            Self::Library => "library",
            Self::ListMusic => "list-music",
            Self::Settings => "settings",
            Self::Search => "search",
            Self::SearchBold => "search-bold",
            Self::RefreshCw => "refresh-cw",
            Self::X => "x",
            Self::ChevronUp => "chevron-up",
            Self::ChevronDown => "chevron-down",
            Self::Check => "check",
            Self::CheckBold => "check-bold",
            Self::Users => "users",
            Self::Heart => "heart",
            Self::HeartFill => "heart-fill",
            Self::ArrowUpFromLine => "arrow-up-from-line",
            Self::Play => "play",
            Self::Pause => "pause",
            Self::PlayLine => "play-line",
            Self::PauseLine => "pause-line",
            Self::ListPlus => "list-plus",
            Self::ListChecks => "list-checks",
            Self::Square => "square",
            Self::SquarePlus => "square-plus",
            Self::SquareCheck => "square-check",
            Self::SkipBack => "skip-back",
            Self::SkipForward => "skip-forward",
            Self::Shuffle => "shuffle",
            Self::Repeat => "repeat",
            Self::Repeat1 => "repeat-1",
            Self::Volume => "volume",
            Self::Volume1 => "volume-1",
            Self::Volume2 => "volume-2",
            Self::VolumeX => "volume-x",
            Self::EllipsisVertical => "ellipsis-vertical",
            Self::Plus => "plus",
            Self::Trash2 => "trash-2",
            Self::Pencil => "pencil",
            Self::Edit2 => "edit-2",
            Self::Link2 => "link-2",
            Self::Minus => "minus",
            Self::Minimize2 => "minimize-2",
            Self::UserRound => "user-round",
            Self::Info => "info",
            Self::Copy => "copy",
            Self::Pin => "pin",
            Self::Disc3 => "disc-3",
            Self::Music => "music",
            Self::List => "list",
            Self::LayoutGrid => "layout-grid",
            Self::CirclePlay => "circle-play",
            Self::CircleAlert => "circle-alert",
            Self::TriangleAlert => "triangle-alert",
            Self::Loader => "loader",
            Self::Turntable => "turntable",
            Self::Wifi => "wifi",
            Self::WifiOff => "wifi-off",
            Self::Lock => "lock",
            Self::LogIn => "log-in",
            Self::LogOut => "log-out",
            Self::PowerOff => "power-off",
            Self::BookmarkPlus => "bookmark-plus",
            Self::BookmarkCheck => "bookmark-check",
            Self::Unplug => "unplug",
            Self::Spotify => "spotify",
            Self::YouTube => "youtube",
            Self::Languages => "languages",
            Self::Headphones => "headphones",
            Self::Moon => "moon",
            Self::ArchiveRestore => "archive-restore",
            Self::FileDown => "file-down",
            Self::Download => "download",
            Self::Upload => "upload",
            Self::ExternalLink => "external-link",
            Self::Save => "save",
            Self::FolderSync => "folder-sync",
            Self::RotateCcw => "rotate-ccw",
            Self::Github => "github",
            Self::CloudDownload => "cloud-download",
            Self::CloudUpload => "cloud-upload",
            Self::Smartphone => "smartphone",
            Self::ShieldAlert => "shield-alert",
            Self::ChevronRight => "chevron-right",
            Self::Tv => "tv",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::ArrowLeft => t("Back"),
            Self::ArrowRight => t("Forward"),
            Self::ArrowUpDown => t("Sort"),
            Self::GripVertical => t("Reorder"),
            Self::Library => t("Playlists"),
            Self::ListMusic => t("Queue"),
            Self::Settings => t("Settings"),
            Self::Search => t("Search"),
            Self::SearchBold => t("Search"),
            Self::RefreshCw => t("Refresh"),
            Self::X => t("Close"),
            Self::ChevronUp => t("Expand"),
            Self::ChevronDown => t("Move down"),
            Self::Check => t("Selected"),
            Self::CheckBold => t("Selected"),
            Self::Users => t("Artists"),
            Self::Heart => t("Follow"),
            Self::HeartFill => t("Following"),
            Self::ArrowUpFromLine => t("Move below current"),
            Self::Play | Self::PlayLine => t("Play"),
            Self::Pause | Self::PauseLine => t("Pause"),
            Self::ListPlus => t("Add to queue"),
            Self::ListChecks => t("Select all"),
            Self::Square => t("Deselect all"),
            Self::SquarePlus => t("Add to playlist"),
            Self::SquareCheck => t("Select"),
            Self::SkipBack => t("Previous"),
            Self::SkipForward => t("Next"),
            Self::Shuffle => t("Shuffle"),
            Self::Repeat | Self::Repeat1 => t("Repeat"),
            Self::Volume | Self::Volume1 | Self::Volume2 => t("Volume"),
            Self::VolumeX => t("Mute"),
            Self::EllipsisVertical => t("More"),
            Self::Plus => t("Add"),
            Self::Trash2 => t("Remove"),
            Self::Pencil => t("Rename"),
            Self::Edit2 => t("Change audio"),
            Self::Link2 => t("Assign audio by ID or URL"),
            Self::Minus => t("Minimize"),
            Self::Minimize2 => t("Compact player"),
            Self::UserRound => t("Account"),
            Self::Info => t("Info"),
            Self::Copy => t("Maximize"),
            Self::Pin => t("Pin"),
            Self::Disc3 | Self::Music => t("Music"),
            Self::List => t("List view"),
            Self::LayoutGrid => t("Grid view"),
            Self::CirclePlay => t("Play"),
            Self::CircleAlert => t("Error"),
            Self::TriangleAlert => t("Warning"),
            Self::Loader => t("Loading"),
            Self::Turntable => t("Rooms"),
            Self::Wifi => t("Connected"),
            Self::WifiOff => t("Offline"),
            Self::Lock => t("Locked"),
            Self::LogIn => t("Connect"),
            Self::LogOut => t("Leave room"),
            Self::PowerOff => t("Offline"),
            Self::BookmarkPlus => t("Save room"),
            Self::BookmarkCheck => t("Saved"),
            Self::Unplug => t("Stop session"),
            Self::Spotify => "Spotify",
            Self::YouTube => "YouTube",
            Self::Languages => t("Language"),
            Self::Headphones => t("Discord"),
            Self::Moon => t("Background memory"),
            Self::ArchiveRestore => t("Backups"),
            Self::FileDown => t("Logs"),
            Self::Download => t("Download"),
            Self::Upload => t("Import"),
            Self::ExternalLink => t("Open release"),
            Self::Save => t("Save"),
            Self::FolderSync => t("Autosave"),
            Self::RotateCcw => t("Reload"),
            Self::Github => t("GitHub"),
            Self::CloudDownload => t("Open from the cloud"),
            Self::CloudUpload => t("Create in the cloud"),
            Self::Smartphone => t("Phone folder"),
            Self::ShieldAlert => t("Risk"),
            Self::ChevronRight => t("Expand"),
            Self::Tv => t("Channel"),
        }
    }

    fn svg(self) -> &'static str {
        match self {
            Self::ArrowLeft => include_str!("../../../../../assets/icons/arrow-left.svg"),
            Self::ArrowRight => include_str!("../../../../../assets/icons/arrow-right.svg"),
            Self::ArrowUpDown => include_str!("../../../../../assets/icons/arrow-up-down.svg"),
            Self::GripVertical => include_str!("../../../../../assets/icons/grip-vertical.svg"),
            Self::Library => include_str!("../../../../../assets/icons/library.svg"),
            Self::ListMusic => include_str!("../../../../../assets/icons/list-music.svg"),
            Self::Settings => include_str!("../../../../../assets/icons/settings.svg"),
            Self::Search => include_str!("../../../../../assets/icons/search.svg"),
            Self::SearchBold => include_str!("../../../../../assets/icons/search-bold.svg"),
            Self::RefreshCw => include_str!("../../../../../assets/icons/refresh-cw.svg"),
            Self::X => include_str!("../../../../../assets/icons/x.svg"),
            Self::ChevronUp => include_str!("../../../../../assets/icons/chevron-up.svg"),
            Self::ChevronDown => include_str!("../../../../../assets/icons/chevron-down.svg"),
            Self::Check => include_str!("../../../../../assets/icons/check.svg"),
            Self::CheckBold => include_str!("../../../../../assets/icons/check-bold.svg"),
            Self::Users => include_str!("../../../../../assets/icons/users.svg"),
            Self::Heart => include_str!("../../../../../assets/icons/heart.svg"),
            Self::HeartFill => include_str!("../../../../../assets/icons/heart-fill.svg"),
            Self::ArrowUpFromLine => {
                include_str!("../../../../../assets/icons/arrow-up-from-line.svg")
            }
            Self::Play => include_str!("../../../../../assets/icons/play-fill.svg"),
            Self::Pause => include_str!("../../../../../assets/icons/pause-fill.svg"),
            Self::PlayLine => include_str!("../../../../../assets/icons/play-line.svg"),
            Self::PauseLine => include_str!("../../../../../assets/icons/pause-line.svg"),
            Self::ListPlus => include_str!("../../../../../assets/icons/list-plus.svg"),
            Self::ListChecks => include_str!("../../../../../assets/icons/list-checks.svg"),
            Self::Square => include_str!("../../../../../assets/icons/square.svg"),
            Self::SquarePlus => include_str!("../../../../../assets/icons/square-plus.svg"),
            Self::SquareCheck => include_str!("../../../../../assets/icons/square-check.svg"),
            Self::SkipBack => include_str!("../../../../../assets/icons/skip-back-fill.svg"),
            Self::SkipForward => include_str!("../../../../../assets/icons/skip-forward-fill.svg"),
            Self::Shuffle => include_str!("../../../../../assets/icons/shuffle.svg"),
            Self::Repeat => include_str!("../../../../../assets/icons/repeat.svg"),
            Self::Repeat1 => include_str!("../../../../../assets/icons/repeat-1.svg"),
            Self::Volume => include_str!("../../../../../assets/icons/volume.svg"),
            Self::Volume1 => include_str!("../../../../../assets/icons/volume-1.svg"),
            Self::Volume2 => include_str!("../../../../../assets/icons/volume-2.svg"),
            Self::VolumeX => include_str!("../../../../../assets/icons/volume-x.svg"),
            Self::EllipsisVertical => {
                include_str!("../../../../../assets/icons/ellipsis-vertical.svg")
            }
            Self::Plus => include_str!("../../../../../assets/icons/plus.svg"),
            Self::Trash2 => include_str!("../../../../../assets/icons/trash-2.svg"),
            Self::Pencil => include_str!("../../../../../assets/icons/pencil.svg"),
            Self::Edit2 => include_str!("../../../../../assets/icons/edit-2.svg"),
            Self::Link2 => include_str!("../../../../../assets/icons/link-2.svg"),
            Self::Minus => include_str!("../../../../../assets/icons/minus.svg"),
            Self::Minimize2 => include_str!("../../../../../assets/icons/minimize-2.svg"),
            Self::UserRound => include_str!("../../../../../assets/icons/user-round.svg"),
            Self::Info => include_str!("../../../../../assets/icons/info.svg"),
            Self::Copy => include_str!("../../../../../assets/icons/copy.svg"),
            Self::Pin => include_str!("../../../../../assets/icons/pin.svg"),
            Self::Disc3 => include_str!("../../../../../assets/icons/disc-3.svg"),
            Self::Music => include_str!("../../../../../assets/icons/music.svg"),
            Self::List => include_str!("../../../../../assets/icons/list.svg"),
            Self::LayoutGrid => include_str!("../../../../../assets/icons/layout-grid.svg"),
            Self::CirclePlay => include_str!("../../../../../assets/icons/circle-play.svg"),
            Self::CircleAlert => include_str!("../../../../../assets/icons/circle-alert.svg"),
            Self::TriangleAlert => include_str!("../../../../../assets/icons/triangle-alert.svg"),
            Self::Loader => include_str!("../../../../../assets/icons/loader.svg"),
            Self::Turntable => include_str!("../../../../../assets/icons/turntable.svg"),
            Self::Wifi => include_str!("../../../../../assets/icons/wifi.svg"),
            Self::WifiOff => include_str!("../../../../../assets/icons/wifi-off.svg"),
            Self::Lock => include_str!("../../../../../assets/icons/lock.svg"),
            Self::LogIn => include_str!("../../../../../assets/icons/log-in.svg"),
            Self::LogOut => include_str!("../../../../../assets/icons/log-out.svg"),
            Self::PowerOff => include_str!("../../../../../assets/icons/power-off.svg"),
            Self::BookmarkPlus => include_str!("../../../../../assets/icons/bookmark-plus.svg"),
            Self::BookmarkCheck => include_str!("../../../../../assets/icons/bookmark-check.svg"),
            Self::Unplug => include_str!("../../../../../assets/icons/unplug.svg"),
            Self::Spotify => include_str!("../../../../../assets/icons/spotify.svg"),
            Self::YouTube => include_str!("../../../../../assets/icons/youtube.svg"),
            Self::Languages => include_str!("../../../../../assets/icons/languages.svg"),
            Self::Headphones => include_str!("../../../../../assets/icons/headphones.svg"),
            Self::Moon => include_str!("../../../../../assets/icons/moon.svg"),
            Self::ArchiveRestore => include_str!("../../../../../assets/icons/archive-restore.svg"),
            Self::FileDown => include_str!("../../../../../assets/icons/file-down.svg"),
            Self::Download => include_str!("../../../../../assets/icons/download.svg"),
            Self::Upload => include_str!("../../../../../assets/icons/upload.svg"),
            Self::ExternalLink => include_str!("../../../../../assets/icons/external-link.svg"),
            Self::Save => include_str!("../../../../../assets/icons/save.svg"),
            Self::FolderSync => include_str!("../../../../../assets/icons/folder-sync.svg"),
            Self::RotateCcw => include_str!("../../../../../assets/icons/rotate-ccw.svg"),
            Self::Github => include_str!("../../../../../assets/icons/github.svg"),
            Self::CloudDownload => include_str!("../../../../../assets/icons/cloud-download.svg"),
            Self::CloudUpload => include_str!("../../../../../assets/icons/cloud-upload.svg"),
            Self::Smartphone => include_str!("../../../../../assets/icons/smartphone.svg"),
            Self::ShieldAlert => include_str!("../../../../../assets/icons/shield-alert.svg"),
            Self::ChevronRight => include_str!("../../../../../assets/icons/chevron-right.svg"),
            Self::Tv => include_str!("../../../../../assets/icons/tv.svg"),
        }
    }
}

pub fn icon(icon: Icon, size: f32, color: Rgba) -> gpui::Svg {
    use gpui::Styled as _;
    gpui::svg()
        .path(SharedString::new_static(icon.name()))
        .data(icon.svg().as_bytes())
        .text_color(color)
        .w(theme::unit(size))
        .h(theme::unit(size))
        .flex_none()
}

pub fn spinning(glyph: Icon, size: f32, color: Rgba) -> impl IntoElement {
    icon(glyph, size, color).with_animation(
        ElementId::Name(format!("spin-{}", glyph.name()).into()),
        Animation::new(Duration::from_secs(1))
            .repeat()
            .with_max_fps(60.0),
        |glyph, delta| {
            glyph.with_transformation(gpui::Transformation::rotate(Radians(
                delta * std::f32::consts::TAU,
            )))
        },
    )
}
