/**
 * sessionGoal.ts
 *
 * Stores the *active* session goal (number of work rounds to complete).
 * - Initialized from settings.session_goal_rounds on app load.
 * - Can be overridden at any time via the SessionGoalModal dialog.
 * - Resets back to the settings default when the timer is reset.
 *
 * The settings value is the *persistent default*. This store holds the
 * *live* value for the current session.
 */

import { writable } from 'svelte/store';
import type { Settings } from '$lib/types';

/** The active goal for the current session (may differ from the settings default). */
export const sessionGoalRounds = writable<number>(8);

/** Controls visibility of the Session Goal dialog. */
export const showGoalModal = writable<boolean>(false);

/**
 * Dynamically calculates the absolute maximum number of rounds that can fit
 * within a 24-hour period (86400 seconds) given the current break/work durations.
 */
export function getMaxSessionRounds(s: Settings): number {
  const MAX_DAY_SECS = 24 * 3600; // 86400
  let n = 1;
  while (true) {
    const studyTime = n * s.time_work_secs;
    
    const interval = s.long_break_interval;
    const totalBreaks = n - 1;
    let longBreaks = 0;
    let shortBreaks = 0;
    
    if (s.long_breaks_enabled) {
      longBreaks = Math.floor(totalBreaks / interval);
      shortBreaks = s.short_breaks_enabled ? totalBreaks - longBreaks : 0;
    } else if (s.short_breaks_enabled) {
      shortBreaks = totalBreaks;
    }
    
    const breakTime = longBreaks * s.time_long_break_secs + shortBreaks * s.time_short_break_secs;
    
    if (studyTime + breakTime > MAX_DAY_SECS) {
      return Math.max(1, n - 1);
    }
    n++;
    if (n > 999) return 999; // Hard cap fallback
  }
}
