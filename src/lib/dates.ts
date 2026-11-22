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
