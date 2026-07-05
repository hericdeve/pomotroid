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

/** The active goal for the current session (may differ from the settings default). */
export const sessionGoalRounds = writable<number>(8);

/** Controls visibility of the Session Goal dialog. */
export const showGoalModal = writable<boolean>(false);
