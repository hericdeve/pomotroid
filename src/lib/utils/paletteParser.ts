/**
 * Command palette parser.
 *
 * Supported syntax:
 *   focus [subject] [-t/--topic <topic>] [-s/--study-type <type>] [-n/--note <text...>]
 */

export type ActiveField = 'command' | 'subject' | 'topic' | 'studyType' | 'note';

export interface ParsedPalette {
  command: 'focus' | null;
  subject: string;
  topic: string;
  studyType: string;
  note: string;
  activeField: ActiveField;
  activeQuery: string;
  canSubmit: boolean;
}

const FLAG_MAP: Record<string, ActiveField> = {
  '-t': 'topic',
  '--topic': 'topic',
  '-s': 'studyType',
  '--study-type': 'studyType',
  '-n': 'note',
  '--note': 'note',
};
const FLAG_KEYS = Object.keys(FLAG_MAP);

function isFlag(t: string) { return t.startsWith('-'); }

export function parse(input: string): ParsedPalette {
  const blank: ParsedPalette = {
    command: null, subject: '', topic: '', studyType: '', note: '',
    activeField: 'command', activeQuery: '', canSubmit: false,
  };

  const trimmed = input.trimStart();
  if (!trimmed) return blank;

  const tokens = trimmed.split(/\s+/);
  const endsWithSpace = /\s$/.test(input);
  const cmd = tokens[0].toLowerCase();

  if (cmd !== 'focus') {
    return { ...blank, activeField: 'command', activeQuery: tokens[0] };
  }

  const result: ParsedPalette = { ...blank, command: 'focus' };
  const rest = tokens.slice(1);

  // Two-pass: first pass extracts committed values, second identifies active field
  let currentFlag: ActiveField | null = null;
  const noteTokens: string[] = [];
  let inNote = false;

  for (let i = 0; i < rest.length; i++) {
    const tok = rest[i];
    const isLast = i === rest.length - 1;
    const isPartial = isLast && !endsWithSpace;

    if (inNote) {
      if (!isPartial) noteTokens.push(tok);
      else { result.activeField = 'note'; result.activeQuery = tok; }
      continue;
    }

    if (FLAG_KEYS.includes(tok)) {
      currentFlag = FLAG_MAP[tok];
      if (currentFlag === 'note') inNote = true;
      if (isPartial) { result.activeField = 'subject'; result.activeQuery = tok; }
      continue;
    }

    if (isFlag(tok)) {
      if (isPartial) { result.activeField = 'subject'; result.activeQuery = tok; }
      continue;
    }

    if (currentFlag && currentFlag !== 'note') {
      if (!isPartial) {
        if (currentFlag === 'topic') result.topic = tok;
        if (currentFlag === 'studyType') result.studyType = tok;
        currentFlag = null;
      } else {
        result.activeField = currentFlag;
        result.activeQuery = tok;
      }
    } else if (!currentFlag) {
      if (!isPartial) result.subject = tok;
      else { result.activeField = 'subject'; result.activeQuery = tok; }
    }
  }

  if (noteTokens.length > 0 && result.activeField !== 'note') {
    result.note = noteTokens.join(' ');
  }

  // If input ends with space and no active field yet, infer next field
  if (endsWithSpace && result.activeField === 'command') {
    result.activeField = result.subject ? 'subject' : 'subject';
    result.activeQuery = '';
  }

  result.canSubmit = result.command === 'focus' && result.subject.trim().length > 0;
  return result;
}

export function completeSuggestion(input: string, parsed: ParsedPalette, value: string): string {
  if (parsed.activeField === 'command') return value + ' ';
  if (input.endsWith(' ') || input.endsWith('\t')) return input + value + ' ';
  const lastSpace = input.lastIndexOf(' ');
  const suffix = parsed.activeField === 'note' ? '' : ' ';
  return input.slice(0, lastSpace + 1) + value + suffix;
}

export function fieldLabel(field: ActiveField): string {
  switch (field) {
    case 'subject': return 'Subject';
    case 'topic': return 'Topic';
    case 'studyType': return 'Study type';
    case 'note': return 'Note';
    default: return 'Command';
  }
}
