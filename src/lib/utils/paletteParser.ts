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
  '-t': 'topic', '--topic': 'topic',
  '-s': 'studyType', '--study-type': 'studyType',
  '-n': 'note', '--note': 'note',
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

  const rawTokens = trimmed.split(/\s+/);
  const endsWithSpace = /\s$/.test(input);
  
  const cmd = rawTokens[0].toLowerCase();
  if (cmd !== 'focus') {
    return { ...blank, activeField: 'command', activeQuery: rawTokens[0] };
  }

  const result: ParsedPalette = { ...blank, command: 'focus' };
  const rest = rawTokens.slice(1);

  let currentFlag: ActiveField | null = null;
  
  for (let i = 0; i < rest.length; i++) {
    const tok = rest[i];
    const isLast = i === rest.length - 1;
    const isPartial = isLast && !endsWithSpace;

    if (FLAG_KEYS.includes(tok)) {
      currentFlag = FLAG_MAP[tok];
      if (isPartial) {
         result.activeField = 'subject'; 
         result.activeQuery = tok;
      }
      continue;
    }

    if (isFlag(tok)) {
      if (isPartial) {
        result.activeField = 'subject'; 
        result.activeQuery = tok;
      }
      continue;
    }

    if (isPartial) {
      result.activeField = currentFlag || 'subject';
      let existing = '';
      if (result.activeField === 'subject') existing = result.subject;
      else if (result.activeField === 'topic') existing = result.topic;
      else if (result.activeField === 'studyType') existing = result.studyType;
      else if (result.activeField === 'note') existing = result.note;
      
      result.activeQuery = existing ? existing + ' ' + tok : tok;
    }
    
    // Always append the token to the actual field value, so that if they hit Submit, it's not missing!
    if (!currentFlag) {
      result.subject = result.subject ? result.subject + ' ' + tok : tok;
    } else if (currentFlag === 'topic') {
      result.topic = result.topic ? result.topic + ' ' + tok : tok;
    } else if (currentFlag === 'studyType') {
      result.studyType = result.studyType ? result.studyType + ' ' + tok : tok;
    } else if (currentFlag === 'note') {
      result.note = result.note ? result.note + ' ' + tok : tok;
    }
  }

  if (endsWithSpace && rest.length >= 0) {
     result.activeField = currentFlag || 'subject';
     result.activeQuery = '';
  }

  result.canSubmit = result.command === 'focus' && result.subject.trim().length > 0;
  return result;
}

export function completeSuggestion(input: string, parsed: ParsedPalette, value: string): string {
  if (parsed.activeField === 'command') return value + ' ';
  
  let out = 'focus ';
  
  const cleanValue = value.replace(/^"|"$/g, '');
  
  const subj = parsed.activeField === 'subject' ? cleanValue : parsed.subject;
  const topic = parsed.activeField === 'topic' ? cleanValue : parsed.topic;
  const type = parsed.activeField === 'studyType' ? cleanValue : parsed.studyType;
  const note = parsed.activeField === 'note' ? cleanValue : parsed.note;
  
  if (subj) out += subj + ' ';
  if (topic) out += `-t ${topic} `;
  if (type) out += `-s ${type} `;
  if (note) out += `-n ${note}`;
  
  return out.trim() + (parsed.activeField === 'note' ? '' : ' ');
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
