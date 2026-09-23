// Helper for consistent subject colors and contrast text across views.

const PALETTE = [
  '#3b82f6', // blue
  '#10b981', // emerald
  '#8b5cf6', // violet
  '#f59e0b', // amber
  '#ec4899', // pink
  '#06b6d4', // cyan
  '#f97316', // orange
  '#14b8a6', // teal
  '#6366f1', // indigo
  '#ef4444', // red
  '#84cc16', // lime
  '#a855f7', // purple
];

export function getSubjectColor(subjectName: string): string {
  if (!subjectName || !subjectName.trim()) return '#6b7280';
  let hash = 0;
  const str = subjectName.trim().toLowerCase();
  for (let i = 0; i < str.length; i++) {
    hash = (hash << 5) - hash + str.charCodeAt(i);
    hash |= 0;
  }
  const index = Math.abs(hash) % PALETTE.length;
  return PALETTE[index];
}

export function getContrastColor(hexBg: string | undefined | null): string {
  if (!hexBg || !hexBg.startsWith('#')) return '#ffffff';
  let hex = hexBg.replace('#', '');
  if (hex.length === 3) {
    hex = hex.split('').map((c) => c + c).join('');
  }
  if (hex.length !== 6) return '#ffffff';
  const r = parseInt(hex.substring(0, 2), 16);
  const g = parseInt(hex.substring(2, 4), 16);
  const b = parseInt(hex.substring(4, 6), 16);
  const yiq = (r * 299 + g * 587 + b * 114) / 1000;
  return yiq >= 128 ? '#111827' : '#ffffff';
}
