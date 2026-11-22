// Local-calendar date helpers. Dates travel as "YYYY-MM-DD" strings in the
// user's local time zone. Never use toISOString() for these: it converts to UTC
// and gives the wrong day east or west of Greenwich.

export function ymd(d: Date): string {
  const m = String(d.getMonth() + 1).padStart(2, "0");
  const day = String(d.getDate()).padStart(2, "0");
  return `${d.getFullYear()}-${m}-${day}`;
}

export function parseYmd(s: string): Date {
  const [y, m, d] = s.split("-").map(Number);
  return new Date(y, m - 1, d);
}

export function todayYmd(): string {
  return ymd(new Date());
}

export function addDays(s: string, n: number): string {
  const d = parseYmd(s);
  d.setDate(d.getDate() + n);
  return ymd(d);
}

export function daysBetween(from: string, to: string): number {
  return Math.round((parseYmd(to).getTime() - parseYmd(from).getTime()) / 86_400_000);
}

/** "Sunday, 22 November" (locale-aware). */
export function longDate(s: string): string {
  return parseYmd(s).toLocaleDateString(undefined, { weekday: "long", day: "numeric", month: "long" });
}

/** "Nov 18", or "Nov 18, 2025" when not in the current year. */
export function shortDate(s: string, today = todayYmd()): string {
  const d = parseYmd(s);
  const sameYear = d.getFullYear() === parseYmd(today).getFullYear();
  return d.toLocaleDateString(undefined, {
    month: "short",
    day: "numeric",
    ...(sameYear ? {} : { year: "numeric" }),
  });
}

/** "Today", "Yesterday", "Tomorrow", or a weekday + date. */
export function relativeDay(s: string, today = todayYmd()): string {
  const diff = daysBetween(today, s);
  if (diff === 0) return "Today";
  if (diff === -1) return "Yesterday";
  if (diff === 1) return "Tomorrow";
  return parseYmd(s).toLocaleDateString(undefined, { weekday: "long" });
}

export function plural(n: number, word: string): string {
  return `${n} ${word}${n === 1 ? "" : "s"}`;
}

export interface MonthCell {
  date: string;
  day: number;
  inMonth: boolean;
}

/** 6×7 grid of dates for a month view, weeks starting on Monday. */
export function monthGrid(year: number, month: number): MonthCell[] {
  const first = new Date(year, month, 1);
  const offset = (first.getDay() + 6) % 7; // Monday = 0
  const start = new Date(year, month, 1 - offset);
  const cells: MonthCell[] = [];
  for (let i = 0; i < 42; i++) {
    const d = new Date(start.getFullYear(), start.getMonth(), start.getDate() + i);
    cells.push({ date: ymd(d), day: d.getDate(), inMonth: d.getMonth() === month });
  }
  return cells;
}

export const WEEKDAYS_SHORT = (() => {
  // Monday-first, locale-aware single letters/short names.
  const base = new Date(2024, 0, 1); // a Monday
  return Array.from({ length: 7 }, (_, i) =>
    new Date(base.getFullYear(), 0, 1 + i).toLocaleDateString(undefined, { weekday: "narrow" }),
  );
})();

export function monthLabel(year: number, month: number): string {
  return new Date(year, month, 1).toLocaleDateString(undefined, { month: "long", year: "numeric" });
}

// ── durations and clock times ─────────────────────────────

/** "1h 20m", "45m", "<1m". */
export function formatDuration(ms: number): string {
  if (ms <= 0) return "0m";
  if (ms < 60_000) return "<1m";
  const totalMin = Math.floor(ms / 60_000);
  const h = Math.floor(totalMin / 60);
  const m = totalMin % 60;
  if (h === 0) return `${m}m`;
  return m === 0 ? `${h}h` : `${h}h ${m}m`;
}

/** Live stopwatch: "4:05", "1:02:03". */
export function formatClock(ms: number): string {
  const s = Math.max(0, Math.floor(ms / 1000));
  const h = Math.floor(s / 3600);
  const m = Math.floor((s % 3600) / 60);
  const sec = String(s % 60).padStart(2, "0");
  return h > 0 ? `${h}:${String(m).padStart(2, "0")}:${sec}` : `${m}:${sec}`;
}

/** "14:30" in local time for a UTC-ms instant. */
export function hhmm(ms: number): string {
  const d = new Date(ms);
  return `${String(d.getHours()).padStart(2, "0")}:${String(d.getMinutes()).padStart(2, "0")}`;
}

/** Local date ("YYYY-MM-DD") of a UTC-ms instant. */
export function ymdOf(ms: number): string {
  return ymd(new Date(ms));
}

/** Parses "9:05", "09:05", "0905" or "9.05" into minutes after midnight; null if invalid. */
export function parseTime(s: string): number | null {
  const t = s.trim().replace(".", ":");
  const m = /^(\d{1,2}):?(\d{2})$/.exec(t);
  if (!m) return null;
  const h = Number(m[1]);
  const min = Number(m[2]);
  if (h > 23 || min > 59) return null;
  return h * 60 + min;
}

/** UTC ms for a local date + minutes after midnight. */
export function localInstant(date: string, minutes: number): number {
  const d = parseYmd(date);
  return new Date(d.getFullYear(), d.getMonth(), d.getDate(), Math.floor(minutes / 60), minutes % 60).getTime();
}

/** Normalizes a reminder time ("9:05", "0905", "9.05", "3pm", "3:30 pm") to "HH:MM"; null if invalid. */
export function normalizeTime(s: string): string | null {
  const t = s.trim().toLowerCase();
  const ampm = /^(\d{1,2})(?::?(\d{2}))?\s*(am|pm)$/.exec(t);
  let minutes: number | null;
  if (ampm) {
    let h = Number(ampm[1]);
    const m = Number(ampm[2] ?? "0");
    if (h < 1 || h > 12 || m > 59) return null;
    if (ampm[3] === "pm" && h !== 12) h += 12;
    if (ampm[3] === "am" && h === 12) h = 0;
    minutes = h * 60 + m;
  } else {
    minutes = parseTime(t);
  }
  if (minutes === null) return null;
  return `${String(Math.floor(minutes / 60)).padStart(2, "0")}:${String(minutes % 60).padStart(2, "0")}`;
}

/** Formats "15:00" for display using the user's 12/24-hour preference. */
export function displayTime(hhmmStr: string): string {
  const [h, m] = hhmmStr.split(":").map(Number);
  return new Date(2000, 0, 1, h, m).toLocaleTimeString(undefined, { hour: "numeric", minute: "2-digit" });
}

/**
 * Parses a custom snooze: minutes ("45", "45m", "45 min"), hours ("1h", "1h30",
 * "1.5h", "2 hours") or a clock time ("16:30", "4:30pm" → today, or tomorrow if
 * already past). Returns the UTC-ms instant to snooze until, or null.
 */
export function parseSnooze(input: string, now = new Date()): number | null {
  const t = input.trim().toLowerCase();
  if (!t) return null;
  const mins = /^(\d+)\s*(m|min|mins|minute|minutes)?$/.exec(t);
  if (mins) {
    const n = Number(mins[1]);
    return n > 0 ? now.getTime() + n * 60_000 : null;
  }
  const hm = /^(\d+(?:\.\d+)?)\s*(h|hr|hrs|hour|hours)\s*(?:(\d+)\s*(m|min|mins)?)?$/.exec(t);
  if (hm) {
    const total = Number(hm[1]) * 60 + Number(hm[3] ?? 0);
    return total > 0 ? now.getTime() + Math.round(total) * 60_000 : null;
  }
  if (t.includes(":") || /(am|pm)$/.test(t)) {
    const clock = normalizeTime(t);
    if (!clock) return null;
    const [h, m] = clock.split(":").map(Number);
    const at = new Date(now.getFullYear(), now.getMonth(), now.getDate(), h, m);
    if (at.getTime() <= now.getTime()) at.setDate(at.getDate() + 1);
    return at.getTime();
  }
  return null;
}
