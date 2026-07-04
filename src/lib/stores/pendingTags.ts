import { writable } from 'svelte/store';

export interface PendingTags {
  subject: string;
  subject_topic: string;
  study_type: string;
  notes: string;
}

export const pendingTags = writable<PendingTags>({
  subject: '',
  subject_topic: '',
  study_type: '',
  notes: '',
});
