use std::{
    fmt::{self, Display},
    sync::atomic::{AtomicU8, Ordering},
};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Locale {
    #[default]
    English,
    Japanese,
}

impl Locale {
    pub const ALL: [Self; 2] = [Self::English, Self::Japanese];

    pub const fn id(self) -> &'static str {
        match self {
            Self::English => "english",
            Self::Japanese => "japanese",
        }
    }
}

impl Display for Locale {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::English => "English",
            Self::Japanese => "日本語",
        })
    }
}

macro_rules! catalog {
    ($($key:ident => $english:literal, $japanese:literal;)+) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub enum Text {
            $($key,)+
        }

        const fn catalog_text(locale: Locale, key: Text) -> &'static str {
            match (locale, key) {
                $((Locale::English, Text::$key) => $english,)+
                $((Locale::Japanese, Text::$key) => $japanese,)+
            }
        }
    };
}

catalog! {
    Telemetry => "Telemetry", "テレメトリー";
    CarSetup => "Car setup", "車両セットアップ";
    Settings => "Settings", "設定";
    Language => "Language", "言語";
    Diagnostics => "Diagnostics", "診断";
    ShowDiagnostics => "Show diagnostics", "診断情報を表示";
    DiagnosticsDescription => "Expose live telemetry and recording details for troubleshooting.", "トラブルシューティング用にライブテレメトリーと記録の詳細を表示します。";
    Runtime => "Runtime", "実行状態";
    ApplicationPreferences => "Application preferences and diagnostics", "アプリケーション設定と診断情報";
    ComponentPreview => "Component preview", "コンポーネントプレビュー";
    ComponentPreviewDescription => "Open the shared modal to verify its layout and dismissal behavior.", "共通モーダルのレイアウトと閉じる操作を確認します。";
    OpenDialogPreview => "Open dialog preview", "ダイアログプレビューを開く";
    About => "About", "このアプリについて";
    DesktopTelemetryInterface => "Desktop telemetry interface", "デスクトップテレメトリーインターフェース";
    Cancel => "Cancel", "キャンセル";
    Confirm => "Confirm", "確認";
    DialogPreview => "Dialog preview", "ダイアログプレビュー";
    DialogPreviewDescription => "A live verification of the shared Chiaro dialog component.", "共通の Chiaro ダイアログコンポーネントをライブで確認できます。";
    CloseDialog => "Close dialog", "ダイアログを閉じる";
    DialogPreviewBody => "The Settings view stays mounted behind this surface.", "この画面の背後では設定ビューが表示されたままです。";
    DialogPreviewInstructions => "Verify the close icon, Cancel button, Escape key, and backdrop click.", "閉じるアイコン、キャンセル、Escape キー、背景クリックを確認してください。";
    DialogPreviewNavigation => "Navigation and window controls should remain inactive while the dialog is open.", "ダイアログ表示中はナビゲーションとウィンドウ操作は無効になります。";
    IbtRecording => "IBT recording", "IBT 記録";
    UnavailableOnThisPlatform => "Unavailable on this platform", "このプラットフォームでは利用できません";
    Disconnected => "Disconnected", "未接続";
    Connecting => "Connecting", "接続中";
    Connected => "Connected", "接続済み";
    Waiting => "Waiting", "待機中";
    Live => "Live", "ライブ";
    Offline => "Offline", "オフライン";
    Unavailable => "Unavailable", "利用不可";
    Connect => "Connect", "接続";
    Disconnect => "Disconnect", "切断";
    OpenIbt => "Open IBT", "IBT を開く";
    Selecting => "Selecting...", "選択中...";
    Loading => "Loading...", "読み込み中...";
    ClearReference => "Clear reference", "参照データを消去";
    NoReferenceLoaded => "No reference loaded", "参照データが読み込まれていません";
    ReferenceIbtUnavailable => "Reference IBT unavailable", "参照 IBT を利用できません";
    ReferenceSetup => "Reference setup", "参照セットアップ";
    ReferenceSetupUnavailable => "The reference IBT does not contain setup data.", "参照 IBT にセットアップデータがありません。";
    MainIbtRequired => "Main IBT required", "メイン IBT が必要です";
    TrackMismatch => "Track mismatch", "トラックが一致しません";
    CarMismatch => "Car mismatch", "車両が一致しません";
    WaitingForTrackData => "Waiting for track data", "トラックデータを待機しています";
    Session => "Session", "セッション";
    Reference => "Reference", "参照";
    MyLaps => "My laps", "マイラップ";
    Charts => "Charts", "チャート";
    SessionLaps => "Session laps", "セッションラップ";
    Timing => "Timing", "タイミング";
    Laps => "Laps", "ラップ";
    Stint => "Stint", "スティント";
    Stints => "Stints", "スティント";
    Sectors => "Sectors", "セクター";
    Best => "Best", "ベスト";
    Average => "Average", "平均";
    Complete => "Complete", "完了";
    NoSectorData => "No sector timing data", "セクタータイムがありません";
    NoStintData => "No stint data", "スティントデータがありません";
    Layout => "Layout", "レイアウト";
    ResetLayout => "Reset layout", "レイアウトをリセット";
    Conditions => "CONDITIONS", "コンディション";
    Car => "Car", "車両";
    Class => "Class", "クラス";
    Time => "Time", "時間";
    Date => "Date", "日付";
    Length => "Length", "全長";
    Type => "Type", "種類";
    Source => "Source", "ソース";
    Weather => "Weather", "天候";
    AirTemperature => "Air temp", "気温";
    TrackTemperature => "Track temp", "路面温度";
    Humidity => "Humidity", "湿度";
    Wind => "Wind", "風";
    LapPosition => "Lap position", "ラップ位置";
    Speed => "Speed", "速度";
    Pedal => "Pedal", "ペダル";
    Throttle => "Throttle", "スロットル";
    Brake => "Brake", "ブレーキ";
    BrakePressure => "Brake pressure", "ブレーキ圧";
    AbsActivity => "ABS activity", "ABS 作動";
    AbsActive => "ABS active", "ABS 作動中";
    Steering => "Steering", "ステアリング";
    SteeringAngle => "Steering angle", "ステアリング角";
    SteeringTorque => "Steering torque", "ステアリングトルク";
    EngineRpm => "Engine RPM", "エンジン回転数";
    Rpm => "RPM", "RPM";
    Gear => "Gear", "ギア";
    VehicleDynamics => "Vehicle dynamics", "車両ダイナミクス";
    Dynamics => "Dynamics", "ダイナミクス";
    LateralG => "Lateral G", "横 G";
    LongitudinalG => "Longitudinal G", "縦 G";
    YawRate => "Yaw rate", "ヨーレート";
    WheelSlip => "Wheel slip", "ホイールスリップ";
    TyreTemperature => "Tyre temperature", "タイヤ温度";
    SuspensionTravel => "Suspension travel", "サスペンション";
    FuelUsed => "Fuel used", "燃料消費";
    Delta => "Delta", "デルタ";
    Fuel => "Fuel", "燃料";
    CurrentLap => "Current lap", "現在のラップ";
    LastLap => "Last lap", "前回のラップ";
    Position => "Position", "順位";
    Travel => "Travel", "ストローク";
    Slip => "slip", "スリップ";
    NotAvailable => "N/A", "該当なし";
    Cursor => "Cursor", "カーソル";
    ReferenceCursor => "Reference cursor", "参照カーソル";
    Vehicle => "Vehicle", "車両";
    Inputs => "Inputs", "操作入力";
    Tyres => "Tyres", "タイヤ";
    Wheels => "Wheels", "ホイール";
    SingleColumn => "Single column", "1 列";
    TwoColumns => "Two columns", "2 列";
    Front => "Front", "フロント";
    Rear => "Rear", "リア";
    FrontLeft => "Front left", "左フロント";
    FrontRight => "Front right", "右フロント";
    RearLeft => "Rear left", "左リア";
    RearRight => "Rear right", "右リア";
    CarcassPressure => "Carcass I / M / O · hot pressure", "カーカス 内側 / 中央 / 外側・温間空気圧";
    ReferenceSpeed => "Reference speed", "参照速度";
    ReferenceThrottle => "Reference throttle", "参照スロットル";
    ReferenceBrake => "Reference brake", "参照ブレーキ";
    ReferenceSteering => "Reference steering", "参照ステアリング";
    ReferenceSteeringTorque => "Reference steering torque", "参照ステアリングトルク";
    ReferenceRpm => "Reference RPM", "参照 RPM";
    ReferenceGear => "Reference gear", "参照ギア";
    ReferenceLateralG => "Reference lateral G", "参照横 G";
    ReferenceLongitudinalG => "Reference longitudinal G", "参照縦 G";
    ReferenceYawRate => "Reference yaw rate", "参照ヨーレート";
    ReferenceFuelUsed => "Reference fuel used", "参照燃料消費";
    ReferenceFrontLeft => "Reference front left", "参照・左フロント";
    ReferenceFrontRight => "Reference front right", "参照・右フロント";
    ReferenceRearLeft => "Reference rear left", "参照・左リア";
    ReferenceRearRight => "Reference rear right", "参照・右リア";
    LapDistance => "Lap distance", "ラップ距離";
    Active => "Active", "作動";
    Off => "Off", "オフ";
    Unlimited => "Unlimited", "無制限";
    Setup => "Setup", "セットアップ";
    SetupData => "Setup data", "セットアップデータ";
    WaitingForCarData => "Waiting for car data", "車両データを待機しています";
    SetupUnavailable => "Setup unavailable", "セットアップを利用できません";
    SetupSectionUnavailable => "This setup section is no longer available.", "このセットアップ項目は現在利用できません。";
    VehicleSpecifications => "Vehicle specifications", "車両仕様";
    Version => "Version", "バージョン";
    Powertrain => "Powertrain", "パワートレイン";
    Transmission => "Transmission", "トランスミッション";
    ShiftLights => "Shift lights", "シフトライト";
    FuelSystem => "Fuel system", "燃料システム";
    TyreCompounds => "Tyre compounds", "タイヤコンパウンド";
    Regulations => "Regulations", "レギュレーション";
    SetupRules => "Setup rules", "セットアップ規則";
    FixedSetup => "Fixed setup", "固定セットアップ";
    OpenSetup => "Open setup", "オープンセットアップ";
    FuelAllowance => "Fuel allowance", "燃料使用制限";
    WeightPenalty => "Weight penalty", "重量ペナルティ";
    PowerAdjustment => "Power adjustment", "出力調整";
    DryTyreSetLimit => "Dry tyre set limit", "ドライタイヤセット上限";
    Electric => "Electric", "電動";
    Combustion => "Combustion", "内燃機関";
    Access => "Access", "アクセス";
    ReadOnly => "Read only", "読み取り専用";
    LoadType => "Load type", "読み込み形式";
    CarPath => "Car path", "車両パス";
    Revision => "Revision", "リビジョン";
    Modified => "Modified", "変更済み";
    Saved => "Saved", "保存済み";
    TechPassed => "Tech passed", "車検合格";
    TechFailed => "Tech failed", "車検不合格";
    CornerComparison => "Corner comparison", "コーナー比較";
    Setting => "Setting", "設定項目";
    Current => "Current", "現在";
    Difference => "Difference", "差分";
    Changed => "Changed", "変更あり";
    Unchanged => "Unchanged", "変更なし";
    CurrentOnly => "Current only", "現在側のみ";
    ReferenceOnly => "Reference only", "参照側のみ";
    ChangedOnly => "Changed values only", "変更項目のみ";
    NoSetupDifferences => "No setup differences", "セットアップの差分はありません";
    NoSetupDifferencesDescription => "The visible values match the reference setup.", "表示対象の値は参照セットアップと一致しています。";
    CarSetupMismatch => "The reference setup belongs to a different car.", "参照セットアップの車両が異なります。";
    NoSetupDataYet => "No setup data yet", "セットアップデータがまだありません";
    NoSetupDataDescription => "Connect to a live telemetry source or load an IBT recording to inspect the player car setup.", "ライブテレメトリーへ接続するか IBT 記録を読み込むと、プレイヤー車両のセットアップを確認できます。";
    SetupUnavailableDescription => "This session does not publish CarSetup data. It can be absent while spectating, replaying, or changing sessions.", "このセッションは CarSetup データを公開していません。観戦中、リプレイ中、セッション切替中には存在しないことがあります。";
    SetupCouldNotBeRead => "Setup could not be read", "セットアップを読み取れませんでした";
    SessionInfoInvalidYaml => "The session information is not valid YAML", "セッション情報が有効な YAML ではありません";
    NoSetupValues => "No setup values", "セットアップ値がありません";
    NoSetupValuesDescription => "iRacing published a CarSetup section, but it contains no displayable values.", "iRacing は CarSetup セクションを公開しましたが、表示できる値がありません。";
    Value => "Value", "値";
    General => "General", "一般";
    Yes => "Yes", "はい";
    No => "No", "いいえ";
    HideTooltips => "Hide tooltips", "ツールチップを非表示";
    ShowTooltips => "Show tooltips", "ツールチップを表示";
    HideSectors => "Hide sectors", "セクター表示を非表示";
    ShowSectors => "Show sectors", "セクター表示を表示";
    RestoreChart => "Restore chart", "チャートを元に戻す";
    MaximizeChart => "Maximize chart", "チャートを最大化";
    RestoreCard => "Restore card", "カードを元に戻す";
    MaximizeCard => "Maximize card", "カードを最大化";
    DragToReorder => "Drag to reorder", "ドラッグして並べ替え";
    Minimize => "Minimize", "最小化";
    Maximize => "Maximize", "最大化";
    Close => "Close", "閉じる";
    OpenChiaroscuro => "Open Chiaroscuro", "Chiaroscuro を開く";
    Quit => "Quit", "終了";
    OpenIRacingTelemetry => "Open iRacing telemetry", "iRacing テレメトリーを開く";
    IRacingTelemetry => "iRacing telemetry", "iRacing テレメトリー";
    IRacingOnThisPc => "iRacing on this PC", "この PC の iRacing";
    LiveWindowsOnly => "Live iRacing telemetry is available only on Windows; IBT recordings remain available.", "ライブ iRacing テレメトリーは Windows でのみ利用できます。IBT 記録は引き続き利用できます。";
    ConfigurationError => "Configuration error", "設定エラー";
    LiveSource => "Live source", "ライブソース";
    Connection => "Connection", "接続状態";
    SamplesReceived => "Samples received", "受信サンプル数";
    Records => "Records", "レコード数";
    TelemetryVariables => "Telemetry variables", "テレメトリー変数";
    LiveSourceUnavailable => "Live source unavailable", "ライブソースを利用できません";
    IbtSource => "IBT source", "IBT ソース";
    SessionInfoUpdate => "Session info update", "セッション情報更新回数";
    TelemetryError => "Telemetry error", "テレメトリーエラー";
    FailedToLoadDesktopSettings => "Failed to load desktop settings", "デスクトップ設定を読み込めませんでした";
    FailedToSaveDesktopSettings => "Failed to save desktop settings", "デスクトップ設定を保存できませんでした";
    ConfigLocationUnavailable => "Neither XDG_CONFIG_HOME, APPDATA nor HOME is available", "XDG_CONFIG_HOME、APPDATA、HOME のいずれも利用できません";
    ConfigPathHasNoParent => "Desktop settings path has no parent directory", "デスクトップ設定パスに親ディレクトリがありません";
    UnknownTrack => "Unknown track", "不明なトラック";
}

static CURRENT_LOCALE: AtomicU8 = AtomicU8::new(Locale::English as u8);

pub fn set_locale(locale: Locale) {
    CURRENT_LOCALE.store(locale as u8, Ordering::Relaxed);
}

pub fn current_locale() -> Locale {
    match CURRENT_LOCALE.load(Ordering::Relaxed) {
        value if value == Locale::Japanese as u8 => Locale::Japanese,
        _ => Locale::English,
    }
}

pub fn tr(key: Text) -> &'static str {
    catalog_text(current_locale(), key)
}

#[derive(Debug, Clone, Copy)]
pub struct Translations(Locale);

impl Translations {
    pub const fn new(locale: Locale) -> Self {
        Self(locale)
    }

    pub const fn locale(self) -> Locale {
        self.0
    }

    pub const fn get(self, key: Text) -> &'static str {
        catalog_text(self.0, key)
    }
}

pub fn count_laps(count: usize) -> String {
    match current_locale() {
        Locale::English if count == 1 => "1 lap".to_owned(),
        Locale::English => format!("{count} laps"),
        Locale::Japanese => format!("{count} ラップ"),
    }
}

pub fn count_samples(count: u64) -> String {
    match current_locale() {
        Locale::English if count == 1 => "1 sample".to_owned(),
        Locale::English => format!("{count} samples"),
        Locale::Japanese => format!("{count} サンプル"),
    }
}

pub fn count_turns(count: i32) -> String {
    match current_locale() {
        Locale::English if count == 1 => "1 turn".to_owned(),
        Locale::English => format!("{count} turns"),
        Locale::Japanese => format!("{count} コーナー"),
    }
}

pub fn expand_label(title: &str) -> String {
    match current_locale() {
        Locale::English => format!("Expand {title}"),
        Locale::Japanese => format!("{title}を展開"),
    }
}

pub fn collapse_label(title: &str) -> String {
    match current_locale() {
        Locale::English => format!("Collapse {title}"),
        Locale::Japanese => format!("{title}を折りたたむ"),
    }
}

pub fn cylinder_count(count: i32) -> String {
    match current_locale() {
        Locale::English => format!("{count} cyl"),
        Locale::Japanese => format!("{count} 気筒"),
    }
}

pub fn gear_count(count: i32) -> String {
    match current_locale() {
        Locale::English => format!("{count}-speed"),
        Locale::Japanese => format!("{count} 速"),
    }
}

pub fn idle_rpm(value: f64) -> String {
    match current_locale() {
        Locale::English => format!("{value:.0} rpm idle"),
        Locale::Japanese => format!("アイドル {value:.0} rpm"),
    }
}

pub fn redline_rpm(value: f64) -> String {
    match current_locale() {
        Locale::English => format!("{value:.0} rpm redline"),
        Locale::Japanese => format!("レッドライン {value:.0} rpm"),
    }
}

pub fn item_number(index: usize) -> String {
    match current_locale() {
        Locale::English => format!("Item {index}"),
        Locale::Japanese => format!("項目 {index}"),
    }
}

pub fn failed_to_open(source: &str, error: &str) -> String {
    match current_locale() {
        Locale::English => format!("Failed to open {source}: {error}"),
        Locale::Japanese => format!("{source} を開けませんでした: {error}"),
    }
}

pub fn no_telemetry_records(file_name: &str) -> String {
    match current_locale() {
        Locale::English => format!("{file_name} contains no telemetry records"),
        Locale::Japanese => format!("{file_name} にテレメトリーレコードがありません"),
    }
}

pub fn failed_to_read_record(record: usize, file_name: &str, error: &str) -> String {
    match current_locale() {
        Locale::English => {
            format!("Failed to read record {record} from {file_name}: {error}")
        },
        Locale::Japanese => {
            format!("{file_name} のレコード {record} を読み取れませんでした: {error}")
        },
    }
}

pub fn non_finite_record(record: usize, file_name: &str) -> String {
    match current_locale() {
        Locale::English => {
            format!("Record {record} in {file_name} contains a non-finite value")
        },
        Locale::Japanese => {
            format!("{file_name} のレコード {record} に非有限値が含まれています")
        },
    }
}

pub fn failed_to_read_last_frame(file_name: &str, error: &str) -> String {
    match current_locale() {
        Locale::English => {
            format!("Failed to read the last frame from {file_name}: {error}")
        },
        Locale::Japanese => {
            format!("{file_name} の最終フレームを読み取れませんでした: {error}")
        },
    }
}

/// Translates a human-readable label derived from an iRacing `CarSetup` YAML key.
///
/// iRacing can add car-specific keys that are not known at compile time. Exact
/// phrases cover the common UI vocabulary while the token fallback also gives
/// new compound keys a useful Japanese label.
pub fn setup_label(label: &str) -> String {
    setup_label_for(current_locale(), label)
}

pub fn setup_label_for(locale: Locale, label: &str) -> String {
    if locale == Locale::English {
        return label.to_owned();
    }

    if let Some(translated) = japanese_setup_phrase(label) {
        return translated.to_owned();
    }

    label
        .split_whitespace()
        .map(|word| japanese_setup_word(word).unwrap_or(word))
        .collect::<Vec<_>>()
        .join("・")
}

fn japanese_setup_phrase(label: &str) -> Option<&'static str> {
    Some(match label {
        "Tires Aero" | "Tyres Aero" => "タイヤ・エアロ",
        "Vehicle Specific Section" => "車両固有設定",
        "Tire Type" | "Tyre Type" => "タイヤ種別",
        "Starting Pressure" => "開始時空気圧",
        "Last Hot Pressure" => "直近の温間空気圧",
        "Last Temps OMI" => "直近温度 外側・中央・内側",
        "Last Temps IMO" => "直近温度 内側・中央・外側",
        "Cold Pressure" => "冷間空気圧",
        "Hot Pressure" => "温間空気圧",
        "Tread Remaining" => "トレッド残量",
        "Front ARB" => "フロント・スタビライザー",
        "Rear ARB" => "リア・スタビライザー",
        "Arb Setting" | "ARB Setting" => "スタビライザー設定",
        "Aero Balance" => "空力バランス",
        "Rear Wing" => "リアウイング",
        "Wing Angle" => "ウイング角度",
        "Ride Height" => "車高",
        "Front Ride Height" => "フロント車高",
        "Rear Ride Height" => "リア車高",
        "Brake Bias" => "ブレーキバイアス",
        "Brake Pressure" => "ブレーキ圧",
        "Brake Pressure Bias" => "ブレーキ圧バイアス",
        "Front Brake Pad Mu" => "フロントブレーキパッド摩擦係数",
        "Rear Brake Pad Mu" => "リアブレーキパッド摩擦係数",
        "Spring Rate" => "スプリングレート",
        "Spring Perch Offset" => "スプリングシートオフセット",
        "Bump Stiffness" => "バンプ剛性",
        "Rebound Stiffness" => "リバウンド剛性",
        "Slow Bump" => "低速バンプ",
        "Fast Bump" => "高速バンプ",
        "Slow Rebound" => "低速リバウンド",
        "Fast Rebound" => "高速リバウンド",
        "Differential Preload" | "Diff Preload" => "デフプリロード",
        "Fuel Level" => "燃料量",
        "Fuel Low Warning" => "燃料残量警告",
        "Steering Ratio" => "ステアリングレシオ",
        "Front Weight" => "フロント重量配分",
        "Corner Weight" => "コーナーウェイト",
        "Cross Weight" => "クロスウェイト",
        "In Car Dials" => "車内調整",
        "Display Page" => "表示ページ",
        "Tc Mode" | "TC Mode" => "TC モード",
        "Abs Setting" | "ABS Setting" => "ABS 設定",
        "Tc Setting" | "TC Setting" => "TC 設定",
        "Throttle Setting" => "スロットル設定",
        "Toe In" => "トーイン",
        "Toe Out" => "トーアウト",
        "LF Tire Pressure" | "LF Tyre Pressure" => "左フロント・タイヤ空気圧",
        "RF Tire Pressure" | "RF Tyre Pressure" => "右フロント・タイヤ空気圧",
        "LR Tire Pressure" | "LR Tyre Pressure" => "左リア・タイヤ空気圧",
        "RR Tire Pressure" | "RR Tyre Pressure" => "右リア・タイヤ空気圧",
        "Left Front" | "Front Left" => "左フロント",
        "Right Front" | "Front Right" => "右フロント",
        "Left Rear" | "Rear Left" => "左リア",
        "Right Rear" | "Rear Right" => "右リア",
        _ => return None,
    })
}

fn japanese_setup_word(word: &str) -> Option<&'static str> {
    Some(match word {
        "LF" => "左フロント",
        "RF" => "右フロント",
        "LR" => "左リア",
        "RR" => "右リア",
        "Left" => "左",
        "Right" => "右",
        "Front" => "フロント",
        "Rear" => "リア",
        "Tire" | "Tires" | "Tyre" | "Tyres" => "タイヤ",
        "Aero" | "Aerodynamics" => "エアロ",
        "Chassis" => "シャシー",
        "Vehicle" => "車両",
        "Specific" => "固有",
        "Section" => "設定",
        "General" => "一般",
        "Arbitrary" => "任意",
        "Setting" | "Settings" => "設定",
        "Cold" => "冷間",
        "Hot" => "温間",
        "Pressure" => "空気圧",
        "Tread" => "トレッド",
        "Last" => "直近",
        "Temps" | "Temperature" | "Temperatures" => "温度",
        "Starting" => "開始時",
        "Remaining" => "残量",
        "ARB" => "スタビライザー",
        "Antiroll" | "Anti-Roll" => "スタビライザー",
        "Bar" => "バー",
        "Damper" | "Dampers" => "ダンパー",
        "Bump" => "バンプ",
        "Rebound" => "リバウンド",
        "Compression" => "コンプレッション",
        "Slow" => "低速",
        "Fast" => "高速",
        "Stiffness" => "剛性",
        "Wing" => "ウイング",
        "Balance" => "バランス",
        "Camber" => "キャンバー",
        "Caster" => "キャスター",
        "Toe" => "トー",
        "In" => "イン",
        "Out" => "アウト",
        "Spring" | "Springs" => "スプリング",
        "Rate" => "レート",
        "Ride" => "車高",
        "Height" => "高さ",
        "Packers" | "Packer" => "パッカー",
        "Brake" | "Brakes" => "ブレーキ",
        "Pad" | "Pads" => "パッド",
        "Mu" => "摩擦係数",
        "Bias" => "バイアス",
        "Fuel" => "燃料",
        "Level" => "量",
        "Differential" | "Diff" => "デフ",
        "Preload" => "プリロード",
        "Power" => "パワー",
        "Coast" => "コースト",
        "Clutch" => "クラッチ",
        "Steering" => "ステアリング",
        "Ratio" => "レシオ",
        "Gear" | "Gearing" => "ギア",
        "Final" => "ファイナル",
        "Drive" => "ドライブ",
        "Weight" => "重量",
        "Cross" => "クロス",
        "Ballast" => "バラスト",
        "Cooling" => "冷却",
        "Duct" | "Ducts" => "ダクト",
        "Traction" => "トラクション",
        "Control" => "コントロール",
        "Mode" => "モード",
        "Page" => "ページ",
        "Display" => "表示",
        "Warning" => "警告",
        "Low" => "低",
        "Angle" => "角度",
        "Perch" => "シート",
        "ABS" => "ABS",
        "Map" => "マップ",
        "Engine" => "エンジン",
        "Throttle" => "スロットル",
        "Smoothing" => "スムージング",
        "Offset" => "オフセット",
        "Travel" => "ストローク",
        "Bumpstop" | "Bumpstops" => "バンプストップ",
        "Third" => "サード",
        "Heave" => "ヒーブ",
        _ => return None,
    })
}

pub fn setup_value(value: &str) -> String {
    setup_value_for(current_locale(), value)
}

pub fn setup_value_for(locale: Locale, value: &str) -> String {
    if locale == Locale::English {
        return value.to_owned();
    }

    let trimmed = value.trim();
    if let Some(page) = trimmed.strip_prefix("Race ") {
        return format!("レース {page}");
    }
    if trimmed.contains("(Delivery)") {
        return value.replace("(Delivery)", "(デリバリー)");
    }
    let translated = match trimmed.to_ascii_lowercase().as_str() {
        "dry" => "ドライ",
        "wet" => "ウェット",
        "soft" => "ソフト",
        "medium" => "ミディアム",
        "hard" => "ハード",
        "disk" => "ディスク",
        "garage" => "ガレージ",
        "delivery" => "デリバリー",
        "race" => "レース",
        "fixed" => "固定",
        "open" => "オープン",
        "enabled" | "on" => "オン",
        "disabled" | "off" => "オフ",
        _ => {
            if let Some(number) = trimmed.strip_suffix(" clicks") {
                return format!("{number} クリック");
            }
            if let Some(number) = trimmed.strip_suffix(" click") {
                return format!("{number} クリック");
            }
            if let Some(number) = trimmed.strip_suffix(" deg") {
                return format!("{number} 度");
            }
            return value.to_owned();
        },
    };

    translated.to_owned()
}

#[cfg(test)]
mod tests {
    use super::{Locale, Text, Translations, count_laps, setup_label_for, setup_value_for};

    #[test]
    fn japanese_catalog_translates_navigation() {
        let translations = Translations::new(Locale::Japanese);
        assert_eq!(translations.get(Text::Telemetry), "テレメトリー");
        assert_eq!(translations.get(Text::Settings), "設定");
    }

    #[test]
    fn japanese_plural_format_does_not_depend_on_english_grammar() {
        super::set_locale(Locale::Japanese);
        assert_eq!(count_laps(2), "2 ラップ");
        super::set_locale(Locale::English);
    }

    #[test]
    fn dynamic_car_setup_keys_and_values_are_localized() {
        assert_eq!(
            setup_label_for(Locale::Japanese, "Cold Pressure"),
            "冷間空気圧"
        );
        assert_eq!(
            setup_label_for(Locale::Japanese, "Front ARB"),
            "フロント・スタビライザー"
        );
        assert_eq!(
            setup_label_for(Locale::Japanese, "LF Tire Pressure"),
            "左フロント・タイヤ空気圧"
        );
        assert_eq!(setup_value_for(Locale::Japanese, "5 clicks"), "5 クリック");
        assert_eq!(setup_value_for(Locale::Japanese, "Dry"), "ドライ");
        assert_eq!(
            setup_label_for(Locale::Japanese, "Front Brake Pad Mu"),
            "フロントブレーキパッド摩擦係数"
        );
        assert_eq!(
            setup_label_for(Locale::Japanese, "Spring Perch Offset"),
            "スプリングシートオフセット"
        );
        assert_eq!(
            setup_label_for(Locale::Japanese, "In Car Dials"),
            "車内調整"
        );
        assert_eq!(
            setup_label_for(Locale::Japanese, "Brake Pressure Bias"),
            "ブレーキ圧バイアス"
        );
        assert_eq!(
            setup_value_for(Locale::Japanese, "0.58(Delivery)"),
            "0.58(デリバリー)"
        );
        assert_eq!(setup_value_for(Locale::Japanese, "Race 2"), "レース 2");
    }
}
