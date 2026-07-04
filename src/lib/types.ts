// Shared TypeScript types mirroring Rust structs (must stay in sync with Rust serde output).

export type RoundType = 'work' | 'short-break' | 'long-break';

/** Mirrors Rust `TimerSnapshot` — emitted via timer:tick / timer:round-change events
 *  and returned by the `timer_get_state` IPC command. */
export interface TimerState {
  round_type: RoundType;
  previous_round_type: string; // round type before this one; "" on first round
  elapsed_secs: number;
  total_secs: number;
  is_running: boolean;
  is_paused: boolean;
  work_round_number: number; // current work round (1-based)
  work_rounds_total: number; // total work rounds before long break
  session_work_count: number; // monotonic focus round count since last reset
  active_session_id: number | null;
}

export interface ImportSummary {
  imported: number;
  skipped: number;
}

/** Mirrors Rust `Settings` struct returned by `settings_get`. */
export interface Settings {
  time_work_secs: number;
  time_short_break_secs: number;
  time_long_break_secs: number;
  long_break_interval: number;
  short_breaks_enabled: boolean;
  long_breaks_enabled: boolean;
  auto_start_work: boolean;
  auto_start_break: boolean;
  tray_icon_enabled: boolean;
  min_to_tray: boolean;
  min_to_tray_on_close: boolean;
  notifications_enabled: boolean;
  always_on_top: boolean;
  break_always_on_top: boolean;
  volume: number; // 0.0–1.0
  tick_sounds_during_work: boolean;
  tick_sounds_during_break: boolean;
  shortcut_toggle: string;
  shortcut_reset: string;
  shortcut_skip: string;
  shortcut_restart: string;
  websocket_enabled: boolean;
  websocket_port: number;
  theme_mode: string; // 'auto' | 'light' | 'dark'
  theme_light: string;
  theme_dark: string;
  dial_countdown: boolean;
  language: string; // 'auto' | 'en' | 'es' | 'fr' | 'de' | 'ja'
  verbose_logging: boolean;
  check_for_updates: boolean;
  global_shortcuts_enabled: boolean;
  enable_window_controls: boolean;
  local_shortcut_toggle: string;
  local_shortcut_reset: string;
  local_shortcut_skip: string;
  local_shortcut_volume_down: string;
  local_shortcut_volume_up: string;
  local_shortcut_mute: string;
  local_shortcut_fullscreen: string;
  history_show_breaks: boolean;
}

/** Returned by `check_update` — describes an available update. */
export interface UpdateInfo {
  version: string;
  body: string | null;
  date: string | null;
}

/** Mirrors Rust `CustomAudioInfo` — null means the built-in sound is active. */
export interface CustomAudioInfo {
  work_alert: string | null;
  short_break_alert: string | null;
  long_break_alert: string | null;
}

/** Mirrors Rust `Theme` struct. Color keys include the `--` CSS var prefix. */
export interface Theme {
  name: string;
  colors: Record<string, string>; // keys like "--color-background", "--color-focus-round"
  is_custom: boolean;
}

// ---------------------------------------------------------------------------
// Stats types — mirror Rust structs in commands.rs / queries.rs
// ---------------------------------------------------------------------------

export interface DailyStats {
  rounds: number;
  focus_mins: number;
  completion_rate: number | null; // null when no sessions started today
  by_hour: number[]; // 24 entries, index = hour of day
}

export interface DayStat {
  date: string; // "YYYY-MM-DD"
  rounds: number;
}

export interface HeatmapEntry {
  date: string; // "YYYY-MM-DD"
  count: number;
}

export interface StreakInfo {
  current: number;
  longest: number;
}

/** Returned by stats_get_detailed — Today + This Week + streak in one call. */
export interface DetailedStats {
  today: DailyStats;
  week: DayStat[];
  streak: StreakInfo;
}

/** Returned by stats_get_heatmap — heatmap entries + lifetime totals. */
export interface HeatmapStats {
  entries: HeatmapEntry[];
  total_rounds: number;
  total_focus_secs: number;
  longest_streak: number;
}

export interface SubjectInsight {
  subject: string;
  focus_secs: number;
}

export interface InsightsStats {
  top_subjects: SubjectInsight[];
  by_day_of_week: number[];
  by_hour_of_day: number[];
}

// ---------------------------------------------------------------------------
// Extended Sessions types
// ---------------------------------------------------------------------------

export interface SessionRow {
  id: number;
  uuid: string;
  started_at: number;
  ended_at: number | null;
  round_type: string;
  duration_secs: number;
  completed: boolean;
  subject: string | null;
  subject_topic: string | null;
  study_type: string | null;
  notes: string | null;
  updated_at: number | null;
  deleted_at: number | null;
}

export interface UpdateSessionPayload {
  subject: string | null;
  subject_topic: string | null;
  study_type: string | null;
  notes: string | null;
}

export interface SessionFilter {
  subject?: string | null;
  subject_topic?: string | null;
  study_type?: string | null;
  date_from?: number | null;
  date_to?: number | null;
  show_breaks?: boolean;
}

export interface SessionHistoryPage {
  sessions: SessionRow[];
  total: number;
  total_work_rounds: number;
  total_focus_secs: number;
  longest_streak: number;
}

export interface CreateManualSessionPayload {
  started_at: number; // unix timestamp
  duration_secs: number;
  subject: string | null;
  subject_topic: string | null;
  study_type: string | null;
  notes: string | null;
}

